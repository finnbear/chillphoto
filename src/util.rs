use std::fs;
use std::io;
use std::path::Path;

use base64::Engine;
use chrono::NaiveDate;

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

pub fn date_format(date: NaiveDate) -> String {
    date.format("%-d %b, %C%y").to_string()
}
