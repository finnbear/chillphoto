use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    #[serde(rename = "png")]
    Png,
    #[serde(rename = "jpg", alias = "jpeg")]
    Jpg,
    #[serde(rename = "webp")]
    Webp,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.extension())
    }
}

pub struct InvalidImageFormat;

impl FromStr for OutputFormat {
    type Err = InvalidImageFormat;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(&format!("\"{s}\"")).map_err(|_| InvalidImageFormat)
    }
}

impl OutputFormat {
    pub fn extension(self) -> String {
        serde_json::to_string(&self)
            .unwrap()
            .trim_matches('"')
            .to_owned()
    }
}
