use crate::commands::rn;
use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use std::io::{self, Error, ErrorKind};
use std::path::Path;

pub fn run() -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) if line.trim() == "exit" => {
                break;
            }
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                rl.add_history_entry(line.as_str())?;
                if let Err(e) = execute(&line) {
                    eprintln!("{e}");
                }
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

fn execute(line: &str) -> io::Result<()> {
    let args = shlex::split(line)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "invalid command syntax"))?;

    let Some(cmd) = args.first() else {
        return Ok(());
    };

    match cmd.as_str() {
        "rn" => {
            let args = rn::RnArgs::try_parse_from(args)
                .map_err(|e| Error::new(ErrorKind::InvalidInput, e.to_string()))?;

            let from = Path::new(&args.from);
            let to = Path::new(args.to.as_deref().unwrap_or(&args.from));
            rn::execute(from, to, args.first, args.verbose)
        }
        _ => {
            println!("command not found: {}", line);
            Ok(())
        }
    }
}
