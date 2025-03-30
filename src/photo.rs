use crate::{exif::ExifData, generate_thumbnail, resize_image, CONFIG};
use image::RgbImage;
use std::{fmt::Debug, sync::OnceLock};

#[derive(Clone, PartialEq)]
pub struct Photo {
    pub name: String,
    pub description: Option<String>,
    pub input_image_data: Vec<u8>,
    pub image: OnceLock<RgbImage>,
    pub preview: OnceLock<RgbImage>,
    pub thumbnail: OnceLock<RgbImage>,
    pub exif: ExifData,
}

impl Photo {
    pub fn image(&self) -> &RgbImage {
        self.image.get_or_init(|| {
            resize_image(
                &image::load_from_memory(&self.input_image_data)
                    .expect("failed to open image")
                    .to_rgb8(),
                CONFIG.photo_resolution,
            )
        })
    }

    pub fn preview(&self) -> &RgbImage {
        self.preview
            .get_or_init(|| resize_image(self.image(), CONFIG.preview_resolution))
    }

    pub fn thumbnail(&self) -> &RgbImage {
        self.thumbnail
            .get_or_init(|| generate_thumbnail(self.image()))
    }
}

impl Debug for Photo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Photo")
            .field("name", &self.name)
            .field("exif", &self.exif)
            .field("resolution", &(self.image().width(), self.image().height()))
            .finish_non_exhaustive()
    }
}
