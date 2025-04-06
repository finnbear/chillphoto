use category_path::CategoryPath;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use config::{CategoryConfig, GalleryConfig, PhotoConfig};
use exif::ExifData;
use gallery::{Gallery, Item, Page, RichText, RichTextFormat};
use indicatif::{ProgressBar, ProgressStyle};
use photo::Photo;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serve::serve;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use util::remove_dir_contents;
use wax::Glob;

mod category_path;
mod config;
mod exif;
mod format;
mod gallery;
mod output;
mod photo;
mod serve;
mod util;

#[derive(Parser)]
#[clap(name = "chillphoto", version)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create default top-level configuration file.
    Init,
    /// Serve gallery preview.
    Serve,
    /// Build static gallery website.
    Build,
}

fn main() {
    let start = Instant::now();

    let args = Args::parse();

    if matches!(args.command, Command::Init) {
        let config = toml::from_str::<GalleryConfig>("").unwrap();
        let string = toml::to_string(&config).unwrap();
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open("./chillphoto.toml")
            .unwrap();
        file.write_all(string.as_bytes()).unwrap();
        file.flush().unwrap();
        file.sync_all().unwrap();
        return;
    }

    let config_text =
        fs::read_to_string("./chillphoto.toml").expect("couldn't read ./chillphoto.toml");
    let config = toml::from_str::<GalleryConfig>(&config_text).unwrap();

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
        item_configs: HashMap<CategoryPath, String>,
    }

    let entries = glob
        .walk(root)
        .not([config.output.as_str()])
        .unwrap()
        .collect::<Vec<_>>();

    let gallery = Mutex::new(GalleryExtras {
        gallery: Gallery {
            children: Vec::new(),
            favicon: None,
            config,
            head_html: None,
        },
        item_configs: HashMap::new(),
    });

    entries.into_par_iter().for_each(|entry| {
        let entry = entry.unwrap();
        let name = entry.matched().complete().to_owned();
        let (category_names, categories, name) =
            if let Some((categories, name)) = name.rsplit_once('/') {
                let category_names = categories
                    .split('/')
                    .map(|s| s.to_owned())
                    .collect::<Vec<String>>();
                let path = CategoryPath::new(
                    &category_names
                        .iter()
                        .map(|n| n.replace(' ', "-"))
                        .collect::<Vec<_>>()
                        .join("/"),
                );
                (category_names, path, name)
            } else {
                (Vec::new(), CategoryPath::ROOT, name.as_str())
            };

        let name_no_extension = name.rsplit_once('.').unwrap().0.to_owned();

        if name.ends_with(".toml") {
            let config_text = fs::read_to_string(entry.path()).expect("couldn't read photo config");

            let mut gallery = gallery.lock().unwrap();
            gallery.item_configs.insert(
                categories.push(name_no_extension.replace(' ', "-")),
                config_text,
            );
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
            if name_no_extension == "head" && matches!(format, RichTextFormat::Html) {
                gallery.gallery.head_html = Some(file);
                return;
            }
            let to_insert = gallery
                .gallery
                .get_or_create_category(&category_names, &categories);
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

        let metadata = fs::metadata(entry.path()).unwrap();
        let input_image_data = fs::read(entry.path()).unwrap();

        let mut gallery = gallery.lock().unwrap();
        if categories.is_root() && name_no_extension == "favicon" {
            gallery.gallery.favicon = Some((input_image_data, OnceLock::new()));
            return;
        }

        let photo = Photo {
            name: name_no_extension,
            text: None,
            exif: ExifData::load(&input_image_data),
            input_image_data,
            image: Default::default(),
            preview: Default::default(),
            thumbnail: Default::default(),
            config: Default::default(),
            file_date: metadata.modified().or(metadata.created()).ok(),
        };

        let to_insert = gallery
            .gallery
            .get_or_create_category(&category_names, &categories);
        to_insert.push(Item::Photo(photo));
    });

    let GalleryExtras {
        mut gallery,
        mut item_configs,
    } = gallery.into_inner().unwrap();
    let mut categories = 0usize;
    let mut category_configs = 0usize;
    let mut category_texts = 0usize;
    let mut photos = 0usize;
    let mut photo_configs = 0usize;
    let mut photo_texts = 0usize;
    let mut pages = 0usize;
    let mut match_pages = |children: &mut Vec<Item>| {
        let mut matches = Vec::<(String, RichText)>::new();
        for name in children
            .iter()
            .filter_map(|i| i.photo().map(|p| &p.name).or(i.category().map(|c| &c.name)))
        {
            for page in children.iter().filter_map(|i| i.page()) {
                if *name == page.name {
                    matches.push((name.clone(), page.text.clone()));
                }
            }
        }
        for (name, text) in matches {
            children.retain_mut(|child| {
                match child {
                    Item::Photo(photo) => {
                        if photo.name == name {
                            photo.text = Some(text.clone());
                            photo_texts += 1;
                        }
                    }
                    Item::Category(category) => {
                        if category.name == name {
                            category.text = Some(text.clone());
                            category_texts += 1;
                        }
                    }
                    Item::Page(page) => return page.name != name,
                }
                true
            });
        }
    };
    match_pages(&mut gallery.children);
    gallery.visit_items_mut(|path, item| match item {
        Item::Category(category) => {
            if let Some(config) = item_configs.remove(&path.push(category.slug())) {
                let config = toml::from_str::<CategoryConfig>(&config).unwrap();
                category.config = config;
                category_configs += 1;
            }

            match_pages(&mut category.children);

            let mut first_date = Option::<NaiveDate>::None;

            category.visit_items(&CategoryPath::ROOT, |_, child| {
                if let Item::Photo(photo) = child {
                    if let Some(date) = photo.date() {
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
            let path = path.push(photo.name.clone());
            if let Some(config) = item_configs.remove(&path) {
                let config = toml::from_str::<PhotoConfig>(&config).expect(&path.to_string());
                photo.config = config;
                photo_configs += 1;
            }
            photos += 1;
        }
        Item::Page(_) => {
            pages += 1;
        }
    });

    #[derive(Eq, PartialEq, Ord, PartialOrd)]
    enum Order {
        Category {
            order: Reverse<i64>,
        },
        Photo {
            order: Reverse<i64>,
            date: Reverse<Option<NaiveDate>>,
        },
        Irrelevant,
    }

    impl Order {
        fn new(item: &Item) -> Self {
            match item {
                Item::Category(category) => Order::Category {
                    order: Reverse(category.config.order),
                },
                Item::Photo(photo) => Order::Photo {
                    order: Reverse(photo.config.order),
                    date: Reverse(photo.exif.date()),
                },
                _ => Order::Irrelevant,
            }
        }
    }

    gallery.children.sort_by_key(Order::new);
    gallery.visit_items_mut(|_, item| match item {
        Item::Category(category) => {
            category.children.sort_by_key(Order::new);
        }
        _ => {}
    });

    //println!("{gallery:?}");
    println!(
        "({:.1}s) Found {photos} photos ({photo_configs} with config, {photo_texts} with caption) in {categories} categories ({category_configs} with config, {category_texts} with caption), and {pages} pages",
        start.elapsed().as_secs_f32()
    );

    let output = gallery.output();

    println!(
        "({:.1}s) Generated output manifest",
        start.elapsed().as_secs_f32(),
    );

    if matches!(args.command, Command::Serve) {
        serve(start, &output);
    } else {
        let progress = ProgressBar::new(output.len() as u64)
            .with_message("Saving website...")
            .with_style(
                ProgressStyle::default_bar()
                    .template("{msg} {wide_bar} {pos}/{len} {eta}")
                    .unwrap(),
            )
            .with_elapsed(start.elapsed());

        if fs::exists(&gallery.config.output).unwrap() {
            remove_dir_contents(&gallery.config.output).expect("failed to clear output directory");
        }

        output.par_iter().for_each(|(path, generator)| {
            let path = gallery.config.subdirectory(path.strip_prefix('/').unwrap());
            let contents = &**generator;
            if let Some((dir, _)) = path.rsplit_once('/') {
                std::fs::create_dir_all(dir).unwrap();
            }
            std::fs::write(path, contents).unwrap();
            progress.inc(1);
        });

        progress.finish_and_clear();

        println!(
            "({:.1}s) Saved website to {}",
            start.elapsed().as_secs_f32(),
            gallery.config.output
        );
    }
}
