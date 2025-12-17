use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use rustyline::error::ReadlineError;
use rustyline::Editor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AsyncRuntime {
    Tokio,
    AsyncStd,
    Smol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Sync,
    Async(AsyncRuntime),
}

struct Session {
    preamble: Vec<String>,
    body: Vec<String>,
    mode: Mode,
}

impl Session {
    fn new() -> Self {
        Session {
            preamble: Vec::new(),
            body: Vec::new(),
            mode: Mode::Sync,
        }
    }

    fn reset(&mut self) {
        self.preamble.clear();
        self.body.clear();
        self.mode = Mode::Sync;
    }

    fn add_code_block(&mut self, block: &str) {
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

    fn show(&self) {
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
}

fn is_preamble_line(line: &str) -> bool {
    let prefixes = [
        "use ",
        "mod ",
        "extern crate",
        "#![",
        "#[",
        "struct ",
        "enum ",
        "type ",
        "trait ",
        "fn ",
        "const ",
        "static ",
    ];

    if prefixes.iter().any(|p| line.starts_with(p)) {
        return true;
    }

    // Handle common impl prefixes.
    if line.starts_with("impl ")
        || line.starts_with("impl<")
        || line.starts_with("impl<T")
        || line.starts_with("impl<U")
    {
        return true;
    }

    false
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = Editor::<()>::new()?;
    let mut session = Session::new();

    loop {
        match read_block(&mut rl) {
            Ok(Some(Input::Command(cmd))) => {
                match cmd.as_str() {
                    ":q" | ":quit" => break,
                    ":reset" => {
                        session.reset();
                        println!("Session reset.");
                    }
                    ":show" => {
                        session.show();
                    }
                    _ => {
                        eprintln!("Unknown command: {cmd}");
                    }
                }
            }
            Ok(Some(Input::Code(block))) => {
                session.add_code_block(&block);
                if let Err(e) = run_session(&session) {
                    eprintln!("Internal rsh error: {e}");
                }
            }
            Ok(None) => {
                // EOF (Ctrl-D) – exit session.
                break;
            }
            Err(e) => {
                eprintln!("Readline error: {e}");
                break;
            }
        }
    }

    Ok(())
}

enum Input {
    Command(String),
    Code(String),
}

fn read_block(rl: &mut Editor<()>) -> Result<Option<Input>, ReadlineError> {
    let mut block: Vec<String> = Vec::new();
    let mut prompt = "rsh> ";

    loop {
        let line = rl.readline(prompt)?;
        let trimmed = line.trim();

        // Meta-commands: only recognized when starting a new block.
        if block.is_empty() && trimmed.starts_with(':') {
            let cmd = trimmed.to_string();
            return Ok(Some(Input::Command(cmd)));
        }

        // Empty line:
        if trimmed.is_empty() {
            if block.is_empty() {
                // Ignore stray empty lines.
                prompt = "rsh> ";
                continue;
            } else {
                // End of multi-line block.
                let code = block.join("\n");
                return Ok(Some(Input::Code(code)));
            }
        }

        rl.add_history_entry(line.as_str());
        block.push(line);
        prompt = "...> ";
    }
}

fn run_session(session: &mut Session) -> Result<(), Box<dyn Error>> {
    // Ensure bin directory exists.
    let bin_dir = Path::new("src").join("bin");
    if !bin_dir.exists() {
        fs::create_dir_all(&bin_dir)?;
    }

    // First attempt in current mode.
    write_rsh_bin(session)?;
    let output = run_cargo_rsh()?;

    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;

    if output.status.success() {
        return Ok(());
    }

    // If already async, just keep going – do not try to switch again.
    if matches!(session.mode, Mode::Async(_)) {
        return Ok(());
    }

    // See if error looks async-related.
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    if !looks_like_async_error(&stderr_str) {
        return Ok(());
    }

    // Try to detect a runtime from Cargo.toml.
    let runtime = detect_async_runtime();
    let Some(runtime) = runtime else {
        eprintln!("rsh: Async usage detected (`await` or async error), but no supported async runtime was found in Cargo.toml.");
        eprintln!("rsh: Please add one of: tokio, async-std, or smol to your Cargo.toml and try again.");
        return Ok(());
    };

    session.mode = Mode::Async(runtime);
    eprintln!("rsh: Detected async usage; switching to async mode with runtime: {:?}.", runtime);

    // Regenerate in async mode and rerun once.
    write_rsh_bin(session)?;
    let output2 = run_cargo_rsh()?;
    io::stdout().write_all(&output2.stdout)?;
    io::stderr().write_all(&output2.stderr)?;

    Ok(())
}

fn write_rsh_bin(session: &Session) -> Result<(), Box<dyn Error>> {
    let path = Path::new("src").join("bin").join("__rsh.rs");
    let mut code = String::new();

    // Preamble at module scope.
    for line in &session.preamble {
        code.push_str(line);
        code.push('\n');
    }
    if !session.preamble.is_empty() {
        code.push('\n');
    }

    match session.mode {
        Mode::Sync => {
            code.push_str(
                "fn __rsh_session() -> Result<(), Box<dyn std::error::Error>> {\n",
            );
            for line in &session.body {
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
            for line in &session.body {
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

fn run_cargo_rsh() -> Result<std::process::Output, Box<dyn Error>> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("__rsh")
        .output()?;
    Ok(output)
}

fn looks_like_async_error(stderr: &str) -> bool {
    let patterns = [
        "E0728",
        "E0752",
        "only allowed inside `async` functions",
        "only allowed inside async functions",
        "cannot be used in a `fn` item that is not `async`",
        "future cannot be sent between threads safely",
        "cannot be sent between threads safely",
        "async fn main",
    ];

    patterns.iter().any(|p| stderr.contains(p))
}

fn detect_async_runtime() -> Option<AsyncRuntime> {
    let Ok(toml) = fs::read_to_string("Cargo.toml") else {
        return None;
    };
    let lower = toml.to_lowercase();

    if lower.contains("tokio") {
        return Some(AsyncRuntime::Tokio);
    }
    if lower.contains("async-std") {
        return Some(AsyncRuntime::AsyncStd);
    }
    if lower.contains("smol") {
        return Some(AsyncRuntime::Smol);
    }

    None
}


