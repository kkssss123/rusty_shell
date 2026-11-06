use crate::{Shell, ShellOutcome};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use std::path::PathBuf;

pub fn run(shell_program: Option<String>) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut shell = Shell::new(shell_program)?;
    let history_path = history_file_path();
    if rl.load_history(&history_path).is_err() {
        println!("No previous history.");
    }

    loop {
        let readline = rl.readline(format!("{}> ", shell.cwd().display()).as_str());
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
    if let Err(e) = rl.save_history(&history_path) {
        eprintln!("Failed to save history: {e}");
    }
    Ok(())
}

fn history_file_path() -> PathBuf {
    if let Some(home) = std::env::home_dir() {
        home.join(".rusty_shell_history")
    } else {
        PathBuf::from(".rusty_shell_history")
    }
}
