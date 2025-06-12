use crate::image_ai::init_image_ai;
use crate::output::Order;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use clap::{Parser, Subcommand};
use copyright_registration::MarginFrameCellDecorator;
use gallery::CategoryPath;
use gallery::ExifData;
use gallery::Photo;
use gallery::StaticFile;
use gallery::{CategoryConfig, GalleryConfig, PageConfig, PhotoConfig};
use gallery::{Gallery, Item, Page, RichText, RichTextFormat};
use genpdfi::fonts::{FontData, FontFamily};
use genpdfi::style::{Style, StyledString};
use genpdfi::{Margins, SimplePageDecorator};
use indicatif::{ProgressBar, ProgressStyle};
use output::serve;
use output::write_image;
use output::OutputFormat;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, ErrorKind, Read, Seek, SeekFrom, Write};
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
    Serve,
    /// Build static gallery website.
    Build,
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

    if let Command::Init { photos, image_ai } = &args.command {
        gallery.visit_items(|path, item| {
            if let Some(photo) = item.photo() {
                if !*photos && !*image_ai {
                    return;
                }

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
            }
        });
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
        // Copyright office doesn't support WebP.
        if gallery.config.photo_format == OutputFormat::WebP {
            gallery.config.photo_format = OutputFormat::Png;
        }

        struct PhotoCopyrightSubmission<'a> {
            filename: String,
            title: String,
            date_time: NaiveDateTime,
            photo: &'a Photo,
        }

        use zip::write::{SimpleFileOptions, ZipWriter};
        let mut archive = ZipWriter::new(Cursor::new(Vec::<u8>::new()));
        let mut manifest = Vec::<PhotoCopyrightSubmission>::new();
        let zip_options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let mut earliest_date = Option::<NaiveDate>::None;
        let mut latest_date = Option::<NaiveDate>::None;

        gallery.visit_items(|_, item| {
            let photo = if let Item::Photo(photo) = item {
                photo
            } else {
                return;
            };

            let date_time = if let Some(date) = photo.date_time() {
                date
            } else {
                return;
            };

            let date = date_time.date();
            if date.year_ce().1 != *year
                || photo
                    .config
                    .author
                    .as_ref()
                    .or(gallery.config.author.as_ref())
                    .map(|a| a != author)
                    .unwrap_or(true)
            {
                return;
            }

            let filename = format!(
                "{}.{}",
                photo.slug(),
                gallery.config.photo_format.extension()
            );
            manifest.push(PhotoCopyrightSubmission {
                title: photo.output_name().to_owned(),
                filename,
                date_time,
                photo,
            });
        });

        manifest.sort_by_key(|m| (m.date_time, m.filename.clone()));

        if let Some(start) = start {
            let index = manifest
                .iter()
                .position(|submission| submission.title == *start || submission.filename == *start)
                .unwrap();
            manifest.drain(0..index);
        }
        if manifest.len() > *limit {
            println!("WARNING: truncated to {limit} photos");
            manifest.truncate(*limit);
        }

        for submission in &manifest {
            archive
                .start_file(&submission.filename, zip_options)
                .unwrap();
            let image_bytes = write_image(
                submission.photo.image(&gallery.config),
                &submission.filename,
                Some((&gallery, submission.photo)),
            );
            archive.write_all(&image_bytes).unwrap();
            let date = submission.date_time.date();
            if earliest_date.map(|d| date < d).unwrap_or(true) {
                earliest_date = Some(date);
            }
            if latest_date.map(|d| date > d).unwrap_or(true) {
                latest_date = Some(date);
            }
        }

        let group_name = format!(
            "{} {} photos by {author} in {year}{}",
            manifest.len(),
            gallery.config.title,
            if let Some(start) = start {
                format!(" starting with {start}")
            } else {
                String::new()
            }
        );
        let manifest_filename = format!("{group_name} Case Number {case_number}.pdf",);
        archive.start_file(&manifest_filename, zip_options).unwrap();

        let mut doc = genpdfi::Document::new(FontFamily {
            regular: FontData::new(
                include_bytes!("./copyright_registration/fonts/LiberationMono-Regular.ttf")
                    .to_vec(),
                None,
            )
            .unwrap(),
            italic: FontData::new(
                include_bytes!("./copyright_registration/fonts/LiberationMono-Italic.ttf").to_vec(),
                None,
            )
            .unwrap(),
            bold: FontData::new(
                include_bytes!("./copyright_registration/fonts/LiberationMono-Bold.ttf").to_vec(),
                None,
            )
            .unwrap(),
            bold_italic: FontData::new(
                include_bytes!("./copyright_registration/fonts/LiberationMono-BoldItalic.ttf")
                    .to_vec(),
                None,
            )
            .unwrap(),
        });

        let mut page_decorator = SimplePageDecorator::new();
        page_decorator.set_margins(Margins::all(5.0));
        doc.set_page_decorator(page_decorator);
        doc.set_title(manifest_filename);
        doc.set_font_size(8);
        doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
            "Group Registration of Published Photographs",
            Style::new().bold().with_font_size(16),
            None,
        )));
        doc.push(genpdfi::elements::Break::new(1.0));
        doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
            format!("This is the Complete List of Photographs for: {case_number}"),
            Style::new().bold(),
            None,
        )));
        doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
            format!("The group is: {group_name}"),
            Style::new(),
            None,
        )));
        doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
            format!("The author is: {author}"),
            Style::new(),
            None,
        )));
        doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
            format!("The year of completion is: {year}"),
            Style::new(),
            None,
        )));
        if let Some((earliest_date, latest_date)) = earliest_date.zip(latest_date) {
            doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
                format!(
                    "The earliest publication date is: {}",
                    earliest_date.format("%m/%d/%Y")
                ),
                Style::new(),
                None,
            )));
            doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
                format!(
                    "The latest publication date is: {}",
                    latest_date.format("%m/%d/%Y")
                ),
                Style::new(),
                None,
            )));
        }
        doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
            format!("Number of photos: {}", manifest.len()),
            Style::new(),
            None,
        )));

        doc.push(genpdfi::elements::Break::new(1.0));

        let mut table = genpdfi::elements::TableLayout::new(vec![1, 3, 3, 1]);

        table.set_cell_decorator(MarginFrameCellDecorator::new(true, true, false));
        table
            .push_row(vec![
                Box::new(genpdfi::elements::Text::new(StyledString::new(
                    "Photo No.",
                    Style::new().bold(),
                    None,
                ))),
                Box::new(genpdfi::elements::Text::new(StyledString::new(
                    "Title of Photo",
                    Style::new().bold(),
                    None,
                ))),
                Box::new(genpdfi::elements::Text::new(StyledString::new(
                    "Filename of Photo",
                    Style::new().bold(),
                    None,
                ))),
                Box::new(genpdfi::elements::Text::new(StyledString::new(
                    "Pub. Date",
                    Style::new().bold(),
                    None,
                ))),
            ])
            .unwrap();

        for (i, submission) in manifest.iter().enumerate() {
            table
                .push_row(vec![
                    Box::new(genpdfi::elements::Text::new((i + 1).to_string())),
                    Box::new(genpdfi::elements::Text::new(&submission.title)),
                    Box::new(genpdfi::elements::Text::new(&submission.filename)),
                    Box::new(genpdfi::elements::Text::new(format!(
                        "{}/{}",
                        submission.date_time.date().month(),
                        submission.date_time.date().year_ce().1
                    ))),
                ])
                .unwrap();
        }

        doc.push(table);

        doc.push(genpdfi::elements::Break::new(1.0));

        doc.push(genpdfi::elements::Paragraph::new(StyledString::new(
            format!(
                "This PDF was generated by chillphoto on {}.",
                chrono::Utc::now()
            ),
            Style::new(),
            None,
        )));

        doc.render(&mut archive).unwrap();

        let archive_contents = archive.finish().unwrap().into_inner();
        assert!(archive_contents.len() <= 500 * 1000 * 1000);
        fs::write(format!("{author}-{year}.zip"), archive_contents).unwrap();
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
