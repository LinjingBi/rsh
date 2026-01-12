use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::history::DefaultHistory;

use super::session::{Segment, Session};

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

/// Handle the `:delete` meta-command.
///
/// Expected syntax:
/// `:delete <preamble|body> <index...>`
pub fn handle_delete_command(cmd: &str, session: &mut Session) {
    let mut parts = cmd.split_whitespace();
    let head = parts.next().unwrap_or("");

    if head != ":delete" {
        eprintln!("rsh: internal error: handle_delete_command called with non-:delete command");
        return;
    }

    let target_str = match parts.next() {
        Some(s) => s,
        None => {
            eprintln!("Usage: :delete <preamble|body> <index...>");
            return;
        }
    };

    let segment = match target_str {
        "preamble" => Segment::Preamble,
        "body" => Segment::Body,
        other => {
            eprintln!("Invalid segment '{}'; expected 'preamble' or 'body'.", other);
            return;
        }
    };

    let mut indices: Vec<usize> = Vec::new();

    for p in parts {
        match p.parse::<usize>() {
            Ok(i) => indices.push(i),
            Err(_) => {
                eprintln!("Invalid index: '{}'", p);
                return;
            }
        }
    }

    if indices.is_empty() {
        eprintln!("Usage: :delete <preamble|body> <index...>");
        return;
    }

    session.delete(segment, &indices);
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

/// Count opening braces ({, [, () in a line
pub fn count_opening_braces(line: &str) -> i32 {
    line.chars().filter(|&c| c == '{' || c == '[' || c == '(').count() as i32
}

/// Count closing braces (}, ], )) in a line
pub fn count_closing_braces(line: &str) -> i32 {
    line.chars().filter(|&c| c == '}' || c == ']' || c == ')').count() as i32
}

