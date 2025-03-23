use serde::Deserialize;
use std::path::Path;

use crate::util::add_trailing_slash_if_nonempty;

#[derive(Deserialize)]
pub struct Config {
    pub input: InputConfig,
    pub thumbnail: ThumbnailConfig,
    pub output: OutputConfig,
}

#[derive(Deserialize)]
pub struct InputConfig {
    #[serde(default = "default_input_path")]
    pub path: String,
}

fn default_input_path() -> String {
    String::from("**")
}

#[derive(Deserialize)]
pub struct ThumbnailConfig {
    #[serde(default = "default_thumbnail_resolution")]
    pub resolution: u32,
}

fn default_thumbnail_resolution() -> u32 {
    100
}

#[derive(Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_path")]
    pub path: String,
}

fn default_output_path() -> String {
    String::from("./output")
}

impl OutputConfig {
    pub fn subdirectory(&self, subdirectory: &str) -> String {
        Path::new(&self.path)
            .join(Path::new(subdirectory))
            .to_str()
            .unwrap()
            .to_owned()
    }

    pub fn variation<const PUBLIC: bool>(
        &self,
        category: &str,
        name: &str,
        variation: &str,
    ) -> String {
        let (name, extension) = name
            .rsplit_once('.')
            .map(|(name, extension)| (name, format!(".{extension}")))
            .unwrap_or((name, String::new()));
        let category = add_trailing_slash_if_nonempty(category);
        let path = format!("{category}{name}{variation}{extension}");
        if PUBLIC {
            format!("/{path}")
        } else {
            self.subdirectory(&path)
        }
    }

    pub fn photo<const PUBLIC: bool>(&self, category: &str, name: &str) -> String {
        self.variation::<PUBLIC>(category, name, "")
    }

    pub fn photo_html<const PUBLIC: bool>(&self, category: &str, name: &str) -> String {
        format!("{}.html", self.variation::<PUBLIC>(category, name, ""))
    }

    pub fn preview<const PUBLIC: bool>(&self, category: &str, name: &str) -> String {
        self.variation::<PUBLIC>(category, name, "_preview")
    }

    pub fn thumbnail<const PUBLIC: bool>(&self, category: &str, name: &str) -> String {
        self.variation::<PUBLIC>(category, name, "_thumbnail")
    }

    pub fn category_html<const PUBLIC: bool>(&self, category: &str, name: &str) -> String {
        format!(
            "{}/index.html",
            self.variation::<PUBLIC>(category, name, "")
        )
    }

    pub fn index_html<const PUBLIC: bool>(&self) -> String {
        format!("{}.html", self.variation::<PUBLIC>("", "index", ""))
    }
}
