use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

use base64::Engine;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;

// https://stackoverflow.com/a/65573340/3064544
pub fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub fn recursively_remove_empty_dirs_of_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    pub fn recursively_remove_empty_dirs<P: AsRef<Path>>(path: P) -> io::Result<bool> {
        let mut keep = false;
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type()?.is_dir() {
                keep |= recursively_remove_empty_dirs(path)?;
            } else {
                keep = true;
            }
        }
        if !keep {
            fs::remove_dir(&path).unwrap();
        }
        Ok(keep)
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            recursively_remove_empty_dirs(path)?;
        }
    }
    Ok(())
}

// TODO: wait for `slice_concat_ext` stabilization.
pub fn join<T: Clone>(slice: &[T], sep: &T) -> Vec<T> {
    let mut iter = slice.iter();
    let first = match iter.next() {
        Some(first) => first,
        None => return vec![],
    };
    let size = slice.len() * 2 - 1;
    let mut result = Vec::with_capacity(size);
    result.extend_from_slice(std::slice::from_ref(first));

    for v in iter {
        result.push(sep.clone());
        result.extend_from_slice(std::slice::from_ref(v))
    }
    result
}

pub fn add_trailing_slash_if_nonempty(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!("{path}/")
    }
}

pub fn checksum(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(&md5::compute(b).0)
}

pub fn is_camera_file_name(name: &str) -> bool {
    name.starts_with("IMG") || name.starts_with("DSC")
}

pub fn progress_bar(name: &str, count: usize, start: Instant) -> ProgressBar {
    ProgressBar::new(count as u64)
        .with_message(name.to_owned())
        .with_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar} {pos}/{len} {eta}")
                .unwrap(),
        )
        .with_elapsed(start.elapsed())
}