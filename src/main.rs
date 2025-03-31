use category_path::CategoryPath;
use chrono::NaiveDate;
use clap::Parser;
use config::{Config, PhotoConfig};
use exif::ExifData;
use gallery::{Gallery, Item, Page, RichText, RichTextFormat};
use photo::Photo;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serve::serve;
use std::collections::HashMap;
use std::fs;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use util::remove_dir_contents;
use wax::Glob;

mod category_path;
mod config;
mod exif;
mod gallery;
mod output;
mod photo;
mod serve;
mod util;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config_text =
        fs::read_to_string("./chillphoto.toml").expect("couldn't read ./chillphoto.toml");
    toml::from_str::<Config>(&config_text).unwrap()
});

#[derive(Parser)]
struct Args {
    #[clap(long)]
    serve: bool,
}

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

    struct GalleryExtras {
        gallery: Gallery,
        photo_configs: HashMap<CategoryPath, PhotoConfig>,
    }

    let gallery = Mutex::new(GalleryExtras {
        gallery: Gallery {
            children: Vec::new(),
        },
        photo_configs: HashMap::new(),
    });

    let entries = glob
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

        let name_no_extension = name.rsplit_once('.').unwrap().0.to_owned();

        if name.ends_with(".toml") {
            let config_text = fs::read_to_string(entry.path()).expect("couldn't read photo config");
            let photo_config = toml::from_str::<PhotoConfig>(&config_text).unwrap();

            let mut gallery = gallery.lock().unwrap();
            gallery
                .photo_configs
                .insert(categories.push(name_no_extension), photo_config);
            return;
        }

        let page_format = if name.ends_with(".html") {
            Some(RichTextFormat::Html)
        } else if name.ends_with(".md") {
            Some(RichTextFormat::Markdown)
        } else if name.ends_with(".txt") {
            Some(RichTextFormat::PlainText)
        } else {
            None
        };

        if let Some(format) = page_format {
            let file = std::fs::read_to_string(entry.path()).unwrap();
            let mut gallery = gallery.lock().unwrap();
            let to_insert = gallery.gallery.get_or_create_category(&categories);
            to_insert.push(Item::Page(Page {
                name: name_no_extension,
                description: None,
                text: RichText {
                    content: file,
                    format,
                },
            }));
            return;
        }

        let input_image_data = std::fs::read(entry.path()).unwrap();

        let photo = Photo {
            name: name_no_extension,
            text: None,
            exif: ExifData::load(&input_image_data),
            input_image_data,
            image: Default::default(),
            preview: Default::default(),
            thumbnail: Default::default(),
            config: Default::default(),
        };

        let mut gallery = gallery.lock().unwrap();
        let to_insert = gallery.gallery.get_or_create_category(&categories);
        to_insert.push(Item::Photo(photo));
    });

    let GalleryExtras {
        mut gallery,
        mut photo_configs,
    } = gallery.into_inner().unwrap();
    let mut categories = 0usize;
    let mut photos = 0usize;
    let mut pages = 0usize;
    gallery.visit_items_mut(|path, item| match item {
        Item::Category(category) => {
            let mut matches = Vec::<(String, RichText)>::new();
            for photo in category.children.iter().filter_map(|i| i.photo()) {
                for page in category.children.iter().filter_map(|i| i.page()) {
                    if photo.name == page.name {
                        matches.push((photo.name.clone(), page.text.clone()));
                    }
                }
            }
            for (name, text) in matches {
                category.children.retain_mut(|child| {
                    match child {
                        Item::Photo(photo) => {
                            if photo.name == name {
                                photo.text = Some(text.clone());
                            }
                        }
                        Item::Page(page) => return page.name != name,
                        _ => {}
                    }
                    true
                });
            }

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
        Item::Photo(photo) => {
            if let Some(config) = photo_configs.remove(&path.push(photo.name.clone())) {
                photo.config = config;
            }
            photos += 1;
        }
        Item::Page(_) => {
            pages += 1;
        }
    });

    gallery.visit_items_mut(|_, item| match item {
        Item::Category(category) => {
            category.children.sort_by_key(|item| {
                std::cmp::Reverse(if let Some(photo) = item.photo() {
                    (photo.config.order, photo.exif.date())
                } else {
                    (0, None)
                })
            });
        }
        _ => {}
    });

    //println!("{gallery:?}");
    println!(
        "({:.1}s) Found {photos} photos in {categories} categories, and {pages} pages",
        start.elapsed().as_secs_f32()
    );

    let output = gallery.output();

    println!(
        "({:.1}s) Generated output manifest",
        start.elapsed().as_secs_f32(),
    );

    if Args::parse().serve {
        serve(start, &output);
    } else {
        if fs::exists(&config.output).unwrap() {
            remove_dir_contents(&config.output).expect("failed to clear output directory");
        }

        output.par_iter().for_each(|(path, generator)| {
            let path = config.subdirectory(path.strip_prefix('/').unwrap());
            let contents = &**generator;
            if let Some((dir, _)) = path.rsplit_once('/') {
                std::fs::create_dir_all(dir).unwrap();
            }
            std::fs::write(path, contents).unwrap();
        });

        println!(
            "({:.1}s) Saved website to {}",
            start.elapsed().as_secs_f32(),
            config.output
        );
    }
}
