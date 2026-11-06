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
            let Some(path) = args.get(1) else {
                return Err(Error::new(ErrorKind::InvalidInput, "cd requires a path"));
            };
            if args.len() != 2 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "cd accepts exactly one path",
                ));
            }
            Ok(Some(BuiltinCommand::Cd(PathBuf::from(path))))
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
