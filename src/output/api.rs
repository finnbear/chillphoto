use crate::gallery::{Gallery, Item, RichTextFormat};
use serde::Serialize;

pub fn render_api(gallery: &Gallery) -> Vec<u8> {
    let mut photos = Vec::<ApiPhoto>::new();

    gallery.visit_items(|path, item| {
        let photo = if let Item::Photo(photo) = item {
            photo
        } else {
            return;
        };

        photos.push(ApiPhoto {
            categories: path
                .iter_paths()
                .filter_map(|p| gallery.category(&p))
                .map(|c| c.name.clone())
                .collect(),
            name: photo.output_name().to_string(),
            page_text_content: photo.text.as_ref().map(|t| t.content.clone()),
            page_text_format: photo.text.as_ref().map(|t| t.format),
            location: photo.config.location.clone(),
            description: photo.config.description.clone(),
            page_path: gallery.config.photo_html::<true>(&path, &photo.slug()),
            photo_path: gallery.config.photo::<true>(&path, &photo.slug()),
            preview_path: gallery.config.preview::<true>(&path, &photo.slug()),
            thumbnail_path: gallery.config.thumbnail::<true>(&path, &photo.slug()),
            date: photo.date_time().map(|d| d.date().to_string()),
            license_url: photo
                .config
                .license_url
                .clone()
                .or(gallery.config.license_url.clone()),
            author: photo
                .config
                .author
                .clone()
                .or(gallery.config.author.clone()),
        });
    });

    let json = serde_json::to_string(&Api {
        title: gallery.config.title.clone(),
        description: gallery.config.description.clone(),
        disallow_ai_training: gallery.config.disallow_ai_training,
        root_url: gallery.config.root_url.clone(),
        photos,
    })
    .unwrap();
    json.into_bytes()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Api {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    disallow_ai_training: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_url: Option<String>,
    photos: Vec<ApiPhoto>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiPhoto {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    categories: Vec<String>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_text_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_text_format: Option<RichTextFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    page_path: String,
    photo_path: String,
    preview_path: String,
    thumbnail_path: String,
    date: Option<String>,
    author: Option<String>,
    license_url: Option<String>,
}
