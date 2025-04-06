use crate::{
    category_path::CategoryPath, format::OutputFormat, util::add_trailing_slash_if_nonempty,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GalleryConfig {
    #[serde(default = "default_input")]
    pub input: String,
    #[serde(default = "default_output")]
    pub output: String,
    #[serde(default = "default_title")]
    pub title: String,
    pub author: Option<String>,
    pub root_url: Option<String>,
    pub author_url: Option<String>,
    pub license_url: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub disallow_ai_training: bool,
    #[serde(default = "default_categories")]
    pub categories: Vec<String>,
    #[serde(default = "default_photo_format")]
    pub photo_format: OutputFormat,
    #[serde(default = "default_photo_resolution")]
    pub photo_resolution: u32,
    #[serde(default = "default_preview_format")]
    pub preview_format: OutputFormat,
    #[serde(default = "default_preview_resolution")]
    pub preview_resolution: u32,
    #[serde(default = "default_thumbnail_format")]
    pub thumbnail_format: OutputFormat,
    #[serde(default = "default_thumbnail_resolution")]
    pub thumbnail_resolution: u32,
}

fn default_input() -> String {
    String::from("**/*.{png,PNG,jpg,JPG,jpeg,JPEG,txt,md,html,toml}")
}

fn default_title() -> String {
    "My Gallery".to_owned()
}

fn default_categories() -> Vec<String> {
    vec!["photo".to_owned()]
}

fn default_photo_format() -> OutputFormat {
    OutputFormat::Jpg
}

fn default_photo_resolution() -> u32 {
    3840
}

fn default_preview_format() -> OutputFormat {
    OutputFormat::Jpg
}

fn default_preview_resolution() -> u32 {
    1920
}

fn default_thumbnail_format() -> OutputFormat {
    OutputFormat::Jpg
}

fn default_thumbnail_resolution() -> u32 {
    100
}

fn default_output() -> String {
    String::from("./output")
}

impl GalleryConfig {
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
            self.photo_format
        )
    }

    pub fn photo_html<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        let base = format!("{}/", self.variation::<PUBLIC>(category, name, ""));
        if PUBLIC {
            base
        } else {
            format!("{base}index.html")
        }
    }

    pub fn preview<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!(
            "{}.{}",
            self.variation::<PUBLIC>(category, name, "_preview"),
            self.preview_format
        )
    }

    pub fn thumbnail<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        format!(
            "{}.{}",
            self.variation::<PUBLIC>(category, name, "_thumbnail"),
            self.thumbnail_format
        )
    }

    pub fn category_html<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        let base = format!("{}/", self.variation::<PUBLIC>(category, name, ""));
        if PUBLIC {
            base
        } else {
            format!("{base}index.html",)
        }
    }

    pub fn favicon<const PUBLIC: bool>(&self) -> String {
        format!(
            "{}.png",
            self.variation::<PUBLIC>(&CategoryPath::ROOT, "favicon", "")
        )
    }

    pub fn manifest<const PUBLIC: bool>(&self) -> String {
        format!(
            "{}.json",
            self.variation::<PUBLIC>(&CategoryPath::ROOT, "manifest", "")
        )
    }

    pub fn page_html<const PUBLIC: bool>(&self, category: &CategoryPath, name: &str) -> String {
        let base = format!("{}/", self.variation::<PUBLIC>(category, name, ""));
        if PUBLIC {
            base
        } else {
            format!("{base}index.html")
        }
    }

    pub fn index_html<const PUBLIC: bool>(&self) -> String {
        if PUBLIC { "/" } else { "/index.html" }.to_owned()
    }
}

#[derive(Deserialize, Debug)]
pub struct PhotoConfig {
    /// Author override.
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license_url: Option<String>,
    #[serde(default)]
    pub alt_text: Option<String>,
    #[serde(default)]
    pub order: i64,
    #[serde(default = "default_thumbnail_crop_factor")]
    pub thumbnail_crop_factor: f64,
    #[serde(default = "default_thumbnail_crop_center")]
    pub thumbnail_crop_center: Point2,
}

fn default_thumbnail_crop_factor() -> f64 {
    1.0
}

fn default_thumbnail_crop_center() -> Point2 {
    Point2 { x: 0.5, y: 0.5 }
}

impl Default for PhotoConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

#[derive(Deserialize, Debug)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize, Debug)]
pub struct CategoryConfig {
    #[serde(default)]
    pub order: i64,
    pub thumbnail: Option<String>,
    pub description: Option<String>,
}

impl Default for CategoryConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}
