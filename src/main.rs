#![allow(unused)]

use config::{Config, ThumbnailConfig};
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
use std::sync::Mutex;
use std::time::Instant;
use wax::Glob;

mod config;
mod exif;
mod gallery;
mod output;
mod photo;
mod util;

fn main() {
    let start = Instant::now();
    let config_text =
        fs::read_to_string("./chillphoto.toml").expect("couldn't read ./chillphoto.toml");
    let config = toml::from_str::<Config>(&config_text).unwrap();
    let mut input_path_string = config.input.path.clone();
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
        .not([config.output.path.as_str()])
        .unwrap()
        .collect::<Vec<_>>();

    entries.into_par_iter().for_each(|entry| {
        let entry = entry.unwrap();
        let name = entry.matched().complete().to_owned();
        let (categories, name) = if let Some((categories, name)) = name.rsplit_once('/') {
            (categories.split('/').collect::<Vec<_>>(), name)
        } else {
            (Vec::new(), name.as_str())
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
            name: name.to_owned(),
            thumbnail: generate_thumbnail(&config.thumbnail, &img),
            preview: generate_preview(&img),
            image: img,
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
        Item::Category(_) => {
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

    gallery.output(&config);

    println!(
        "({:.1}s) Saved website to {}",
        start.elapsed().as_secs_f32(),
        config.output.path
    );
}

fn is_text_file(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "txt"),
        None => false,
    }
}

fn generate_thumbnail(config: &ThumbnailConfig, img: &RgbImage) -> RgbImage {
    let (width, height) = img.dimensions();
    let size = width.min(height);
    let x_offset = (width - size) / 2;
    let y_offset = (height - size) / 2;
    let cropped = imageops::crop_imm(img, x_offset, y_offset, size, size).to_image();
    imageops::resize(
        &cropped,
        config.resolution,
        config.resolution,
        FilterType::Lanczos3,
    )
}

fn generate_preview(img: &RgbImage) -> RgbImage {
    DynamicImage::ImageRgb8(img.clone())
        .resize(1920, 1080, FilterType::Lanczos3)
        .to_rgb8()
}
