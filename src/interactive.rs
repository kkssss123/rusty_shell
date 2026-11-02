use alphanumeric_sort;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use shlex;
use std::fs;
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
            let count = match args.get(3) {
                Some(c) => c.parse().map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!("count should be 0 or more."),
                    )
                })?,
                None => 1,
            };

            let from = args.get(1).map_or(Path::new("."), |p| Path::new(p));
            let to = args.get(2).map_or(from, |p| Path::new(p));

            rename_files(from, to, count)
        }
        _ => {
            println!("command not found: {}", line);
            Ok(())
        }
    }
}

fn rename_files(from: &Path, to: &Path, count: u32) -> io::Result<()> {
    let from = dunce::canonicalize(from)?;

    fs::create_dir_all(to)?;
    let to = dunce::canonicalize(to)?;
    println!("{:?} {:?}", from, to);

    let is_subdir = to != from && to.starts_with(&from);

    rename_files_recursive(&from, &to, &to, is_subdir, count)
}

fn rename_files_recursive(
    from: &Path,
    to: &Path,
    to_root: &Path,
    is_subdir: bool,
    count: u32
) -> io::Result<()> {
    let mut file_names: Vec<String> = vec![];

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if is_subdir && dunce::canonicalize(&path)? == to_root {
                continue;
            }
            let relative = path.strip_prefix(from).expect("path should be inside from");
            rename_files_recursive(&path, &to.join(relative), to_root, is_subdir, count)?;
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();
        if is_target(&file_name) {
            file_names.push(file_name);
        }
    }

    file_names.sort_by(|a, b| {
        let a = a.to_lowercase();
        let a = filename(&a);
        let b = b.to_lowercase();
        let b = filename(&b);

        alphanumeric_sort::compare_str(a, b)
    });

    fs::create_dir_all(to)?;

    let mut count = count;
    let width = (count + file_names.len() as u32)
        .saturating_sub(1)
        .to_string()
        .len()
        .max(3);
    for f in file_names {
        let ext = extension(&f);
        let new_name = format!("{:0width$}.{ext}", count, width = width);
        println!("file renamed to: {} -> {}", f, new_name);
        fs::rename(from.join(f), to.join(new_name))?;
        count += 1;
    }
    Ok(())
}

static TARGET: [&str; 3] = ["jpg", "jpeg", "webp"];

fn is_target(file_name: &str) -> bool {
    TARGET
        .iter()
        .any(|ext| extension(file_name).eq_ignore_ascii_case(ext))
}

fn filename(file_name: &str) -> &str {
    file_name
        .rsplit_once(".")
        .map_or(file_name, |(name, _)| name)
}

fn extension(file_name: &str) -> &str {
    file_name.rsplit(".").next().unwrap_or("no extension")
}
