use clap::Parser;
use std::path::Path;
use std::{fs, io};

#[derive(Parser, Debug)]
pub(crate) struct RnArgs {
    pub(crate) from: Option<String>,

    #[arg(short, long)]
    pub(crate) to: Option<String>,

    #[arg(short, long, value_parser = parse_first)]
    pub(crate) first: Option<u32>,

    #[arg(short, long)]
    pub(crate) verbose: bool,
}

fn parse_first(value: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| "first should be 0 or more integer.".to_string())
}

pub(crate) fn execute(from: &Path, to: &Path, first: u32, verbose: bool) -> io::Result<()> {
    let from = dunce::canonicalize(from)?;

    fs::create_dir_all(to)?;
    let to = dunce::canonicalize(to)?;

    let is_subdir = to != from && to.starts_with(&from);

    rename_files(&from, &to, &to, is_subdir, first, verbose)
}

fn rename_files(
    from: &Path,
    to: &Path,
    to_root: &Path,
    is_subdir: bool,
    first: u32,
    verbose: bool,
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
            rename_files(
                &path,
                &to.join(relative),
                to_root,
                is_subdir,
                first,
                verbose,
            )?;
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

    let mut count = first;
    let width = (count + file_names.len() as u32)
        .saturating_sub(1)
        .to_string()
        .len()
        .max(3);
    for f in file_names {
        let ext = extension(&f);
        let new_name = format!("{:0width$}.{ext}", count, width = width);
        if verbose {
            println!("file renamed to: {} -> {}", f, new_name);
        }
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
