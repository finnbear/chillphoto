#![allow(unused)]

use category_path::CategoryPath;
use chrono::NaiveDate;
use config::Config;
use exif::ExifData;
use gallery::{Gallery, Item, Page, PageFormat};
use image::imageops::{self, resize, FilterType};
use image::{DynamicImage, GenericImageView, RgbImage};
use photo::Photo;
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use wax::Glob;

mod category_path;
mod config;
mod exif;
mod gallery;
mod output;
mod photo;
mod util;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config_text =
        fs::read_to_string("./chillphoto.toml").expect("couldn't read ./chillphoto.toml");
    toml::from_str::<Config>(&config_text).unwrap()
});

fn main() {
    let start = Instant::now();

    LazyLock::force(&CONFIG);
    let config = &*CONFIG;

    let mut input_path_string = config.input.clone();
    if let Some(remainder) = input_path_string.strip_prefix("~/") {
        #[allow(deprecated)]
        let home_dir = std::env::home_dir();
        input_path_string = format!(
            "{}/{remainder}",
            home_dir
                .unwrap()
                .to_str()
                .expect("invalid utf-8 in os-string")
        );
    }
    let (root, glob) = Glob::new(&input_path_string).unwrap().partition();

    let gallery = Mutex::new(Gallery {
        children: Vec::new(),
    });

    let mut entries = glob
        .walk(root)
        .not([config.output.as_str()])
        .unwrap()
        .collect::<Vec<_>>();

    entries.into_par_iter().for_each(|entry| {
        let entry = entry.unwrap();
        let name = entry.matched().complete().to_owned();
        let (categories, name) = if let Some((categories, name)) = name.rsplit_once('/') {
            (CategoryPath::new(categories), name)
        } else {
            (CategoryPath::ROOT, name.as_str())
        };

        let page_format = if name.ends_with(".html") {
            Some(PageFormat::Html)
        } else if name.ends_with(".md") {
            Some(PageFormat::Markdown)
        } else if name.ends_with(".txt") {
            Some(PageFormat::PlainText)
        } else {
            None
        };

        if let Some(format) = page_format {
            let file = std::fs::read_to_string(entry.path()).unwrap();
            let mut gallery = gallery.lock().unwrap();
            let mut to_insert = gallery.get_or_create_category(&categories);
            to_insert.push(Item::Page(Page {
                name: name.rsplit_once('.').unwrap().0.to_owned(),
                description: None,
                content: file,
                format,
            }));
            return;
        }

        let file = std::fs::read(entry.path()).unwrap();
        let img = image::load_from_memory(&file)
            .expect("failed to open image")
            .to_rgb8();

        let mut photo = Photo {
            name: name.rsplit_once('.').unwrap().0.to_owned(),
            description: None,
            thumbnail: generate_thumbnail(&img),
            preview: resize_image(&img, config.preview_resolution),
            image: resize_image(&img, config.photo_resolution),
            exif: ExifData::load(&file),
        };

        let mut gallery = gallery.lock().unwrap();
        let mut to_insert = gallery.get_or_create_category(&categories);
        to_insert.push(Item::Photo(photo));
    });

    let mut gallery = gallery.into_inner().unwrap();
    let mut categories = 0usize;
    let mut photos = 0usize;
    let mut pages = 0usize;
    gallery.visit_items_mut(|_, item| match item {
        Item::Category(category) => {
            let mut first_date = Option::<NaiveDate>::None;

            category.visit_items(&CategoryPath::ROOT, |_, child| {
                if let Item::Photo(photo) = child {
                    if let Some(date) = photo.exif.date() {
                        if first_date.map(|fd| date < fd).unwrap_or(true) {
                            first_date = Some(date);
                        }
                    }
                }
            });

            category.creation_date = first_date;

            categories += 1;
        }
        Item::Photo(_) => {
            photos += 1;
        }
        Item::Page(_) => {
            pages += 1;
        }
    });

    //println!("{gallery:?}");
    println!(
        "({:.1}s) Found {photos} photos in {categories} categories, and {pages} pages",
        start.elapsed().as_secs_f32()
    );

    gallery.output();

    println!(
        "({:.1}s) Saved website to {}",
        start.elapsed().as_secs_f32(),
        config.output
    );
}

fn is_text_file(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "txt"),
        None => false,
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
