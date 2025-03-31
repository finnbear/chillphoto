#![allow(unused)]

use category_path::CategoryPath;
use chrono::NaiveDate;
use config::Config;
use exif::ExifData;
use gallery::{Gallery, Item, Page, PageFormat};
use http::request::Parts;
use http::{Extensions, HeaderMap, HeaderName, HeaderValue, Method, Uri, Version};
use httparse::Status;
use image::imageops::{self, resize, FilterType};
use image::{DynamicImage, GenericImageView, RgbImage};
use photo::Photo;
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::io::Write;
use std::io::{Cursor, Read};
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::thread::available_parallelism;
use std::time::{Duration, Instant};
use util::remove_dir_contents;
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

        let input_image_data = std::fs::read(entry.path()).unwrap();

        let mut photo = Photo {
            name: name.rsplit_once('.').unwrap().0.to_owned(),
            description: None,
            exif: ExifData::load(&input_image_data),
            input_image_data,
            image: Default::default(),
            preview: Default::default(),
            thumbnail: Default::default(),
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

    let output = &gallery.output();

    println!(
        "({:.1}s) Generated output manifest",
        start.elapsed().as_secs_f32(),
    );

    if fs::exists(&config.output).unwrap() {
        remove_dir_contents(&config.output).expect("failed to clear output directory");
    }

    let http_threads = &AtomicUsize::new(0);
    let http_idle = &Condvar::new();
    let mut queue = output.iter().collect::<Vec<_>>();
    queue.sort_by_key(|(path, _)| !path.contains("_thumbnail"));
    let work = &Mutex::new(queue.iter());
    std::thread::scope(|scope| {
        // Background initialization.
        let cpus = available_parallelism().map(|n| n.get()).unwrap_or(1);
        for thread in 0..cpus {
            scope.spawn(move || {
                while let Some((path, i)) = {
                    let next = work.lock().unwrap().next();
                    next
                } {
                    LazyLock::force(i);
                    println!("[background] {path}");
                    while http_threads.load(Ordering::SeqCst) > thread {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            });
        }

        let listener = std::net::TcpListener::bind("0.0.0.0:8080").unwrap();

        loop {
            let mut stream = if let Ok((stream, _)) = listener.accept() {
                stream
            } else {
                continue;
            };
            scope.spawn(move || {
                struct Guard<'a>(&'a AtomicUsize);

                http_threads.fetch_add(1, Ordering::SeqCst);

                impl<'a> Drop for Guard<'a> {
                    fn drop(&mut self) {
                        self.0.fetch_sub(1, Ordering::SeqCst);
                    }
                }

                let _guard = Guard(http_threads);

                let mut buf = Vec::new();

                let (request, body) = loop {
                    let mut tmp = [0u8; 1024];
                    match stream.read(&mut tmp) {
                        Ok(0) => return,
                        Ok(n) => {
                            buf.extend_from_slice(&tmp[0..n]);
                        }
                        Err(_) => {
                            return;
                        }
                    };

                    let mut headers = [httparse::EMPTY_HEADER; 128];
                    let mut parse_req = httparse::Request::new(&mut headers);
                    let res = parse_req.parse(&buf).unwrap();
                    if let Status::Complete(body) = res {
                        let method = if let Some(method) =
                            parse_req.method.and_then(|m| Method::from_str(m).ok())
                        {
                            method
                        } else {
                            return;
                        };
                        let uri =
                            if let Some(uri) = parse_req.path.and_then(|p| Uri::from_str(p).ok()) {
                                uri
                            } else {
                                return;
                            };
                        let headers = HeaderMap::new();

                        let mut builder = http::Request::builder().method(method).uri(uri).version(
                            if parse_req.version == Some(1) {
                                Version::HTTP_11
                            } else {
                                Version::HTTP_10
                            },
                        );
                        for header in parse_req.headers {
                            builder = builder.header(header.name, header.value);
                        }
                        let request = builder.body(Vec::<u8>::new()).unwrap();

                        break (request, body);
                    }
                };

                let response = if request.uri().path().ends_with('/') {
                    http::Response::builder()
                        .version(request.version())
                        .status(307)
                        .header("Location", format!("{}index.html", request.uri().path()))
                        .body(Vec::new())
                        .unwrap()
                } else if let Some(file) = output.get(request.uri().path()) {
                    http::Response::builder()
                        .version(request.version())
                        .status(200)
                        .body((&***file).to_vec())
                        .unwrap()
                } else {
                    http::Response::builder()
                        .version(request.version())
                        .status(404)
                        .body(b"not found".to_vec())
                        .unwrap()
                };

                println!("[{}] {}", response.status(), request.uri());

                let status_line = format!(
                    "{:?} {} {}\r\n",
                    response.version(),
                    response.status().as_u16(),
                    response.status().canonical_reason().unwrap()
                );

                let mut headers = String::new();
                for (name, value) in response.headers() {
                    headers.push_str(&format!("{}: {}\r\n", name, value.to_str().unwrap_or("")));
                }

                let body: &[u8] = response.body().as_ref();
                let content_length = body.len();
                headers.push_str(&format!("Content-Length: {}\r\n\r\n", content_length));

                if stream.write_all(status_line.as_bytes()).is_err() {
                    return;
                }
                if stream.write_all(headers.as_bytes()).is_err() {
                    return;
                }
                if stream.write_all(body).is_err() {
                    return;
                }
                let _ = stream.flush();
            });
        }
    });

    /*
    output.into_par_iter().for_each(|(path, generator)| {
        let contents = &*generator;
        if let Some((dir, _)) = path.rsplit_once('/') {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    });
    */

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
