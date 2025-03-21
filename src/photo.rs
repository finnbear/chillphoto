use crate::exif::ExifData;
use image::RgbImage;
use std::fmt::Debug;

pub struct Photo {
    pub name: String,
    pub image: RgbImage,
    pub preview: RgbImage,
    pub thumbnail: RgbImage,
    pub exif: ExifData,
}

impl Debug for Photo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Photo")
            .field("name", &self.name)
            .field("exif", &self.exif)
            .field("resolution", &(self.image.width(), self.image.height()))
            .finish_non_exhaustive()
    }
}
