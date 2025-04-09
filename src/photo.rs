use crate::{
    config::{GalleryConfig, PhotoConfig},
    exif::ExifData,
    gallery::RichText,
};
use chrono::{DateTime, NaiveDateTime};
use image::{
    imageops::{self, FilterType},
    DynamicImage, ImageDecoder, ImageReader, RgbImage,
};
use std::{fmt::Debug, io, sync::OnceLock, time::SystemTime};

pub struct Photo {
    pub name: String,
    pub text: Option<RichText>,
    pub input_image_data: Vec<u8>,
    pub image: OnceLock<RgbImage>,
    pub preview: OnceLock<RgbImage>,
    pub thumbnail: OnceLock<RgbImage>,
    pub exif: ExifData,
    pub file_date: Option<SystemTime>,
    pub config: PhotoConfig,
}

impl Photo {
    pub fn date_time(&self) -> Option<NaiveDateTime> {
        self.exif.date_time().or_else(|| {
            self.file_date.and_then(|fd| {
                DateTime::from_timestamp_millis(
                    fd.duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                )
                .map(|dt| dt.naive_local())
            })
        })
    }

    pub fn image_dimensions(&self, config: &GalleryConfig) -> (u32, u32) {
        if let Some((width, height)) = self.exif.dimensions().filter(|_| !self.exif.oriented()) {
            // Avoid decoding the image if we don't have to.
            resize_dimensions(
                width,
                height,
                config.photo_resolution,
                config.photo_resolution,
            )
        } else {
            self.image(config).dimensions()
        }
    }

    pub fn image(&self, config: &GalleryConfig) -> &RgbImage {
        self.image.get_or_init(|| {
            let mut decoder = ImageReader::new(io::Cursor::new(&self.input_image_data))
                .with_guessed_format()
                .unwrap()
                .into_decoder()
                .unwrap();
            let orientation = decoder.orientation();
            let mut image = DynamicImage::from_decoder(decoder).unwrap();

            if let Ok(orientation) = orientation {
                image.apply_orientation(orientation);
            }

            resize_image(&image.to_rgb8(), config.photo_resolution)
        })
    }

    pub fn preview_dimensions(&self, config: &GalleryConfig) -> (u32, u32) {
        let (width, height) = self.image_dimensions(config);
        resize_dimensions(
            width,
            height,
            config.preview_resolution,
            config.preview_resolution,
        )
    }

    pub fn preview(&self, config: &GalleryConfig) -> &RgbImage {
        self.preview
            .get_or_init(|| resize_image(self.image(config), config.preview_resolution))
    }

    pub fn thumbnail(&self, config: &GalleryConfig) -> &RgbImage {
        self.thumbnail
            .get_or_init(|| generate_thumbnail(config, self.image(config), &self.config))
    }
}

impl Debug for Photo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Photo")
            .field("name", &self.name)
            .field("exif", &self.exif)
            .field("resolution", &self.exif.dimensions())
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

fn generate_thumbnail(
    config: &GalleryConfig,
    img: &RgbImage,
    photo_config: &PhotoConfig,
) -> RgbImage {
    let (width, height) = img.dimensions();
    let (size, x_offset, y_offset) = if false {
        let size = width.min(height);
        let x_offset = (width - size) / 2;
        let y_offset = (height - size) / 2;
        (size, x_offset, y_offset)
    } else {
        let min = width.min(height);
        let dim = min as f64 / photo_config.thumbnail_crop_factor.max(1.0);
        let x_center = width as f64 * photo_config.thumbnail_crop_center.x;
        let y_center = height as f64 * photo_config.thumbnail_crop_center.y;
        let x_offset = x_center - dim * 0.5;
        let y_offset = y_center - dim * 0.5;
        let size = dim.ceil() as u32;
        (
            size,
            (x_offset as u32).min(width - size),
            (y_offset as u32).min(height - size),
        )
    };

    let cropped = imageops::crop_imm(img, x_offset, y_offset, size, size);
    imageops::resize(
        &*cropped,
        config.thumbnail_resolution,
        config.thumbnail_resolution,
        FilterType::Lanczos3,
    )
}

fn resize_image(img: &RgbImage, resolution: u32) -> RgbImage {
    if img.width() <= resolution && img.height() <= resolution {
        return img.clone();
    }
    let (width, height) = resize_dimensions(img.width(), img.height(), resolution, resolution);
    imageops::resize(img, width, height, FilterType::Lanczos3)
}
