use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::input::is_preamble_line;
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

pub struct Session {
    preamble: Vec<String>,
    body: Vec<String>,
    mode: Mode,
    prev_preamble_len: usize,
    prev_body_len: usize,
    runtime_dir: PathBuf,
    rsh_path: PathBuf,
    cargo_path: PathBuf,
}

impl Session {
    pub fn new() -> Self {
        let cargo_path = Path::new("Cargo.toml").to_path_buf();
        let runtime_dir = Path::new("src").join("bin");
        let rsh_path = runtime_dir.join("__rsh.rs");
        Session {
            preamble: Vec::new(),
            body: Vec::new(),
            mode: Mode::Sync,
            prev_preamble_len: 0,
            prev_body_len: 0,
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

        for line in block.lines() {
            let trimmed_start = line.trim_start();
            if trimmed_start.is_empty() {
                continue;
            }

            if is_preamble_line(trimmed_start) {
                self.preamble.push(line.to_string());
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
            for line in &self.preamble {
                println!("{line}");
            }
        }
        println!("--- BODY ---");
        if self.body.is_empty() {
            println!("<empty>");
        } else {
            for line in &self.body {
                println!("{line}");
            }
        }
        println!("--- MODE ---");
        println!("{:?}", self.mode);
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
        let output = run_cargo_rsh()?;

        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;

        if output.status.success() {
            return Ok(());
        }

        // See if error looks async-related.
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        // not aysnc error, remove the code from session
        if !looks_like_async_error(&stderr_str) {
            // user code failed: roll back buffers only
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
        let output2 = run_cargo_rsh()?;
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

