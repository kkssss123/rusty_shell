use crate::{Shell, ShellOutcome};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

pub fn run(shell_program: Option<String>) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut shell = Shell::new(shell_program)?;
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                match shell.execute(&line) {
                    Ok(ShellOutcome::Continue) => {}
                    Ok(ShellOutcome::Exit) => break,
                    Err(e) => eprintln!("{e}"),
                }
                rl.add_history_entry(line.as_str())?;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(ReadlineError::Interrupted) => {
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    rl.save_history("history.txt")?;
    Ok(())
}
