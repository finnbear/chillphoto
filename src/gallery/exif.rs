use chrono::NaiveDateTime;
use exif::{Exif, In, Tag};

/// https://www.cipa.jp/std/documents/e/DC-008-2012_E.pdf
#[derive(Debug, Default, Clone, PartialEq)]
#[allow(unused)]
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
    // TODO: REI = ISO?
    pub iso_sensitivity: Option<String>,
    pub exposure_compensation: Option<String>,
    pub focal_length: Option<String>,
    pub metering_mode: Option<String>,
    pub flash: Option<String>,
    pub orientation: Option<String>,
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

    pub fn date_time(&self) -> Option<NaiveDateTime> {
        self.original_time_taken
            .as_ref()
            .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
    }

    pub fn oriented(&self) -> bool {
        self.orientation
            .as_ref()
            .is_some_and(|o| o.as_str() != "row 0 at top and column 0 at left")
    }

    pub fn new(exif: Vec<u8>) -> Self {
        let exifreader = exif::Reader::new();
        let meta = exifreader.read_raw(exif).ok();

        // Debug.
        // meta.as_ref().is_some_and(|m| m.get_field(Tag::Make, In::PRIMARY).unwrap().display_value().to_string().contains("Canon"))
        if false {
            if let Some(meta) = &meta {
                for f in meta.fields() {
                    println!(
                        "{} {} {}",
                        f.tag,
                        f.ifd_num,
                        f.display_value().with_unit(meta)
                    );
                }
            }
        }

        fn lookup(meta: &Option<Exif>, tag: Tag) -> Option<String> {
            let meta = meta.as_ref()?;
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
            aperture: lookup(&meta, Tag::FNumber),
            iso_sensitivity: lookup(&meta, Tag::PhotographicSensitivity),
            exposure_compensation: lookup(&meta, Tag::ExposureBiasValue),
            focal_length: lookup(&meta, Tag::FocalLength),
            metering_mode: lookup(&meta, Tag::MeteringMode),
            flash: lookup(&meta, Tag::Flash),
            orientation: lookup(&meta, Tag::Orientation),
        }
    }
}
