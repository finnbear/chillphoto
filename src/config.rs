use serde::Deserialize;
use std::path::Path;

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

    pub fn variation(&self, category: &str, name: &str, variation: &str) -> String {
        let (name, extension) = name
            .rsplit_once('.')
            .map(|(name, extension)| (name, format!(".{extension}")))
            .unwrap_or((name, String::new()));
        self.subdirectory(&format!("{category}/{name}{variation}{extension}"))
    }

    pub fn photo(&self, category: &str, name: &str) -> String {
        self.variation(category, name, "")
    }

    pub fn preview(&self, category: &str, name: &str) -> String {
        self.variation(category, name, "_preview")
    }

    pub fn thumbnail(&self, category: &str, name: &str) -> String {
        self.variation(category, name, "_thumbnail")
    }
}
