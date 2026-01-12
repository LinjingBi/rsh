use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::input::{is_preamble_line, count_opening_braces, count_closing_braces};
use super::utils::{run_cargo_rsh, looks_like_async_error, detect_async_runtime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncRuntime {
    Tokio,
    AsyncStd,
    Smol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Sync,
    Async(AsyncRuntime),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Preamble,
    Body,
}

pub struct Session {
    preamble: Vec<String>,
    body: Vec<String>,
    mode: Mode,
    prev_preamble_len: usize,
    prev_body_len: usize,
    base_dir: PathBuf,
    runtime_dir: PathBuf,
    rsh_path: PathBuf,
    cargo_path: PathBuf,
}

impl Session {
    /// Create a new Session with an optional base directory.
    /// If `base_dir` is `None`, uses the current directory.
    pub fn new<P: AsRef<Path>>(base_dir: Option<P>) -> Self {
        let base = if let Some(dir) = base_dir {
            dir.as_ref().canonicalize()
                .unwrap_or_else(|_| dir.as_ref().to_path_buf())
        } else {
            std::env::current_dir()
                .unwrap()
                .canonicalize()
                .unwrap_or_else(|_| std::env::current_dir().unwrap())
        };
        
        let cargo_path = base.join("Cargo.toml");
        let runtime_dir = base.join("src").join("bin");
        let rsh_path = runtime_dir.join("__rsh.rs");
        
        Session {
            preamble: Vec::new(),
            body: Vec::new(),
            mode: Mode::Sync,
            prev_preamble_len: 0,
            prev_body_len: 0,
            base_dir: base,
            runtime_dir,
            rsh_path,
            cargo_path,
        }
    }

    pub fn reset(&mut self) {
        self.preamble.clear();
        self.body.clear();
        self.mode = Mode::Sync;
        self.prev_preamble_len = 0;
        self.prev_body_len = 0;
    }



    pub fn add_code_block(&mut self, block: &str) {
        // snapshot previous successful state
        self.prev_preamble_len = self.preamble.len();
        self.prev_body_len = self.body.len();

        let mut in_preamble_construct = false;
        let mut brace_depth = 0;

        for line in block.lines() {
            let trimmed_start = line.trim_start();
            if trimmed_start.is_empty() {
                // Preserve empty lines when in preamble construct
                if in_preamble_construct {
                    self.preamble.push(line.to_string());
                }
                // Skip empty lines when not in preamble construct
                continue;
            }

            // Check if this line starts a preamble construct
            let starts_preamble = is_preamble_line(trimmed_start);
            
            // Update brace depth based on the line content
            brace_depth += count_opening_braces(trimmed_start);
            brace_depth -= count_closing_braces(trimmed_start);
            
            // If we start a preamble construct, enter preamble mode
            if starts_preamble {
                in_preamble_construct = true;
            }
            
            // If we're in a preamble construct, add to preamble
            if in_preamble_construct {
                self.preamble.push(line.to_string());
                // Exit preamble construct when all braces are closed
                if brace_depth == 0 {
                    in_preamble_construct = false;
                }
            } else {
                self.body.push(line.to_string());
            }
        }
    }

    pub fn show(&self) {
        println!("--- PREAMBLE ---");
        if self.preamble.is_empty() {
            println!("<empty>");
        } else {
            for (index, line) in self.preamble.iter().enumerate() {
                println!("[{}] {}", index, line);
            }
        }
        println!("--- BODY ---");
        if self.body.is_empty() {
            println!("<empty>");
        } else {
            for (index, line) in self.body.iter().enumerate() {
                println!("[{}] {}", index, line);
            }
        }
        println!("--- MODE ---");
        println!("{:?}", self.mode);
    }

    // Public getters for testing (integration tests need these)
    pub fn preamble(&self) -> &[String] {
        &self.preamble
    }

    pub fn body(&self) -> &[String] {
        &self.body
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn delete(&mut self, segment: Segment, indices: &[usize]) {
        let target_vec = match segment {
            Segment::Preamble => &mut self.preamble,
            Segment::Body => &mut self.body,
        };

        let mut sorted = indices.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        

        // Atomic validation: if any index is out of bounds, abort the whole delete.
        if let Some(&max_idx) = sorted.last() {
            if max_idx >= target_vec.len() {
                eprintln!(
                    "rsh: indices {:?} out of bounds for {:?} (len = {})",
                    sorted,
                    segment,
                    target_vec.len()
                );
                return;
            }
        } else {
            // No indices after dedup – nothing to do.
            return;
        }

        // All indices valid; perform deletions from largest to smallest.
        for &idx in sorted.iter().rev() {
            target_vec.remove(idx);
        }

        self.show();
    }


    pub fn write_rsh_bin(&self) -> Result<(), Box<dyn Error>> {
        let path = &self.rsh_path;
        let mut code = String::new();

        // Preamble at module scope.
        for line in &self.preamble {
            code.push_str(line);
            code.push('\n');
        }
        if !self.preamble.is_empty() {
            code.push('\n');
        }

        match self.mode {
            Mode::Sync => {
                code.push_str(
                    "fn __rsh_session() -> Result<(), Box<dyn std::error::Error>> {\n",
                );
                for line in &self.body {
                    code.push_str("    ");
                    code.push_str(line);
                    code.push('\n');
                }
                code.push_str("    Ok(())\n");
                code.push_str("}\n\n");
                code.push_str("fn main() {\n");
                code.push_str("    if let Err(e) = __rsh_session() {\n");
                code.push_str("        eprintln!(\"{}\", e);\n");
                code.push_str("    }\n");
                code.push_str("}\n");
            }
            Mode::Async(runtime) => {
                code.push_str(
                    "async fn __rsh_session() -> Result<(), Box<dyn std::error::Error>> {\n",
                );
                for line in &self.body {
                    code.push_str("    ");
                    code.push_str(line);
                    code.push('\n');
                }
                code.push_str("    Ok(())\n");
                code.push_str("}\n\n");

                match runtime {
                    AsyncRuntime::Tokio => {
                        code.push_str("#[tokio::main]\n");
                    }
                    AsyncRuntime::AsyncStd => {
                        code.push_str("#[async_std::main]\n");
                    }
                    AsyncRuntime::Smol => {
                        // smol does not provide a proc-macro main by default; use a manual executor.
                        code.push_str("fn main() {\n");
                        code.push_str("    smol::block_on(async {\n");
                        code.push_str("        if let Err(e) = __rsh_session().await {\n");
                        code.push_str("            eprintln!(\"{}\", e);\n");
                        code.push_str("        }\n");
                        code.push_str("    });\n");
                        code.push_str("}\n");
                        fs::write(path, code)?;
                        return Ok(());
                    }
                }

                code.push_str("async fn main() {\n");
                code.push_str("    if let Err(e) = __rsh_session().await {\n");
                code.push_str("        eprintln!(\"{}\", e);\n");
                code.push_str("    }\n");
                code.push_str("}\n");
            }
        }

        fs::write(path, code)?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // Ensure bin directory exists.
        if !self.runtime_dir.exists() {
            fs::create_dir_all(&self.runtime_dir)?;
        }

        // First attempt in current mode.
        self.write_rsh_bin()?;
        let output = run_cargo_rsh(&self.base_dir)?;

        // io::stdout().write_all(&output.stdout)?;
        // io::stderr().write_all(&output.stderr)?;

        if output.status.success() {
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
            return Ok(());
        }

        // See if error looks async-related.
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        // not aysnc error, remove the code from session
        if !looks_like_async_error(&stderr_str) {
            // user code failed: roll back buffers only
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
            self.preamble.truncate(self.prev_preamble_len);
            self.body.truncate(self.prev_body_len);
            return Ok(());
        }
        // yes async-liked error
        // If already async, just keep going – do not try to switch again.
        if matches!(self.mode, Mode::Async(_)) {
            return Ok(());
        }

        // Try to detect a runtime from Cargo.toml.
        let runtime = detect_async_runtime(&self.cargo_path);
        let Some(runtime) = runtime else {
            eprintln!("rsh: Async usage detected (`await` or async error), but no supported async runtime was found in Cargo.toml.");
            eprintln!("rsh: Please add one of: tokio, async-std, or smol to your Cargo.toml and try again.");
            self.preamble.truncate(self.prev_preamble_len);
            self.body.truncate(self.prev_body_len);
            return Ok(());
        };

        self.mode = Mode::Async(runtime);
        eprintln!("rsh: Detected async usage; switching to async mode with runtime: {:?}.", runtime);

        // Regenerate in async mode and rerun once.
        self.write_rsh_bin()?;
        let output2 = run_cargo_rsh(&self.base_dir)?;
        io::stdout().write_all(&output2.stdout)?;
        io::stderr().write_all(&output2.stderr)?;

        Ok(())
    }

    pub fn cleanup(&self) {
        if self.rsh_path.exists() {
            if let Err(e) = fs::remove_file(&self.rsh_path) {
                eprintln!("rsh: failed to remove generated __rsh.rs: {e}");
            }
        }
    }
}

