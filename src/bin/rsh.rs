use std::error::Error;
use std::path::PathBuf;

use rustyline::Editor;
use rustyline::history::DefaultHistory;

use rsh::rsh::{Session, read_block, Input, handle_delete_command};

fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = Editor::<(), DefaultHistory>::new()?;
    let mut session = Session::new(None::<PathBuf>);

    loop {
        match read_block(&mut rl) {
            Ok(Some(Input::Command(cmd))) => {
                if cmd.starts_with(":delete ") {
                    handle_delete_command(&cmd, &mut session);
                } else {
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
