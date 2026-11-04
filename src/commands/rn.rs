use clap::Parser;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::{fs, result};

#[derive(Parser, Debug)]
#[command(
    name = "rn",
    about = "Recursively renames supported image files using sequential numbers.",
    long_about = "\
Recursively searches FROM for supported image files (.jpg, .jpeg, .webp),
sorts them by filename, and renames them using sequential numbers.

The original directory structure is preserved. Numbering starts at FIRST
and restarts in each directory. Filenames are zero-padded to at least
three digits, with the width expanding when necessary.

If --to is omitted, files are renamed in place. If --to is provided,
renamed files are written under TO. Existing files in a different
destination directory may be overwritten.

PREFIX and SUFFIX are added before and after the sequence number,
respectively. Both are optional and default to an empty string.",
    override_usage = "\
rn [FROM] [OPTIONS]",
    after_help = "\
Examples:
  rn
      Rename image files in the current directory in place.

  rn ./photos --to ./numbered
      Rename image files from ./photos into ./numbered.

  rn ./photos --prefix IMG_ --suffix _edited
      Rename files with a prefix and suffix."
)]
pub(crate) struct RnArgs {
    #[arg(
        help = "Source directory to process (defaults to current directory)",
        default_value = ".",
        hide_default_value = true
    )]
    from: PathBuf,

    #[arg(short, long, help = "Destination directory (defaults to FROM)")]
    to: Option<PathBuf>,

    #[arg(short, long, value_parser = RnArgs::parse_first, default_value = "1", hide_default_value = true, help = "Starting number for each directory (defaults to 1)")]
    first: u32,

    #[arg(short, long, help = "Print each source and destination path")]
    verbose: bool,

    #[arg(short, long, value_parser = RnArgs::parse_affix, default_value = "", hide_default_value = true, help = "Prefix for renamed files")]
    prefix: String,

    #[arg(short, long, value_parser = RnArgs::parse_affix, default_value = "", hide_default_value = true, help = "Suffix for renamed files")]
    suffix: String,
}

impl RnArgs {
    fn parse_first(value: &str) -> result::Result<u32, String> {
        value
            .parse()
            .map_err(|_| "first should be 0 or more integer.".to_string())
    }

    fn parse_affix(value: &str) -> result::Result<String, String> {
        if value.contains("/") || value.contains("\\") {
            return Err("prefix/suffix should not contain \\ or /.".to_string());
        }
        Ok(value.to_string())
    }
}

struct RenamePaths {
    from: PathBuf,
    to: PathBuf,
    backup: PathBuf,
}

impl RenamePaths {
    fn setup(from: &Path, to: &Path) -> Result<RenamePaths> {
        let from = dunce::canonicalize(&from)?;

        fs::create_dir_all(to)?;
        let to = dunce::canonicalize(&to)?;

        let backup = from.join("backup");
        if to.starts_with(&backup) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("destination must not be inside {}", backup.display()),
            ));
        }

        fs::create_dir(&backup)?;
        let backup = dunce::canonicalize(&backup)?;

        Ok(RenamePaths { from, to, backup })
    }

    fn join(&self, path: &Path) -> RenamePaths {
        let from = self.from.join(path);
        let to = self.to.join(path);
        let backup = self.backup.join(path);

        RenamePaths { from, to, backup }
    }
}

struct RenameContext {
    to_root: PathBuf,
    backup_root: PathBuf,
    is_subdir: bool,
    first: u32,
    verbose: bool,
    prefix: String,
    suffix: String,
}

impl RenameContext {
    fn new(
        from: &Path,
        to_root: PathBuf,
        backup_root: PathBuf,
        first: u32,
        verbose: bool,
        prefix: String,
        suffix: String,
    ) -> RenameContext {
        let is_subdir = to_root != from && to_root.starts_with(&from);

        RenameContext {
            to_root,
            backup_root,
            is_subdir,
            first,
            verbose,
            prefix,
            suffix,
        }
    }
}

pub(crate) fn execute(args: RnArgs) -> Result<()> {
    let from = args.from.as_path();
    let to = args.to.as_deref().unwrap_or(from);
    let paths = RenamePaths::setup(from, to)?;
    let context = RenameContext::new(
        &paths.from,
        paths.to.clone(),
        paths.backup.clone(),
        args.first,
        args.verbose,
        args.prefix,
        args.suffix,
    );
    rename_files(paths, &context)?;
    fs::remove_dir(&context.backup_root)?;
    Ok(())
}

fn rename_files(paths: RenamePaths, context: &RenameContext) -> Result<()> {
    let mut file_names: Vec<String> = vec![];

    for entry in fs::read_dir(&paths.from)? {
        let entry = entry?;
        let path = dunce::canonicalize(entry.path())?;

        if path.is_dir() {
            if (context.is_subdir && path == context.to_root) || path == context.backup_root {
                continue;
            }
            let relative = path
                .strip_prefix(&paths.from)
                .expect("path should be inside from");
            let paths = paths.join(&relative);
            rename_files(paths, context)?;
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

    fs::create_dir_all(&paths.backup)?;
    copy_and_rename_to_backup(&paths, context, &file_names)?;
    remove_src_files(&paths.from, &file_names)?;
    move_backup_to_dst(&paths.backup, &paths.to)?;
    if paths.backup != context.backup_root {
        fs::remove_dir(paths.backup)?;
    }
    Ok(())
}

fn copy_and_rename_to_backup(
    paths: &RenamePaths,
    context: &RenameContext,
    file_names: &[String],
) -> Result<()> {
    let mut count = context.first;
    let width = (count + file_names.len() as u32)
        .saturating_sub(1)
        .to_string()
        .len()
        .max(3);
    for f in file_names {
        let ext = extension(&f);
        let new_name = format!(
            "{}{:0width$}{}.{ext}",
            context.prefix,
            count,
            context.suffix,
            width = width
        );
        if context.verbose {
            println!(
                "will be renamed to: {} -> {}",
                paths.from.join(f).display(),
                paths.to.join(&new_name).display()
            );
        }
        fs::copy(paths.from.join(f), paths.backup.join(new_name))?;
        count += 1;
    }
    Ok(())
}

fn remove_src_files(from: &Path, file_names: &[String]) -> Result<()> {
    for f in file_names {
        fs::remove_file(from.join(f))?;
    }
    Ok(())
}

fn move_backup_to_dst(backup: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;

    for entry in fs::read_dir(backup)? {
        let entry = entry?;
        let backup_file = entry.path();
        if !backup_file.is_file() {
            continue;
        }

        fs::rename(
            &backup_file,
            to.join(&backup_file.file_name().expect("file_name always exists")),
        )?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use tempfile::*;

    fn execute_with_args(args: impl IntoIterator<Item = OsString>) -> Result<()> {
        let args = std::iter::once(OsString::from("rn")).chain(args);
        let args = RnArgs::try_parse_from(args).expect("arguments should parsed correctly");

        execute(args)
    }

    #[test]
    fn file_renames_to_number() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img_before = path.join("image.jpg");
        fs::write(&img_before, b"test image")?;

        execute_with_args([path.as_os_str().to_os_string()])?;

        let img_after = path.join("001.jpg");

        assert!(!img_before.exists());
        assert_eq!(fs::read(img_after)?, b"test image");

        Ok(())
    }

    #[test]
    fn multiple_files_renamed_in_order() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img = path.join("image1.jpg");
        fs::write(&img, b"image1")?;
        let img = path.join("image10.jpg");
        fs::write(&img, b"image10")?;
        let img = path.join("image2.jpg");
        fs::write(&img, b"image2")?;

        execute_with_args([path.as_os_str().to_os_string()])?;

        let img = path.join("001.jpg");
        assert_eq!(fs::read(img)?, b"image1");
        let img = path.join("002.jpg");
        assert_eq!(fs::read(img)?, b"image2");
        let img = path.join("003.jpg");
        assert_eq!(fs::read(img)?, b"image10");

        Ok(())
    }

    #[test]
    fn non_target_files_untouched() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let jpg = path.join("image1.jpg");
        fs::write(&jpg, b"jpg")?;
        let jpeg = path.join("image2.jpeg");
        fs::write(&jpeg, b"jpeg")?;
        let webp = path.join("image3.webp");
        fs::write(&webp, b"webp")?;
        let txt = path.join("nontarget.txt");
        fs::write(&txt, b"never changed")?;

        execute_with_args([path.as_os_str().to_os_string()])?;

        let jpg = path.join("001.jpg");
        assert_eq!(fs::read(jpg)?, b"jpg");
        let jpeg = path.join("002.jpeg");
        assert_eq!(fs::read(jpeg)?, b"jpeg");
        let webp = path.join("003.webp");
        assert_eq!(fs::read(webp)?, b"webp");
        assert_eq!(fs::read(txt)?, b"never changed");

        Ok(())
    }

    #[test]
    fn numbering_starts_from_first() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img = path.join("image1.jpg");
        fs::write(&img, b"image1")?;
        let img = path.join("image2.jpg");
        fs::write(&img, b"image2")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--first=10"),
        ])?;

        let img = path.join("010.jpg");
        assert_eq!(fs::read(img)?, b"image1");
        let img = path.join("011.jpg");
        assert_eq!(fs::read(img)?, b"image2");

        Ok(())
    }

    #[test]
    fn number_width_expands_past_three_digits() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        for i in 0..500 {
            let img = path.join(format!("{}.jpg", i));
            fs::write(&img, i.to_string())?;
        }

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--first=501"),
        ])?;

        let img = path.join("0501.jpg");
        assert_eq!(fs::read_to_string(&img)?, "0");
        let img = path.join("1000.jpg");
        assert_eq!(fs::read_to_string(&img)?, "499");

        Ok(())
    }

    #[test]
    fn dir_hierarchy_is_retained() -> Result<()> {
        let from = tempdir()?;
        let from_path = from.path();

        let img = from_path.join("image.jpg");
        fs::write(&img, b"jpg")?;

        let subdir1 = tempdir_in(from_path)?;
        let subdir1_path = subdir1.path();
        let img = subdir1_path.join("image.jpeg");
        fs::write(&img, b"jpeg")?;

        let subdir2 = tempdir_in(from_path)?;
        let subdir2_path = subdir2.path();
        let img = subdir2_path.join("image.webp");
        fs::write(&img, b"webp")?;

        let to = tempdir()?;
        let to_path = to.path();
        execute_with_args([
            from_path.as_os_str().to_os_string(),
            OsString::from("--to"),
            to_path.as_os_str().to_os_string(),
        ])?;

        let img = to_path.join("001.jpg");
        assert_eq!(fs::read(&img)?, b"jpg");

        let img = to_path
            .join(
                subdir1_path
                    .strip_prefix(from_path)
                    .expect("prefix always exists"),
            )
            .join("001.jpeg");
        assert_eq!(fs::read(&img)?, b"jpeg");

        let img = to_path
            .join(
                subdir2_path
                    .strip_prefix(from_path)
                    .expect("prefix always exists"),
            )
            .join("001.webp");
        assert_eq!(fs::read(&img)?, b"webp");

        Ok(())
    }

    #[test]
    fn numbering_restarts_in_each_dir() -> Result<()> {
        let from = tempdir()?;
        let from_path = from.path();

        let subdir1 = tempdir_in(from_path)?;
        let subdir1_path = subdir1.path();
        let img = subdir1_path.join("image_in_subdir1.jpg");
        fs::write(&img, b"image in subdir1")?;

        let subdir2 = tempdir_in(from_path)?;
        let subdir2_path = subdir2.path();
        let img = subdir2_path.join("image_in_subdir2.jpg");
        fs::write(&img, b"image in subdir2")?;

        let to = tempdir()?;
        let to_path = to.path();
        execute_with_args([
            from_path.as_os_str().to_os_string(),
            OsString::from("--to"),
            to_path.as_os_str().to_os_string(),
            OsString::from("--first=1000"),
        ])?;

        let img = to_path
            .join(
                subdir1_path
                    .strip_prefix(from_path)
                    .expect("prefix always exists"),
            )
            .join("1000.jpg");
        assert_eq!(fs::read(&img)?, b"image in subdir1");

        let img = to_path
            .join(
                subdir2_path
                    .strip_prefix(from_path)
                    .expect("prefix always exists"),
            )
            .join("1000.jpg");
        assert_eq!(fs::read(&img)?, b"image in subdir2");

        Ok(())
    }

    #[test]
    fn dst_inside_src_is_skipped() -> Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let img = dir_path.join("img1.jpg");
        fs::write(&img, b"img1")?;

        let subdir1 = tempdir_in(dir_path)?;
        let subdir1_path = subdir1.path();
        let img = subdir1_path.join("img2.jpg");
        fs::write(&img, b"img2")?;

        let subdir2 = tempdir_in(subdir1_path)?;
        let subdir2_path = subdir2.path();
        let img = subdir2_path.join("img3.jpg");
        fs::write(&img, b"img3")?;

        execute_with_args([
            dir_path.as_os_str().to_os_string(),
            OsString::from("--to"),
            subdir2_path.as_os_str().to_os_string(),
        ])?;

        let img = subdir2_path.join("img3.jpg");
        assert_eq!(fs::read(&img)?, b"img3");

        let img = subdir2_path.join("001.jpg");
        assert_eq!(fs::read(&img)?, b"img1");

        let img = subdir2_path
            .join(
                subdir1_path
                    .strip_prefix(dir_path)
                    .expect("prefix always exists"),
            )
            .join("001.jpg");
        assert_eq!(fs::read(&img)?, b"img2");

        Ok(())
    }

    #[test]
    fn in_place_renaming_does_not_overwrite_files() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        for i in 1..=10 {
            let img = path.join(format!("{:03}.jpg", i));
            fs::write(&img, i.to_string())?;
        }

        execute_with_args([path.as_os_str().to_os_string(), OsString::from("--first=3")])?;

        for i in 3..=12 {
            let img = path.join(format!("{:03}.jpg", i));
            assert_eq!(fs::read_to_string(&img)?, format!("{}", i - 2));
        }

        Ok(())
    }

    #[test]
    fn existing_dst_file_is_overwritten() -> Result<()> {
        let from = tempdir()?;
        let from_path = from.path();

        let img = from_path.join("from_image.jpg");
        fs::write(&img, b"from image")?;

        let to = tempdir()?;
        let to_path = to.path();
        let img = to_path.join("001.jpg");
        fs::write(&img, b"to image")?;

        execute_with_args([
            from_path.as_os_str().to_os_string(),
            OsString::from("--to"),
            to_path.as_os_str().to_os_string(),
        ])?;
        assert_eq!(fs::read(&img)?, b"from image");

        Ok(())
    }

    #[test]
    fn file_renames_with_prefix() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img = path.join("image.jpg");
        fs::write(&img, b"test image")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--prefix=new_"),
            OsString::from("--first=54321"),
        ])?;

        let img = path.join("new_54321.jpg");
        assert_eq!(fs::read(img)?, b"test image");

        Ok(())
    }

    #[test]
    fn file_renames_with_suffix() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img = path.join("image.jpg");
        fs::write(&img, b"test image")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--suffix=-IMG"),
            OsString::from("--first=54321"),
        ])?;

        let img = path.join("54321-IMG.jpg");
        assert_eq!(fs::read(img)?, b"test image");

        Ok(())
    }

    #[test]
    fn file_renames_with_affixes() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img = path.join("image.jpg");
        fs::write(&img, b"test image")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--prefix=new_"),
            OsString::from("--suffix=-IMG"),
            OsString::from("--first=54321"),
        ])?;

        let img = path.join("new_54321-IMG.jpg");
        assert_eq!(fs::read(img)?, b"test image");

        Ok(())
    }

    #[test]
    fn affix_rejects_path_separators() {
        assert!(RnArgs::parse_affix("/").is_err());
        assert!(RnArgs::parse_affix("\\").is_err());
    }
}
