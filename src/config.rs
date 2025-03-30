use serde::Deserialize;
use std::path::Path;

use crate::{category_path::CategoryPath, util::add_trailing_slash_if_nonempty, CONFIG};

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Config {
    #[serde(default = "default_input")]
    pub input: String,
    #[serde(default = "default_output")]
    pub output: String,
    pub title: String,
    pub author: Option<String>,
    pub author_url: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_photo_format")]
    pub photo_format: String,
    #[serde(default = "default_photo_resolution")]
    pub photo_resolution: u32,
    #[serde(default = "default_preview_format")]
    pub preview_format: String,
    #[serde(default = "default_preview_resolution")]
    pub preview_resolution: u32,
    #[serde(default = "default_thumbnail_format")]
    pub thumbnail_format: String,
    #[serde(default = "default_thumbnail_resolution")]
    pub thumbnail_resolution: u32,
}

fn default_input() -> String {
    String::from("**/*.{png,PNG,jpg,JPG,jpeg,JPEG,txt,md,html}")
}

fn default_photo_format() -> String {
    "jpg".to_owned()
}

fn default_photo_resolution() -> u32 {
    3840
}

fn default_preview_format() -> String {
    "webp".to_owned()
}

fn default_preview_resolution() -> u32 {
    1920
}

fn default_thumbnail_format() -> String {
    "webp".to_owned()
}

fn default_thumbnail_resolution() -> u32 {
    100
}

fn default_output() -> String {
    String::from("./output")
}

impl Config {
    pub fn subdirectory(&self, subdirectory: &str) -> String {
        Path::new(&self.output)
            .join(Path::new(subdirectory))
            .to_str()
            .unwrap()
            .to_owned()
    }

    pub fn variation<const PUBLIC: bool>(
        &self,
        category: &CategoryPath,
        name: &str,
        variation: &str,
    ) -> String {
        let (name, extension) = name
            .rsplit_once('.')
            .map(|(name, extension)| (name, format!(".{extension}")))
            .unwrap_or((name, String::new()));
        let category = add_trailing_slash_if_nonempty(&category.to_string_without_leading_slash());
        let path = format!("{category}{name}{variation}{extension}");
        if true || PUBLIC {
            format!("/{path}")
        } else {
            self.subdirectory(&path)
        }
    }

    pub fn photo<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!(
            "{}.{}",
            self.variation::<PUBLIC>(category, name, ""),
            CONFIG.photo_format
        )
    }

    pub fn photo_html<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!("{}.html", self.variation::<PUBLIC>(category, name, ""))
    }

    pub fn preview<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!(
            "{}.{}",
            self.variation::<PUBLIC>(category, name, "_preview"),
            CONFIG.preview_format
        )
    }

    pub fn thumbnail<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!(
            "{}.{}",
            self.variation::<PUBLIC>(category, name, "_thumbnail"),
            CONFIG.thumbnail_format
        )
    }

    pub fn category_html<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!(
            "{}/index.html",
            self.variation::<PUBLIC>(category, name, "")
        )
    }

    pub fn page_html<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!("{}.html", self.variation::<PUBLIC>(category, name, ""))
    }

    pub fn index_html<const PUBLIC: bool>(&self) -> String {
        format!(
            "{}.html",
            self.variation::<PUBLIC>(&CategoryPath::ROOT, "index", "")
        )
    }
}
