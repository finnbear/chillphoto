use chrono::NaiveDate;
use exif::{Exif, In, Tag};
use std::io::Cursor;

/// https://www.cipa.jp/std/documents/e/DC-008-2012_E.pdf
#[derive(Debug, Clone, PartialEq)]
pub struct ExifData {
    pub width: Option<String>,
    pub height: Option<String>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_make: Option<String>,
    pub lens_model: Option<String>,
    /// YYYY-MM-DD HH:MM:SS
    pub original_time_taken: Option<String>,
    /// Seconds.
    pub exposure_time: Option<String>,
    pub aperture: Option<String>,
    // TODO: units
    pub iso_sensitivity: Option<String>,
    pub exposure_compensation: Option<String>,
    pub focal_length: Option<String>,
    pub metering_mode: Option<String>,
    pub flash: Option<String>,
}

impl ExifData {
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        fn parse_pixels(s: &str) -> Option<u32> {
            s.split(' ').next().and_then(|s| s.parse::<u32>().ok())
        }
        self.width
            .as_deref()
            .zip(self.height.as_deref())
            .and_then(|(w, h)| parse_pixels(w).zip(parse_pixels(h)))
    }

    pub fn date(&self) -> Option<NaiveDate> {
        self.original_time_taken
            .as_ref()
            .and_then(|s| NaiveDate::parse_and_remainder(s, "%Y-%m-%d").ok())
            .map(|(d, _)| d)
    }

    pub fn load(file: &[u8]) -> Self {
        let exifreader = exif::Reader::new();
        let meta = exifreader
            .read_from_container(&mut Cursor::new(file))
            .unwrap();

        /*
        for f in meta.fields() {
            println!(
                "{} {} {}",
                f.tag,
                f.ifd_num,
                f.display_value().with_unit(&meta)
            );
        }
        */

        fn lookup(meta: &Exif, tag: Tag) -> Option<String> {
            let field = meta.get_field(tag, In::PRIMARY)?;
            // TODO: rounding.
            Some(field.display_value().with_unit(meta).to_string())
        }

        Self {
            width: lookup(&meta, Tag::PixelXDimension).or_else(|| lookup(&meta, Tag::ImageWidth)),
            height: lookup(&meta, Tag::PixelYDimension).or_else(|| lookup(&meta, Tag::ImageLength)),
            camera_make: lookup(&meta, Tag::Make),
            camera_model: lookup(&meta, Tag::Model),
            lens_make: lookup(&meta, Tag::LensMake),
            lens_model: lookup(&meta, Tag::LensModel),
            original_time_taken: lookup(&meta, Tag::DateTimeOriginal),
            exposure_time: lookup(&meta, Tag::ExposureTime),
            aperture: lookup(&meta, Tag::ApertureValue),
            iso_sensitivity: lookup(&meta, Tag::PhotographicSensitivity),
            exposure_compensation: lookup(&meta, Tag::ExposureBiasValue),
            focal_length: lookup(&meta, Tag::FocalLength),
            metering_mode: lookup(&meta, Tag::MeteringMode),
            flash: lookup(&meta, Tag::Flash),
        }
    }
}
