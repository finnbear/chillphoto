use crate::{exif::ExifData, CONFIG};
use image::{
    imageops::{self, FilterType},
    DynamicImage, RgbImage,
};
use std::{fmt::Debug, sync::OnceLock};

#[derive(PartialEq)]
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
    pub fn image_dimensions(&self) -> (u32, u32) {
        if let Some((width, height)) = self.exif.dimensions() {
            // Avoid decoding the image if we don't have to.
            resize_dimensions(
                width,
                height,
                CONFIG.photo_resolution,
                CONFIG.photo_resolution,
            )
        } else {
            self.image().dimensions()
        }
    }

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

    pub fn preview_dimensions(&self) -> (u32, u32) {
        let (width, height) = self.image_dimensions();
        resize_dimensions(
            width,
            height,
            CONFIG.preview_resolution,
            CONFIG.preview_resolution,
        )
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

// from `image`
fn resize_dimensions(width: u32, height: u32, nwidth: u32, nheight: u32) -> (u32, u32) {
    use std::cmp::max;
    let fill = false;
    let wratio = f64::from(nwidth) / f64::from(width);
    let hratio = f64::from(nheight) / f64::from(height);

    let ratio = if fill {
        f64::max(wratio, hratio)
    } else {
        f64::min(wratio, hratio)
    };

    let nw = max((f64::from(width) * ratio).round() as u64, 1);
    let nh = max((f64::from(height) * ratio).round() as u64, 1);

    if nw > u64::from(u32::MAX) {
        let ratio = f64::from(u32::MAX) / f64::from(width);
        (u32::MAX, max((f64::from(height) * ratio).round() as u32, 1))
    } else if nh > u64::from(u32::MAX) {
        let ratio = f64::from(u32::MAX) / f64::from(height);
        (max((f64::from(width) * ratio).round() as u32, 1), u32::MAX)
    } else {
        (nw as u32, nh as u32)
    }
}

fn generate_thumbnail(img: &RgbImage) -> RgbImage {
    let (width, height) = img.dimensions();
    let size = width.min(height);
    let x_offset = (width - size) / 2;
    let y_offset = (height - size) / 2;
    let cropped = imageops::crop_imm(img, x_offset, y_offset, size, size).to_image();
    imageops::resize(
        &cropped,
        CONFIG.thumbnail_resolution,
        CONFIG.thumbnail_resolution,
        FilterType::Lanczos3,
    )
}

fn resize_image(img: &RgbImage, resolution: u32) -> RgbImage {
    if img.width() <= resolution && img.height() <= resolution {
        return img.clone();
    }
    DynamicImage::ImageRgb8(img.clone())
        .resize(resolution, resolution, FilterType::Lanczos3)
        .to_rgb8()
}
