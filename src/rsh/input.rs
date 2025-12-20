use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::history::DefaultHistory;

pub enum Input {
    Command(String),
    Code(String),
}

pub fn read_block(rl: &mut Editor<(), DefaultHistory>) -> Result<Option<Input>, ReadlineError> {
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

        // inside read_block, where you currently call add_history_entry
        if let Err(e) = rl.add_history_entry(line.as_str()) {
        // internal failure: bubble up so main can print and exit
            return Err(e);
        }
        block.push(line);
        prompt = "...> ";
    }
}

pub fn is_preamble_line(line: &str) -> bool {
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

