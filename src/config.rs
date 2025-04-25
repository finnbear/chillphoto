use crate::{
    category_path::CategoryPath, format::OutputFormat, util::add_trailing_slash_if_nonempty,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
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
    pub acquire_license_url: Option<String>,
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
    #[serde(default = "default_image_ai_model")]
    pub image_ai_model: String,
    #[serde(default = "default_ai_description_system_prompt")]
    pub ai_description_system_prompt: String,
    #[serde(default)]
    pub pagination_flavor: PaginationFlavor,
    #[serde(default = "default_items_per_page")]
    pub items_per_page: usize,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaginationFlavor {
    #[default]
    Path,
    Query,
}

fn default_items_per_page() -> usize {
    30
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

fn default_image_ai_model() -> String {
    String::from("gemma3")
}

fn default_ai_description_system_prompt() -> String {
    String::from("You are a photo summarizer tasked with generating descriptions for photos on an gallery website, with an emphasis on accessibility. Visually-impaired people will rely on your descriptions, so make them accurate and interesting. You never explicitly speculate, mention a lack of text, or use more than 2 sentences.")
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

    pub fn category_html<const PUBLIC: bool>(
        &self,
        category: &CategoryPath,
        name: &str,
        page: usize,
    ) -> String {
        let base = self.variation::<PUBLIC>(category, name, "");
        format!("{base}{}", self.index_html::<PUBLIC>(page))
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

    pub fn index_html<const PUBLIC: bool>(&self, page: usize) -> String {
        let base = if page == 0 || matches!(self.pagination_flavor, PaginationFlavor::Query) {
            "/".to_owned()
        } else {
            format!("/page/{}/", page + 1)
        };
        let filename = if PUBLIC { "" } else { "index.html" }.to_owned();
        let suffix = if page > 0 && matches!(self.pagination_flavor, PaginationFlavor::Query) {
            format!("?page={}", page + 1)
        } else {
            "".to_owned()
        };
        format!("{base}{filename}{suffix}")
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
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub ai_description_hint: Option<String>,
    #[serde(default)]
    pub ai_description_input_checksum: Option<String>,
    #[serde(default)]
    pub ai_description_output_checksum: Option<String>,
    #[serde(default)]
    pub order: i64,
    #[serde(default = "default_thumbnail_crop_factor")]
    pub thumbnail_crop_factor: f64,
    #[serde(default = "default_thumbnail_crop_center")]
    pub thumbnail_crop_center: Point2,
    /// Stops of exposure to add.
    #[serde(default)]
    pub exposure: f32,
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
    #[serde(default)]
    pub ai_description_hint: Option<String>,
    #[serde(default = "default_items_per_page")]
    pub items_per_page: usize,
}

impl Default for CategoryConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

#[derive(Deserialize, Debug)]
pub struct PageConfig {
    #[serde(default)]
    pub order: i64,
    pub description: Option<String>,
}

impl Default for PageConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}
