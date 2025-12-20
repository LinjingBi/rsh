use std::error::Error;

use rustyline::Editor;
use rustyline::history::DefaultHistory;

use rsh::rsh::{Session, read_block, Input};

fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = Editor::<(), DefaultHistory>::new()?;
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
                if let Err(e) = session.run() {
                    eprintln!("Internal rsh error: {e}");
                    break;
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
    // cleanup
    session.cleanup();

    Ok(())
}
