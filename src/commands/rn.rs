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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::*;

    #[test]
    fn file_renames_to_number() -> io::Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img_before = path.join("image.jpg");
        fs::write(&img_before, b"test image")?;

        execute(path, path, 1, false)?;

        let img_after = path.join("001.jpg");

        assert!(!img_before.exists());
        assert_eq!(fs::read(img_after)?, b"test image");

        Ok(())
    }

    #[test]
    fn multiple_files_renamed_in_order() -> io::Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img = path.join("image1.jpg");
        fs::write(&img, b"image1")?;
        let img = path.join("image10.jpg");
        fs::write(&img, b"image10")?;
        let img = path.join("image2.jpg");
        fs::write(&img, b"image2")?;

        execute(path, path, 1, false)?;

        let img = path.join("001.jpg");
        assert_eq!(fs::read(img)?, b"image1");
        let img = path.join("002.jpg");
        assert_eq!(fs::read(img)?, b"image2");
        let img = path.join("003.jpg");
        assert_eq!(fs::read(img)?, b"image10");

        Ok(())
    }

    #[test]
    fn non_target_files_untouched() -> io::Result<()> {
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

        execute(path, path, 1, false)?;

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
    fn numbering_starts_from_first() -> io::Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        let img = path.join("image1.jpg");
        fs::write(&img, b"image1")?;
        let img = path.join("image2.jpg");
        fs::write(&img, b"image2")?;

        execute(path, path, 10, false)?;

        let img = path.join("010.jpg");
        assert_eq!(fs::read(img)?, b"image1");
        let img = path.join("011.jpg");
        assert_eq!(fs::read(img)?, b"image2");

        Ok(())
    }

    #[test]
    fn number_width_expands_past_three_digits() -> io::Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        for i in 0..500 {
            let img = path.join(format!("{}.jpg", i));
            fs::write(&img, i.to_string())?;
        }

        execute(path, path, 501, false)?;
        let img = path.join("0501.jpg");
        assert_eq!(fs::read_to_string(&img)?, "0");
        let img = path.join("1000.jpg");
        assert_eq!(fs::read_to_string(&img)?, "499");

        Ok(())
    }

    #[test]
    fn dir_hierarchy_is_retained() -> io::Result<()> {
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
        execute(from_path, to_path, 1, false)?;

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
    fn numbering_restarts_in_each_dir() -> io::Result<()> {
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
        execute(from_path, to_path, 1000, false)?;

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
    fn dst_inside_src_is_skipped() -> io::Result<()> {
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

        execute(dir_path, subdir2_path, 1, false)?;

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
    fn in_place_renaming_does_not_overwrite_files() -> io::Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        for i in 1..=10 {
            let img = path.join(format!("{:03}.jpg", i));
            fs::write(&img, i.to_string())?;
        }

        execute(path, path, 3, false)?;
        for i in 3..=12 {
            let img = path.join(format!("{:03}.jpg", i));
            assert_eq!(fs::read_to_string(&img)?, format!("{}", i - 2));
        }

        Ok(())
    }

    #[test]
    fn existing_dst_file_is_overwritten() -> io::Result<()> {
        let from = tempdir()?;
        let from_path = from.path();
        
        let img = from_path.join("from_image.jpg");
        fs::write(&img, b"from image")?;

        let to = tempdir()?;
        let to_path = to.path();
        let img = to_path.join("001.jpg");
        fs::write(&img, b"to image")?;

        execute(from_path, to_path, 1, false)?;
        assert_eq!(fs::read(&img)?, b"from image");

        Ok(())
    }
}
