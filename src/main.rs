use crate::gallery::Order;
use crate::image_ai::init_image_ai;
use crate::util::recursively_remove_empty_dirs_of_contents;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use gallery::CategoryPath;
use gallery::ExifData;
use gallery::Photo;
use gallery::StaticFile;
use gallery::{CategoryConfig, GalleryConfig, PageConfig, PhotoConfig};
use gallery::{Gallery, Item, Page, RichText, RichTextFormat};
use indicatif::{ProgressBar, ProgressStyle};
use output::serve;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use toml_edit::DocumentMut;
use util::remove_dir_contents;
use wax::Glob;

mod copyright_registration;
mod gallery;
mod image_ai;
mod output;
mod util;

#[derive(Parser)]
#[clap(name = "chillphoto", version)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create configuration files, starting with
    /// the top-level config file.
    Init {
        /// Generate config files for all photos.
        #[arg(long)]
        photos: bool,
        /// Use an AI model (via `ollama`) to generate missing photo descriptions.
        #[arg(long)]
        image_ai: bool,
    },
    /// Output a year's worth of photos from a single author
    /// in a format suitable (not legal advice) for the US
    /// Copyright Office.
    ///
    /// WARNING:
    /// This makes several assumptions that, if incorrect,
    /// could invalidate your registration and/or create
    /// civil and/or criminal liability. Ensure you absolutely
    /// know what you are doing and check the entire output
    /// before using it.
    Copyright {
        /// Bulk registration must be for a single year.
        ///
        /// This will be used to filter photos. Photos without
        /// a known date will be skipped no matter what the
        /// year is.
        #[arg(long)]
        year: u32,
        /// Bulk registration must be for a single author.
        ///
        /// This will be used to filter photos.
        #[arg(long)]
        author: String,
        /// Get this from the Copyright Office website.
        #[arg(long)]
        case_number: String,
        /// Start on this photo (name or filename).
        #[arg(long)]
        start: Option<String>,
        /// Limit to this many photos. This is useful for staying
        /// under the Copyright Office's limits, such as a 500MB
        /// upload limit.
        #[arg(long, default_value_t = 750)]
        limit: usize,
    },
    /// Serve gallery preview.
    Serve {
        /// In between HTTP requests, build a cache of image
        /// assets in the background. This is RAM-prohibitive
        /// for large galleries but can reduce latency when
        /// browsing small galleries. A better alternative is
        /// using a web browser that supports Speculation Rules.
        #[arg(long)]
        background: bool,
    },
    /// Build static gallery website.
    Build,
    /// Clear the output directory.
    Clean,
}

fn main() {
    let start = Instant::now();
    let args = Args::parse();

    if matches!(args.command, Command::Init { .. }) {
        if let Some(mut file) = match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open("./chillphoto.toml")
        {
            Ok(file) => Some(file),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => None,
            Err(e) => {
                panic!("{}", e);
            }
        } {
            let config = toml::from_str::<GalleryConfig>("").unwrap();
            let string = toml::to_string(&config).unwrap();
            file.write_all(string.as_bytes()).unwrap();
            file.flush().unwrap();
            file.sync_all().unwrap();
        }
    }

    let config_text =
        fs::read_to_string("./chillphoto.toml").expect("couldn't read ./chillphoto.toml");
    let config = toml::from_str::<GalleryConfig>(&config_text).unwrap();

    if matches!(args.command, Command::Clean) {
        if fs::exists(&config.output).unwrap() {
            remove_dir_contents(&config.output).expect("failed to clear output directory");
        }
        return;
    }

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
        item_configs: HashMap<String, String>,
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
            static_files: Vec::new(),
        },
        item_configs: HashMap::new(),
    });

    entries.into_par_iter().for_each(|entry| {
        let entry = entry.unwrap();
        if !entry.file_type().is_file() {
            return;
        }
        let entire_path = entry.matched().complete().to_owned();
        let (category_names, categories, except_name, name) =
            if let Some((categories, name)) = entire_path.rsplit_once('/') {
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
                (category_names, path, categories, name)
            } else {
                (Vec::new(), CategoryPath::ROOT, "", entire_path.as_str())
            };

        if !category_names.is_empty() && category_names[0] == "static" {
            let mut gallery = gallery.lock().unwrap();
            gallery.gallery.static_files.push(StaticFile {
                path: format!("/{entire_path}"),
                contents: fs::read(entry.path()).unwrap(),
            });
            return;
        }

        let name_no_extension = name.rsplit_once('.').unwrap().0.to_owned();
        let path_no_extension = if except_name.is_empty() {
            name_no_extension.clone()
        } else {
            format!("{except_name}/{name_no_extension}")
        };

        if name.ends_with(".toml") {
            let config_text = fs::read_to_string(entry.path()).expect("couldn't read photo config");

            let mut gallery = gallery.lock().unwrap();
            gallery.item_configs.insert(path_no_extension, config_text);
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
                src_key: path_no_extension,
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
            src_key: path_no_extension,
            parsed_config_date: None,
            distinct_name: None,
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
    let date_format = gallery.config.date_format.clone();
    gallery.visit_items_mut(|path, item| match item {
        Item::Category(category) => {
            if let Some(config) = item_configs.remove(&category.src_key) {
                let config = toml::from_str::<CategoryConfig>(&config).unwrap();
                category.config = config;
                category_configs += 1;
            }

            match_pages(None, &mut category.children);

            categories += 1;
        }
        Item::Photo(photo) => {
            if let Some(config) = item_configs.remove(&photo.src_key) {
                let config = toml::from_str::<PhotoConfig>(&config).expect(&path.to_string());
                photo.config = config;
                if let Some(date) = &photo.config.date {
                    photo.parsed_config_date =
                        Some(NaiveDate::parse_from_str(date, &date_format).unwrap());
                }
                photo_configs += 1;
            }
            photos += 1;
        }
        Item::Page(page) => {
            if let Some(config) = item_configs.remove(&page.src_key) {
                let config = toml::from_str::<PageConfig>(&config).expect(&path.to_string());
                page.config = config;
                page_configs += 1;
            }
            pages += 1;
        }
    });

    fn sort_and_make_photo_names_distinct(items: &mut [Item]) {
        // Don't let user-defined order change distinct names.
        items.sort_by_key(|item| {
            if let Item::Photo(photo) = item {
                Some((std::cmp::Reverse(photo.date_time()), photo.name.clone()))
            } else {
                None
            }
        });

        let mut indices = HashMap::<String, usize>::new();
        for item in items.iter_mut().rev() {
            let photo = if let Item::Photo(photo) = item {
                photo
            } else {
                continue;
            };

            let name = photo.output_name();
            let index = indices.entry(name.to_owned()).or_default();
            *index += 1;
            if *index > 1 {
                photo.distinct_name = Some(format!("{name} {index}"));
            }
        }

        items.sort_by_key(Order::new);
    }

    sort_and_make_photo_names_distinct(&mut gallery.children);
    gallery.visit_items_mut(|_, item| match item {
        Item::Category(category) => {
            sort_and_make_photo_names_distinct(&mut category.children);
        }
        _ => {}
    });

    //println!("{gallery:?}");
    println!(
        "({:.1}s) Found {photos} photos ({photo_configs} with config, {photo_texts} with caption) in {categories} categories ({category_configs} with config, {category_texts} with caption), and {pages} pages ({page_configs} with config)",
        start.elapsed().as_secs_f32()
    );

    if let Command::Init { photos, image_ai } = &args.command {
        let mut jobs = Vec::new();
        gallery.visit_items(|path, item| {
            if let Some(photo) = item.photo() {
                if !*photos && !*image_ai {
                    return;
                }
                jobs.push((path.to_owned(), photo));
            }
        });

        let each = |(path, photo): (CategoryPath, &Photo)| {
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

            if *image_ai {
                init_image_ai(&gallery, &path, photo, &mut doc);
            }

            file.seek(SeekFrom::Start(0)).unwrap();
            file.set_len(0).unwrap();
            file.write_all(doc.to_string().as_bytes()).unwrap();
            file.sync_data().unwrap();
        };
        if gallery.config.image_ai_api_key.is_some() {
            jobs.into_par_iter().for_each(each);
        } else {
            // AI is local, so avoid exausting local resources.
            jobs.into_iter().for_each(each);
        }
        return;
    }

    if let Command::Copyright {
        year,
        author,
        case_number,
        start,
        limit,
    } = &args.command
    {
        gallery.copyright(*year, author, case_number, start.as_deref(), *limit);
        return;
    }

    let output = gallery.output();

    println!(
        "({:.1}s) Generated output manifest",
        start.elapsed().as_secs_f32(),
    );

    if let Command::Serve { background } = &args.command {
        serve(start, *background, &output);
    } else {
        let progress = ProgressBar::new(output.len() as u64)
            .with_message("Saving website...")
            .with_style(
                ProgressStyle::default_bar()
                    .template("{msg} {wide_bar} {pos}/{len} {eta}")
                    .unwrap(),
            )
            .with_elapsed(start.elapsed());

        let reused = AtomicUsize::new(0);
        let total = AtomicUsize::new(0);
        let mut removals = 0usize;

        // Remove obsolete files.
        for file in Glob::new("**").unwrap().walk(&gallery.config.output) {
            let file = file.unwrap();
            if !file.file_type().is_file() {
                continue;
            }
            let path = format!("/{}", file.matched().complete());
            if output.get(&path).is_none() {
                //println!("removing obsolete {path}");
                fs::remove_file(file.path()).unwrap();
                removals += 1;
            }
        }
        recursively_remove_empty_dirs_of_contents(&gallery.config.output).unwrap();

        output.par_iter().for_each(|(path, (generator, hasher))| {
            let path = gallery.config.subdirectory(path.strip_prefix('/').unwrap());

            let new_hash = hasher.as_ref().map(|hasher| (&**hasher).as_bytes());

            let mut reuse = false;
            if let Some(new_hash) = new_hash {
                if fs::exists(&path).unwrap() {
                    if let Ok(Some(old_hash)) = fsquirrel::get(&path, "chillphotohash") {
                        if old_hash == new_hash {
                            reuse = true;
                        }
                    }
                }

                total.fetch_add(1, Ordering::Relaxed);
                if reuse {
                    reused.fetch_add(1, Ordering::Relaxed);
                }
            }

            if !reuse {
                let contents = &**generator;
                if let Some((dir, _)) = path.rsplit_once('/') {
                    std::fs::create_dir_all(dir).unwrap();
                }
                let _ = fsquirrel::remove(&path, "chillphotohash");
                std::fs::write(&path, contents).unwrap();
                if let Some(new_hash) = new_hash {
                    fsquirrel::set(&path, "chillphotohash", new_hash).unwrap();
                }
            }

            progress.inc(1);
        });

        progress.finish_and_clear();

        println!(
            "({:.1}s) Saved website to {}, reusing {}/{} images files, removed {removals} obsolete files",
            start.elapsed().as_secs_f32(),
            gallery.config.output,
            reused.load(Ordering::Relaxed),
            total.load(Ordering::Relaxed),
        );
    }
}
