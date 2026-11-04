use clap::Parser;
use std::fs::DirEntry;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fs, result};

#[derive(Parser, Debug)]
#[command(
    name = "rn",
    about = "Recursively renames supported media files using sequential numbers.",
    long_about = "\
Recursively searches FROM for supported media files
(currently .jpg, .jpeg, .webp, .png, and .mp4),
sorts them by filename, and renames them using sequential numbers.

The supported file extensions may change as the project evolves.

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
      Rename supported files in the current directory in place.

  rn ./photos --to ./numbered
      Rename supported files from ./photos into ./numbered.

  rn ./photos --prefix IMG_ --suffix _edited
      Rename files with a prefix and suffix."
)]
pub(crate) struct RnArgs {
    #[arg(
        default_value = ".",
        hide_default_value = true,
        help = "Source directory to process (defaults to current directory)"
    )]
    from: PathBuf,

    #[arg(short, long, help = "Destination directory (defaults to FROM)")]
    to: Option<PathBuf>,

    #[arg(
        short,
        long,
        value_parser = RnArgs::parse_first,
        default_value = "1",
        hide_default_value = true,
        help = "Starting number for each directory (defaults to 1)"
    )]
    first: u32,

    #[arg(short, long, help = "Print each source and destination path")]
    verbose: bool,

    #[arg(
        short,
        long,
        value_parser = RnArgs::parse_affix,
        default_value = "",
        hide_default_value = true,
        help = "Prefix for renamed files"
    )]
    prefix: String,

    #[arg(
        short,
        long,
        value_parser = RnArgs::parse_affix,
        default_value = "",
        hide_default_value = true,
        help = "Suffix for renamed files"
    )]
    suffix: String,

    #[arg(
        long,
        conflicts_with_all = [
            "sort_by_modified_time",
            "sort_by_modified_time_desc",
        ],
        help = "Sort by file name in descending order"
    )]
    reverse: bool,

    #[arg(
        long,
        conflicts_with_all = [
            "reverse",
            "sort_by_modified_time_desc",
        ],
        help = "Sort by modification time in ascending order"
    )]
    sort_by_modified_time: bool,

    #[arg(
        long,
        conflicts_with_all = [
            "reverse",
            "sort_by_modified_time",
        ],
        help = "Sort by modification time in descending order"
    )]
    sort_by_modified_time_desc: bool,
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
    fn setup(from: &Path, to: Option<&Path>) -> Result<RenamePaths> {
        let from = dunce::canonicalize(from)?;

        let to = to.unwrap_or(from.as_path());
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

enum SortBy {
    FileName { desc: bool },
    ModificationTime { desc: bool },
}

struct RenameContext {
    to_root: PathBuf,
    backup_root: PathBuf,
    is_subdir: bool,
    first: u32,
    verbose: bool,
    prefix: String,
    suffix: String,
    sort_by: SortBy,
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
        reverse: bool,
        sort_by_modified_time: bool,
        sort_by_modified_time_desc: bool,
    ) -> RenameContext {
        let is_subdir = to_root != from && to_root.starts_with(&from);
        let sort_by = if reverse {
            SortBy::FileName { desc: true }
        } else if sort_by_modified_time {
            SortBy::ModificationTime { desc: false }
        } else if sort_by_modified_time_desc {
            SortBy::ModificationTime { desc: true }
        } else {
            SortBy::FileName { desc: false }
        };

        RenameContext {
            to_root,
            backup_root,
            is_subdir,
            first,
            verbose,
            prefix,
            suffix,
            sort_by,
        }
    }
}

struct TargetFile {
    name: String,
    modified: SystemTime,
}

impl TargetFile {
    fn build(entry: &DirEntry) -> Result<Option<TargetFile>> {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_target(&name) {
            return Ok(None);
        }
        let modified = entry.metadata()?.modified()?;

        Ok(Some(TargetFile { name, modified }))
    }
}

struct Entries {
    target_files: Vec<TargetFile>,
    subdirs: Vec<RenamePaths>,
}

impl Entries {
    fn new() -> Entries {
        Entries {
            target_files: vec![],
            subdirs: vec![],
        }
    }
}

pub(crate) fn execute(args: RnArgs) -> Result<()> {
    let paths = RenamePaths::setup(args.from.as_path(), args.to.as_deref())?;
    let context = RenameContext::new(
        &paths.from,
        paths.to.clone(),
        paths.backup.clone(),
        args.first,
        args.verbose,
        args.prefix,
        args.suffix,
        args.reverse,
        args.sort_by_modified_time,
        args.sort_by_modified_time_desc,
    );
    rename_files(paths, &context)?;
    fs::remove_dir(&context.backup_root)?;
    Ok(())
}

fn rename_files(paths: RenamePaths, context: &RenameContext) -> Result<()> {
    let mut entries = collect_entries(&paths, context)?;
    for subdir in entries.subdirs {
        rename_files(subdir, context)?;
    }

    sort_files(&mut entries.target_files, &context.sort_by);
    fs::create_dir_all(&paths.backup)?;
    copy_and_rename_to_backup(&paths, context, &entries.target_files)?;
    remove_src_files(&paths.from, &entries.target_files)?;
    move_backup_to_dst(&paths.backup, &paths.to)?;
    if paths.backup != context.backup_root {
        fs::remove_dir(paths.backup)?;
    }
    Ok(())
}

fn collect_entries(paths: &RenamePaths, context: &RenameContext) -> Result<Entries> {
    let mut entries = Entries::new();

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
            entries.subdirs.push(paths);
            continue;
        }

        if let Some(target_file) = TargetFile::build(&entry)? {
            entries.target_files.push(target_file);
        }
    }

    Ok(entries)
}

fn sort_files(target_files: &mut [TargetFile], sort_by: &SortBy) {
    target_files.sort_by(|a, b| {
        let a = a.name.to_lowercase();
        let a = filename(&a);
        let b = b.name.to_lowercase();
        let b = filename(&b);

        alphanumeric_sort::compare_str(a, b)
    });
}

fn copy_and_rename_to_backup(
    paths: &RenamePaths,
    context: &RenameContext,
    target_files: &[TargetFile],
) -> Result<()> {
    let mut count = context.first;
    let width = (count + target_files.len() as u32)
        .saturating_sub(1)
        .to_string()
        .len()
        .max(3);
    for file in target_files {
        let ext = extension(&file.name);
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
                paths.from.join(&file.name).display(),
                paths.to.join(&new_name).display()
            );
        }
        fs::copy(paths.from.join(&file.name), paths.backup.join(new_name))?;
        count += 1;
    }
    Ok(())
}

fn remove_src_files(from: &Path, target_files: &[TargetFile]) -> Result<()> {
    for file in target_files {
        fs::remove_file(from.join(&file.name))?;
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

static TARGET: [&str; 5] = ["jpg", "jpeg", "webp", "png", "mp4"];

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

    fn make_file<'a>(path: PathBuf, content: &[u8]) -> Result<PathBuf> {
        fs::write(&path, content)?;
        Ok(path)
    }

    #[test]
    fn file_renames_to_number() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        let img_before = make_file(path.join("image.jpg"), b"test image")?;

        execute_with_args([path.as_os_str().to_os_string()])?;

        assert!(!img_before.exists());
        assert_eq!(fs::read(path.join("001.jpg"))?, b"test image");

        Ok(())
    }

    #[test]
    fn multiple_files_renamed_in_order() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        make_file(path.join("image1.jpg"), b"image1")?;
        make_file(path.join("image10.jpg"), b"image10")?;
        make_file(path.join("image2.jpg"), b"image2")?;

        execute_with_args([path.as_os_str().to_os_string()])?;

        assert_eq!(fs::read(path.join("001.jpg"))?, b"image1");
        assert_eq!(fs::read(path.join("002.jpg"))?, b"image2");
        assert_eq!(fs::read(path.join("003.jpg"))?, b"image10");

        Ok(())
    }

    #[test]
    fn non_target_files_untouched() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        make_file(path.join("image1.jpg"), b"jpg")?;
        make_file(path.join("image2.jpeg"), b"jpeg")?;
        make_file(path.join("image3.webp"), b"webp")?;
        let txt = make_file(path.join("not_target.txt"), b"never changed")?;

        execute_with_args([path.as_os_str().to_os_string()])?;

        assert_eq!(fs::read(path.join("001.jpg"))?, b"jpg");
        assert_eq!(fs::read(path.join("002.jpeg"))?, b"jpeg");
        assert_eq!(fs::read(path.join("003.webp"))?, b"webp");
        assert_eq!(fs::read(txt)?, b"never changed");

        Ok(())
    }

    #[test]
    fn numbering_starts_from_first() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        make_file(path.join("image1.jpg"), b"image1")?;
        make_file(path.join("image2.jpg"), b"image2")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--first=10"),
        ])?;

        assert_eq!(fs::read(path.join("010.jpg"))?, b"image1");
        assert_eq!(fs::read(path.join("011.jpg"))?, b"image2");

        Ok(())
    }

    #[test]
    fn number_width_expands_past_three_digits() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        for i in 0..500 {
            make_file(path.join(format!("{}.jpg", i)), i.to_string().as_bytes())?;
        }

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--first=501"),
        ])?;

        assert_eq!(fs::read_to_string(path.join("0501.jpg"))?, "0");
        assert_eq!(fs::read_to_string(path.join("1000.jpg"))?, "499");

        Ok(())
    }

    #[test]
    fn dir_hierarchy_is_retained() -> Result<()> {
        let from = tempdir()?;
        let from_path = from.path();
        make_file(from_path.join("image.jpg"), b"jpg")?;

        let subdir1 = tempdir_in(from_path)?;
        let subdir1_path = subdir1.path();
        make_file(subdir1_path.join("image.jpeg"), b"jpeg")?;

        let subdir2 = tempdir_in(from_path)?;
        let subdir2_path = subdir2.path();
        make_file(subdir2_path.join("image.webp"), b"webp")?;

        let to = tempdir()?;
        let to_path = to.path();
        execute_with_args([
            from_path.as_os_str().to_os_string(),
            OsString::from("--to"),
            to_path.as_os_str().to_os_string(),
        ])?;

        assert_eq!(fs::read(to_path.join("001.jpg"))?, b"jpg");

        let img = to_path
            .join(
                subdir1_path
                    .strip_prefix(from_path)
                    .expect("prefix always exists"),
            )
            .join("001.jpeg");
        assert_eq!(fs::read(img)?, b"jpeg");

        let img = to_path
            .join(
                subdir2_path
                    .strip_prefix(from_path)
                    .expect("prefix always exists"),
            )
            .join("001.webp");
        assert_eq!(fs::read(img)?, b"webp");

        Ok(())
    }

    #[test]
    fn numbering_restarts_in_each_dir() -> Result<()> {
        let from = tempdir()?;
        let from_path = from.path();

        let subdir1 = tempdir_in(from_path)?;
        let subdir1_path = subdir1.path();
        make_file(
            subdir1_path.join("image_in_subdir1.jpg"),
            b"image in subdir1",
        )?;

        let subdir2 = tempdir_in(from_path)?;
        let subdir2_path = subdir2.path();
        make_file(
            subdir2_path.join("image_in_subdir2.jpg"),
            b"image in subdir2",
        )?;

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
        assert_eq!(fs::read(img)?, b"image in subdir1");

        let img = to_path
            .join(
                subdir2_path
                    .strip_prefix(from_path)
                    .expect("prefix always exists"),
            )
            .join("1000.jpg");
        assert_eq!(fs::read(img)?, b"image in subdir2");

        Ok(())
    }

    #[test]
    fn dst_inside_src_is_skipped() -> Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();
        make_file(dir_path.join("img1.jpg"), b"img1")?;

        let subdir1 = tempdir_in(dir_path)?;
        let subdir1_path = subdir1.path();
        make_file(subdir1_path.join("img2.jpg"), b"img2")?;

        let subdir2 = tempdir_in(subdir1_path)?;
        let subdir2_path = subdir2.path();
        make_file(subdir2_path.join("img3.jpg"), b"img3")?;

        execute_with_args([
            dir_path.as_os_str().to_os_string(),
            OsString::from("--to"),
            subdir2_path.as_os_str().to_os_string(),
        ])?;

        assert_eq!(fs::read(subdir2_path.join("img3.jpg"))?, b"img3");
        assert_eq!(fs::read(subdir2_path.join("001.jpg"))?, b"img1");

        let img = subdir2_path
            .join(
                subdir1_path
                    .strip_prefix(dir_path)
                    .expect("prefix always exists"),
            )
            .join("001.jpg");
        assert_eq!(fs::read(img)?, b"img2");

        Ok(())
    }

    #[test]
    fn in_place_renaming_does_not_overwrite_files() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        for i in 1..=10 {
            make_file(path.join(format!("{:03}.jpg", i)), i.to_string().as_bytes())?;
        }

        execute_with_args([path.as_os_str().to_os_string(), OsString::from("--first=3")])?;

        for i in 3..=12 {
            let img = path.join(format!("{:03}.jpg", i));
            assert_eq!(fs::read_to_string(img)?, format!("{}", i - 2));
        }

        Ok(())
    }

    #[test]
    fn existing_dst_file_is_overwritten() -> Result<()> {
        let from = tempdir()?;
        let from_path = from.path();
        make_file(from_path.join("from_image.jpg"), b"from image")?;

        let to = tempdir()?;
        let to_path = to.path();
        make_file(to_path.join("001.jpg"), b"to image")?;

        execute_with_args([
            from_path.as_os_str().to_os_string(),
            OsString::from("--to"),
            to_path.as_os_str().to_os_string(),
        ])?;
        assert_eq!(fs::read(to_path.join("001.jpg"))?, b"from image");

        Ok(())
    }

    #[test]
    fn file_renames_with_prefix() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        make_file(path.join("from_image.jpg"), b"test image")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--prefix=new_"),
            OsString::from("--first=54321"),
        ])?;

        assert_eq!(fs::read(path.join("new_54321.jpg"))?, b"test image");

        Ok(())
    }

    #[test]
    fn file_renames_with_suffix() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        make_file(path.join("image.jpg"), b"test image")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--suffix=-IMG"),
            OsString::from("--first=54321"),
        ])?;

        assert_eq!(fs::read(path.join("54321-IMG.jpg"))?, b"test image");

        Ok(())
    }

    #[test]
    fn file_renames_with_affixes() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        make_file(path.join("image.jpg"), b"test image")?;

        execute_with_args([
            path.as_os_str().to_os_string(),
            OsString::from("--prefix=new_"),
            OsString::from("--suffix=-IMG"),
            OsString::from("--first=54321"),
        ])?;

        assert_eq!(fs::read(path.join("new_54321-IMG.jpg"))?, b"test image");

        Ok(())
    }

    #[test]
    fn affix_rejects_path_separators() {
        assert!(RnArgs::parse_affix("/").is_err());
        assert!(RnArgs::parse_affix("\\").is_err());
    }

    #[test]
    fn files_sort_by_filename_desc() -> Result<()> {
        Ok(())
    }

    #[test]
    fn files_sort_by_modified_time_asc() -> Result<()> {
        Ok(())
    }

    #[test]
    fn files_sort_by_modified_time_desc() -> Result<()> {
        Ok(())
    }

    #[test]
    fn files_with_same_modified_time_sort_by_filename() -> Result<()> {
        Ok(())
    }
}
