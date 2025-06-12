use serde::Serialize;

use crate::gallery::Gallery;

/// Must be at least 144.
pub const MANIFEST_ICON_RESOLUTION: u32 = 256;

pub fn write_manifest(gallery: &Gallery) -> Vec<u8> {
    let manifest = Manifest {
        name: gallery.config.title.clone(),
        description: gallery.config.description.clone(),
        display: "standalone".to_owned(),
        categories: gallery.config.categories.clone(),
        start_url: "/".to_owned(),
        handle_links: "not-preferred".to_owned(),
        icons: gallery
            .thumbnail()
            .map(|(_, _)| Icon {
                src: "/manifest.png".to_owned(),
                sizes: format!("{}x{}", MANIFEST_ICON_RESOLUTION, MANIFEST_ICON_RESOLUTION),
            })
            .into_iter()
            .collect::<Vec<_>>(),
    };

    serde_json::to_string(&manifest).unwrap().into_bytes()
}

#[derive(Serialize)]
struct Manifest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    display: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    categories: Vec<String>,
    start_url: String,
    handle_links: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    icons: Vec<Icon>,
}

#[derive(Serialize)]
struct Icon {
    src: String,
    sizes: String,
}
