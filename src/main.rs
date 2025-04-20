use category_path::CategoryPath;
use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use config::{CategoryConfig, GalleryConfig, PageConfig, PhotoConfig};
use exif::ExifData;
use gallery::{Gallery, Item, Page, RichText, RichTextFormat};
use image_ai::{image_ai, ImageAiPrompt};
use indicatif::{ProgressBar, ProgressStyle};
use photo::Photo;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serve::serve;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use toml_edit::DocumentMut;
use util::{checksum, remove_dir_contents};
use wax::Glob;

mod category_path;
mod config;
mod exif;
mod format;
mod gallery;
mod image_ai;
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
    /// Use an AI model (via `ollama`) to generate missing photo descriptions.
    ImageAi,
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
        .walk(&root)
        .not([config.output.as_str()])
        .unwrap()
        .collect::<Vec<_>>();

    let gallery = Mutex::new(GalleryExtras {
        gallery: Gallery {
            children: Vec::new(),
            favicon: None,
            config,
            head_html: None,
            home_text: None,
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
                text: RichText {
                    content: file,
                    format,
                },
                config: PageConfig::default(),
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
    let mut page_configs = 0usize;
    let mut match_pages = |mut home: Option<&mut Option<RichText>>, children: &mut Vec<Item>| {
        let mut matches = Vec::<(String, RichText)>::new();
        for name in children
            .iter()
            .filter_map(|i| {
                i.photo()
                    .map(|p| p.name.as_str())
                    .or(i.category().map(|c| c.name.as_str()))
            })
            .chain(std::iter::once("home"))
        {
            for page in children.iter().filter_map(|i| i.page()) {
                if *name == page.name {
                    matches.push((name.to_owned(), page.text.clone()));
                }
            }
        }
        for (name, text) in matches {
            if let Some(home) = &mut home {
                if name == "home" {
                    **home = Some(text.clone());
                }
            }
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
                    Item::Page(page) => {
                        return page.name != name && (home.is_none() || page.name != "home")
                    }
                }
                true
            });
        }
    };
    match_pages(Some(&mut gallery.home_text), &mut gallery.children);
    gallery.visit_items_mut(|path, item| match item {
        Item::Category(category) => {
            if let Some(config) = item_configs.remove(&path.push(category.slug())) {
                let config = toml::from_str::<CategoryConfig>(&config).unwrap();
                category.config = config;
                category_configs += 1;
            }

            match_pages(None, &mut category.children);

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
        Item::Page(page) => {
            let path = path.push(page.name.clone());
            if let Some(config) = item_configs.remove(&path) {
                let config = toml::from_str::<PageConfig>(&config).expect(&path.to_string());
                page.config = config;
                page_configs += 1;
            }
            pages += 1;
        }
    });

    #[derive(Eq, PartialEq, Ord, PartialOrd)]
    enum Order {
        Category {
            order: Reverse<i64>,
            name: String,
        },
        Photo {
            order: Reverse<i64>,
            date: Reverse<Option<NaiveDateTime>>,
            name: String,
        },
        Page {
            order: Reverse<i64>,
            name: String,
        },
    }

    impl Order {
        fn new(item: &Item) -> Self {
            match item {
                Item::Category(category) => Order::Category {
                    order: Reverse(category.config.order),
                    name: category.name.clone(),
                },
                Item::Photo(photo) => Order::Photo {
                    order: Reverse(photo.config.order),
                    date: Reverse(photo.date_time()),
                    name: photo.name.clone(),
                },
                Item::Page(page) => Order::Page {
                    order: Reverse(page.config.order),
                    name: page.name.clone(),
                },
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
        "({:.1}s) Found {photos} photos ({photo_configs} with config, {photo_texts} with caption) in {categories} categories ({category_configs} with config, {category_texts} with caption), and {pages} pages ({page_configs} with config)",
        start.elapsed().as_secs_f32()
    );

    if matches!(args.command, Command::ImageAi) {
        gallery.visit_items(|path, item| {
            if let Some(photo) = item.photo() {
                let mut prompt = String::new();
                if path.is_root() {
                    writeln!(
                        prompt,
                        "The photo is in the gallery: {}.",
                        gallery.config.title
                    )
                    .unwrap();
                    if let Some(description) = &gallery.config.description {
                        writeln!(prompt, "The gallery description is: {description}.",).unwrap();
                    }
                } else {
                    let category = gallery.category(path).unwrap();
                    writeln!(prompt, "The photo is in the category: {}.", category.name).unwrap();
                    if let Some(description) = &category.config.description {
                        writeln!(prompt, "The category description is: {description}.",).unwrap();
                    }
                    if let Some(hint) = &category.config.ai_description_hint {
                        writeln!(
                            prompt,
                            "A hint for the entire category been provided: {hint}."
                        )
                        .unwrap();
                    }
                }
                if let Some(hint) = &photo.config.ai_description_hint {
                    writeln!(prompt, "A hint has been provided: {hint}.").unwrap();
                }

                writeln!(prompt, "Describe the photo.").unwrap();

                let prompt = ImageAiPrompt {
                    prompt: &prompt,
                    photo,
                    config: &gallery.config,
                };

                if let Some(description) = photo.config.description.as_ref() {
                    if photo.config.ai_description_output_checksum
                        != Some(checksum(description.as_bytes()))
                    {
                        println!("keeping manual description for {}", photo.name);
                        return;
                    }
                }

                let input_checksum = prompt.checksum();
                if let Some(sum) = &photo.config.ai_description_input_checksum {
                    if input_checksum == *sum {
                        println!("keeping existing ai description for {}", photo.name);
                        return;
                    } else {
                        println!("regenerating ai description for {}", photo.name);
                    }
                }

                let summary = image_ai(prompt);

                let mut config_path = root.clone();
                for path in path.iter_paths().skip(1) {
                    config_path.push(&gallery.category(&path).unwrap().name);
                }
                config_path.push(format!("{}.toml", photo.name));

                let mut file = fs::OpenOptions::new()
                    .read(true)
                    .create(true)
                    .write(true)
                    .open(&config_path)
                    .unwrap();
                file.seek(std::io::SeekFrom::Start(0)).unwrap();
                let mut existing = String::new();
                file.read_to_string(&mut existing).unwrap();
                let mut doc = existing.parse::<DocumentMut>().unwrap();
                doc["description"] = toml_edit::value(summary.clone());
                doc["ai_description_input_checksum"] = toml_edit::value(input_checksum);
                doc["ai_description_output_checksum"] =
                    toml_edit::value(checksum(&summary.as_bytes()));
                file.seek(SeekFrom::Start(0)).unwrap();
                file.set_len(0).unwrap();
                file.write_all(doc.to_string().as_bytes()).unwrap();
                file.sync_data().unwrap();

                println!("summarized {}: {summary}", photo.name);
            }
        });
        return;
    }

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
