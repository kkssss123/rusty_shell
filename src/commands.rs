pub mod rn;

use clap::Parser;
use std::io::{self, Error, ErrorKind};
use std::path::PathBuf;

pub(crate) enum BuiltinCommand {
    Cd(PathBuf),
    Exit,
    Rn(rn::RnArgs),
}

pub(crate) fn parse_builtin(line: &str) -> io::Result<Option<BuiltinCommand>> {
    let Some(name) = line.split_whitespace().next() else {
        return Ok(None);
    };

    if !matches!(name, "cd" | "exit" | "rn") {
        return Ok(None);
    }

    let args = shlex::split(line)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "error: invalid command syntax"))?;

    match name {
        "cd" => {
            if args.len() > 2 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "cd: too many arguments",
                ));
            }
            let path = match args.get(1).map(String::as_str) {
                None | Some("~") => std::env::home_dir().ok_or_else(|| {
                    Error::new(
                        ErrorKind::NotFound,
                        "cd: could not determine home directory",
                    )
                })?,
                Some(path) if path.starts_with("~/") || path.starts_with("~\\") => {
                    let home = std::env::home_dir().ok_or_else(|| {
                        Error::new(
                            ErrorKind::NotFound,
                            "cd: could not determine home directory",
                        )
                    })?;
                    home.join(&path[2..])
                }
                Some(path) => PathBuf::from(path),
            };
            Ok(Some(BuiltinCommand::Cd(path)))
        }
        "exit" => Ok(Some(BuiltinCommand::Exit)),
        "rn" => {
            let args = rn::RnArgs::try_parse_from(args)
                .map_err(|e| Error::new(ErrorKind::InvalidInput, e.to_string()))?;
            Ok(Some(BuiltinCommand::Rn(args)))
        }
        _ => unreachable!("only registered built-in commands reach this branch"),
    }
}
