use crate::{
    gallery::{CategoryPath, Gallery, Item, Order, Page, Photo},
    output::search::render_search,
};
use chrono::Datelike;
use image::{ImageFormat, RgbImage};
use sitemap_rs::{
    image::Image,
    url::{ChangeFrequency, Url},
    url_builder::UrlBuilder,
    url_set::UrlSet,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    fs,
    io::Cursor,
    sync::LazyLock,
};
use xmp_toolkit::{xmp_ns, OpenFileOptions, XmpMeta, XmpValue};
use yew::{html, Html};

mod app;
mod format;
mod pwa;
mod rich_text;
mod search;
mod serve;
mod structured_data;

pub use app::*;
pub use format::*;
pub use pwa::*;
pub use rich_text::*;
pub use serve::*;
pub use structured_data::*;

fn page_items<'a>(gallery: &'a Gallery, path: &CategoryPath) -> Vec<(String, &'a Page)> {
    let mut ret: Vec<(String, &'a Page)> = path
        .iter_paths()
        .flat_map(|path| {
            gallery
                .children(&path)
                .into_iter()
                .flatten()
                .filter_map(|i| i.page())
                .filter(|p| !p.config.unlisted)
                .map(move |p| (gallery.config.page_html::<true>(&path, &p.slug()), p))
        })
        .collect();
    ret.sort_by_key(|(_, p)| Order::new_page(p));
    ret
}

impl Gallery {
    pub fn output<'a>(
        &'a self,
    ) -> HashMap<String, LazyLock<Vec<u8>, Box<dyn FnOnce() -> Vec<u8> + Send + Sync + 'a>>> {
        let config = &self.config;

        let mut ret = HashMap::<
            String,
            LazyLock<Vec<u8>, Box<dyn FnOnce() -> Vec<u8> + Send + Sync + 'a>>,
        >::new();
        fn ret_insert<'a>(
            ret: &mut HashMap<
                String,
                LazyLock<Vec<u8>, Box<dyn FnOnce() -> Vec<u8> + Send + Sync + 'a>>,
            >,
            path: String,
            file: LazyLock<Vec<u8>, Box<dyn FnOnce() -> Vec<u8> + Send + Sync + 'a>>,
        ) {
            assert!(ret.insert(path.clone(), file).is_none(), "duplicate {path}");
        }
        let mut sitemap = Vec::<Url>::new();

        let root_og_image = self.thumbnail().map(|(path, preview)| {
            (
                self.config.preview::<true>(&path, &preview.slug()),
                preview.preview_dimensions(&self.config),
            )
        });

        self.visit_items(|path, item| {
            let path = path.clone();
            match item {
                Item::Photo(photo) => {
                    let mut num_photos = 0usize;
                    let mut photo_index = Option::<usize>::None;
                    let photos = self
                        .children(&path)
                        .unwrap()
                        .iter()
                        .filter_map(|i| i.photo())
                        .collect::<Vec<_>>();
                    for p in &photos {
                        if p.name == photo.name {
                            photo_index = Some(num_photos);
                        }
                        num_photos += 1;
                    }
                    let photo_index = photo_index.unwrap();
                    let photo_path = config.photo::<false>(&path, &photo.slug());
                    let xmp = Some((self, photo));
                    ret_insert(&mut ret,
                        photo_path.clone(),
                        LazyLock::new(Box::new(move || {
                            write_image(photo.image(&self.config), &photo_path, xmp)
                        })),
                    );
                    let preview_path = config.preview::<false>(&path, &photo.slug());
                    ret_insert(&mut ret,
                        preview_path.clone(),
                        LazyLock::new(Box::new(move || {
                            write_image(photo.preview(&self.config), &preview_path, xmp)
                        })),
                    );
                    let thumbnail_path = config.thumbnail::<false>(&path, &photo.slug());
                    ret_insert(&mut ret,
                        thumbnail_path.clone(),
                        LazyLock::new(Box::new(move || {
                            write_image(photo.thumbnail(&self.config), &thumbnail_path, xmp)
                        })),
                    );

                    let canonical = config.photo_html::<true>(&path, &photo.slug());
                    if let Some(root) = &self.config.root_url {
                        sitemap.push(UrlBuilder::new(format!("{root}{canonical}"))
                            .change_frequency(ChangeFrequency::Monthly)
                            .images(vec![
                                Image::new(format!("{root}{}", config.photo::<true>(&path, &photo.slug()))),
                                Image::new(format!("{root}{}", config.preview::<true>(&path, &photo.slug()))),
                                Image::new(format!("{root}{}", config.thumbnail::<true>(&path, &photo.slug()))),
                            ]).build().unwrap())
                    }

                    let page_items = page_items(self, &path);
                    ret_insert(&mut ret,
                        config.photo_html::<false>(&path, &photo.slug()),
                        LazyLock::new(Box::new(move || {
                            let photo_structured_data = write_structured_data(photo_structured_data(self, photo, config.photo_html::<true>(&path, &photo.slug()), self.config.photo::<true>(&path, &photo.slug()), Some(self.config.thumbnail::<true>(&path, &photo.slug())), true));

                            let group = self.item_name(&path);

                            render_html(AppProps {
                                canonical,
                                gallery: self,
                                title: format!("{} | {group}", photo.output_name()).into(),
                                description: photo.config.description.clone().map(|s| s.into()),
                                head: photo_structured_data,
                                body: html! {<>
                                    <a
                                        class="preview_container"
                                        href={config.photo::<true>(&path, &photo.slug())}
                                    >
                                        <img
                                            class="preview"
                                            width={photo.preview_dimensions(config).0.to_string()}
                                            height={photo.preview_dimensions(config).1.to_string()}
                                            alt={photo.config.description.clone().unwrap_or_else(|| photo.output_name().to_owned())}
                                            src={config.preview::<true>(&path, &photo.slug())}
                                        />
                                    </a>
                                    if let Some(text) = &photo.text {
                                        {rich_text_html(text)}
                                    }
                                    /*
                                    {Html::from_html_unchecked(r#"
                                        <div class="commentbox"></div>
                                        <script src="https://unpkg.com/commentbox.io/dist/commentBox.min.js"></script>
                                        <script>commentBox('5728549692506112-proj')</script>
                                    "#.into())}
                                    */
                                </>},
                                sidebar: html!{
                                    <div class="sidebar_panel">
                                        <h2 class="sidebar_panel_heading">{"Details"}</h2>
                                        if let Some(location) = photo.config.location.clone() {
                                            <div
                                                title={"Location"}
                                                class={"sidebar_details_panel_text"}
                                            >
                                                {location}
                                            </div>
                                        }
                                        if let Some(date_time) = photo.date_time() {
                                            <time
                                                datetime={date_time.date().to_string()}
                                                title={"Date Taken"}
                                                class={"sidebar_details_panel_text"}
                                            >
                                                {self.config.format_date(date_time.date())}
                                            </time>
                                        }
                                        if photo.exif.focal_length.is_some() || photo.exif.aperture.is_some() {
                                            <div
                                                title={photo.exif.lens_model.as_ref().map(|s| s.trim_matches('"').to_owned())}
                                                class={"sidebar_details_panel_text"}
                                            >
                                                if let Some(focal_length) = &photo.exif.focal_length {
                                                    {focal_length.replace(' ', "").to_owned()}
                                                }
                                                if photo.exif.focal_length.is_some() && photo.exif.aperture.is_some() {
                                                    {" "}
                                                }
                                                if let Some(aperture) = &photo.exif.aperture {
                                                    {aperture.clone()}
                                                }
                                            </div>
                                        }
                                        if photo.exif.exposure_time.is_some() || photo.exif.iso_sensitivity.is_some() {
                                            <div
                                                title={photo.exif.camera_model.as_ref().map(|s| s.trim_matches('"').to_owned())}
                                                class={"sidebar_details_panel_text"}
                                            >
                                                if let Some(exposure_time) = &photo.exif.exposure_time {
                                                    {exposure_time.replace(' ', "").to_owned()}
                                                }
                                                if photo.exif.exposure_time.is_some() && photo.exif.iso_sensitivity.is_some() {
                                                    {" "}
                                                }
                                                if let Some(iso_sensitivity) = &photo.exif.iso_sensitivity {
                                                    {format!("ISO{}", iso_sensitivity)}
                                                }
                                            </div>
                                        }
                                        if photo.config.exposure != 0.0 {
                                            <details class={"sidebar_details_panel_text"}>
                                                <summary>{"Adjustments"}</summary>
                                                {format!("{:+}EV exposure", photo.config.exposure)}
                                            </details>
                                        }
                                        if let Some(description) = &photo.config.description {
                                            <details class={"sidebar_details_panel_text"}>
                                                <summary>{"Description"}</summary>
                                                {description.clone()}
                                            </details>
                                        }
                                    </div>
                                },
                                pages: page_items,
                                path: path.push(photo.slug()).clone(),
                                relative: Some(RelativeNavigation {
                                    index: photo_index,
                                    count: num_photos,
                                    previous: photo_index
                                        .checked_sub(1)
                                        .and_then(|i| photos.get(i))
                                        .map(|p| config.photo_html::<true>(&path, &p.slug())),
                                    next: photo_index
                                        .checked_add(1)
                                        .and_then(|i| photos.get(i))
                                        .map(|p| config.photo_html::<true>(&path, &p.slug())),
                                }),
                                og_image: Some((self.config.preview::<true>(&path, &photo.slug()), photo.preview_dimensions(&self.config))),
                            })
                        })),
                    );
                }
                Item::Category(category) => {
                    let category_path = path.push(category.slug());
                    let page_items = page_items(self, &category_path);
                    for chunk in paginate(&category.children, category.config.items_per_page) {
                        let path = path.clone();
                        let category_path = category_path.clone();
                        let page_items = page_items.clone();
                        let mut title = category.name.clone();
                        if chunk.index != 0 {
                            write!(title, " (page {})", chunk.index + 1).unwrap();
                        }
                        ret_insert(&mut ret,
                            config.category_html::<false>(&path, &category.slug(), chunk.index),
                            LazyLock::new(Box::new(move || {
                                render_html(AppProps {
                                    canonical: config.category_html::<true>(&path, &category.slug(), chunk.index),
                                    gallery: self,
                                    title: title.into(),
                                    description: category.config.description.clone().map(|d| d.into()),
                                    head: Default::default(),
                                    body: html!{<>
                                        {render_items(self, &category_path, chunk.items)}
                                        if let Some(text) = &category.text {
                                            {rich_text_html(text)}
                                        }
                                    </>},
                                    sidebar: Html::default(),
                                    pages: page_items,
                                    path: category_path.clone(),
                                    relative: (chunk.count != 1).then_some(RelativeNavigation {
                                        index: chunk.index,
                                        count: chunk.count,
                                        previous: (chunk.index != 0)
                                            .then(|| config.category_html::<true>(&path, &category.slug(), chunk.index - 1)),
                                        next: (chunk.index != chunk.count - 1)
                                            .then(|| config.category_html::<true>(&path, &category.slug(), chunk.index + 1)),
                                    }),
                                    og_image: category.thumbnail(&path).map(|(path, preview)| (self.config.preview::<true>(&path, &preview.slug()), preview.preview_dimensions(&self.config)))
                                })
                            })),
                        );
                    };
                }
                Item::Page(page) => {
                    let page_items = page_items(self, &path);
                    let root_thumbnail = root_og_image.clone();
                    ret_insert(&mut ret,
                        config.page_html::<false>(&path, &page.slug()),
                        LazyLock::new(Box::new(move || {
                            render_html(AppProps {
                                canonical: config.page_html::<true>(&path, &page.slug()),
                                gallery: self,
                                title: page.name.clone().into(),
                                description: page.config.description.clone().map(|s| s.into()),
                                head: Default::default(),
                                body: rich_text_html(&page.text),
                                sidebar: Html::default(),
                                pages: page_items.clone(),
                                path: path.push(page.slug()),
                                relative: None,
                                og_image: root_thumbnail,
                            })
                        })),
                    );
                }
            }
        });

        if self.favicon.is_some() {
            let favicon_path = config.favicon::<false>();
            ret_insert(
                &mut ret,
                favicon_path.clone(),
                LazyLock::new(Box::new(move || {
                    write_image(self.favicon().unwrap(), &favicon_path, None)
                })),
            );
        }

        let manifest_path = config.manifest::<false>();
        ret_insert(
            &mut ret,
            manifest_path.clone(),
            LazyLock::new(Box::new(move || write_manifest(self))),
        );

        if let Some((_, thumbnail)) = self.thumbnail() {
            let manifest_path = "/manifest.png".to_owned();
            ret_insert(
                &mut ret,
                manifest_path.clone(),
                LazyLock::new(Box::new(move || {
                    write_image(
                        &thumbnail.custom_thumbnail(&self.config, MANIFEST_ICON_RESOLUTION),
                        &manifest_path,
                        None,
                    )
                })),
            );
        }

        let page_items = page_items(self, &CategoryPath::ROOT);
        {
            let page_items = page_items.clone();
            let root_og_image = root_og_image.clone();
            ret_insert(
                &mut ret,
                self.config.search_html::<false>(),
                LazyLock::new(Box::new(move || {
                    render_html(AppProps {
                        canonical: self.config.search_html::<true>(),
                        gallery: self,
                        title: format!("Search {}", self.config.title).into(),
                        description: Some(format!("Search {}", self.config.title).into()),
                        head: Default::default(),
                        body: render_search(self),
                        sidebar: Html::default(),
                        pages: page_items,
                        path: CategoryPath::ROOT.push("search".to_owned()),
                        relative: None,
                        og_image: root_og_image,
                    })
                })),
            );
        }

        for chunk in paginate(&self.children, self.config.items_per_page) {
            let page_items = page_items.clone();
            let root_og_image = root_og_image.clone();
            ret_insert(
                &mut ret,
                self.config.index_html::<false>(chunk.index),
                LazyLock::new(Box::new(move || {
                    let mut title = self.config.title.clone();
                    if chunk.index != 0 {
                        write!(title, " (page {})", chunk.index + 1).unwrap();
                    }

                    render_html(AppProps {
                        canonical: self.config.index_html::<true>(chunk.index),
                        gallery: self,
                        title: title.into(),
                        description: self.config.description.clone().map(|d| d.into()),
                        head: Default::default(),
                        body: html! {<>
                            {render_items(self, &CategoryPath::ROOT, chunk.items)}
                            if let Some(text) = &self.home_text {
                                {rich_text_html(text)}
                            }
                        </>},
                        sidebar: Html::default(),
                        pages: page_items,
                        path: CategoryPath::ROOT,
                        relative: (chunk.count != 1).then_some(RelativeNavigation {
                            index: chunk.index,
                            count: chunk.count,
                            previous: (chunk.index != 0)
                                .then(|| self.config.index_html::<true>(chunk.index - 1)),
                            next: (chunk.index != chunk.count - 1)
                                .then(|| self.config.index_html::<true>(chunk.index + 1)),
                        }),
                        og_image: root_og_image.clone(),
                    })
                })),
            );
        }

        if let Some(root_url) = &self.config.root_url {
            for page in ret
                .keys()
                .filter_map(|k| {
                    let mut k = k.as_str();
                    let q = if let Some((p, q)) = k.split_once('?') {
                        k = p;
                        format!("?{q}")
                    } else {
                        "".to_owned()
                    };

                    if k.ends_with('/') || k.ends_with(".html") {
                        Some(format!("{}{}", k.trim_end_matches("index.html"), q))
                    } else {
                        None
                    }
                })
                .map(|s| {
                    Url::builder(format!("{root_url}{}", s))
                        .change_frequency(ChangeFrequency::Weekly)
                        .build()
                        .unwrap()
                })
            {
                if sitemap.iter().all(|url| url.location != page.location) {
                    sitemap.push(page);
                }
            }

            // add some search queries to sitemap
            if let Some(root_url) = &self.config.root_url {
                let mut locations = HashSet::<&str>::new();
                self.visit_items(|_, item| {
                    if let Item::Photo(photo) = item {
                        if let Some(location) = &photo.config.location {
                            locations.insert(location);
                        }
                    }
                });

                for location in locations {
                    let page = Url::builder(format!(
                        "{root_url}/search/?query={}",
                        location.replace(" ", "+")
                    ))
                    .change_frequency(ChangeFrequency::Weekly)
                    .build()
                    .unwrap();
                    sitemap.push(page);
                }
            }

            sitemap.sort_by_key(|url| {
                (
                    url.location.contains("/?query="),
                    url.location.chars().filter(|c| *c == '/').count(),
                    url.location.clone(),
                )
            });
            let sitemap = UrlSet::new(sitemap).unwrap();
            ret_insert(
                &mut ret,
                "/sitemap.xml".to_owned(),
                LazyLock::new(Box::new(move || {
                    let mut ret = Vec::<u8>::new();
                    sitemap.write(&mut ret).unwrap();
                    ret
                })),
            );
        }

        ret_insert(
            &mut ret,
            "/robots.txt".to_owned(),
            LazyLock::new(Box::new(move || {
                let mut robots_txt = String::new();
                writeln!(robots_txt, "User-agent: *").unwrap();
                if self.config.disallow_ai_training {
                    writeln!(robots_txt, "DisallowAITraining: /").unwrap();
                }
                writeln!(robots_txt, "Allow: /").unwrap();
                if let Some(url) = &self.config.root_url {
                    writeln!(robots_txt, "Sitemap: {url}/sitemap.xml").unwrap();
                }
                robots_txt.into_bytes()
            })),
        );

        for file in &self.static_files {
            ret_insert(
                &mut ret,
                file.path.clone(),
                LazyLock::new(Box::new(move || file.contents.clone())),
            );
        }

        ret
    }
}

struct PageChunk<'a> {
    items: &'a [Item],
    index: usize,
    #[allow(unused)]
    count: usize,
}

fn paginate(items: &[Item], items_per_page: usize) -> impl Iterator<Item = PageChunk<'_>> + '_ {
    let items = &items[0..items
        .iter()
        .rposition(|i| i.photo().is_some() || i.category().is_some())
        .unwrap()
        + 1];
    let chunks = items.chunks(items_per_page);
    let count = chunks.len();
    chunks.enumerate().map(move |(index, items)| PageChunk {
        items,
        index,
        count,
    })
}

#[derive(PartialEq)]
pub struct RelativeNavigation {
    index: usize,
    count: usize,
    previous: Option<String>,
    next: Option<String>,
}

fn render_items(gallery: &Gallery, category_path: &CategoryPath, items: &[Item]) -> Html {
    html! {
        <section id="page_main_body_items" data-nosnippet="nosnippet">
            {items.iter().filter_map(|child| {
                match child {
                    Item::Photo(photo) => {
                        let content_url = gallery.config.photo::<true>(&category_path, &photo.slug());
                        let thumbnail_url = gallery.config.thumbnail::<true>(&category_path, &photo.slug());
                        let html_url = gallery.config.photo_html::<true>(&category_path, &photo.slug());
                        Some(html!{
                            <a
                                class="thumbnail_container"
                                href={html_url.clone()}
                            >
                                <img
                                    title={photo.output_name().to_owned()}
                                    alt={photo.config.description.clone().unwrap_or_else(|| photo.output_name().to_owned())}
                                    src={thumbnail_url.clone()}
                                    class="thumbnail"
                                />
                                {write_structured_data(
                                    photo_structured_data(gallery, photo, html_url, content_url, Some(thumbnail_url), false)
                                )}
                            </a>
                        })
                    }
                    Item::Category(category) => {
                        let (photo_path, photo) = category.thumbnail(category_path)?;
                        let thumbnail_url = gallery.config.thumbnail::<true>(&photo_path, &photo.slug());
                        Some(html!{
                            <a
                                class="thumbnail_container category_item"
                                href={gallery.config.category_html::<true>(&category_path, &category.slug(), 0)}
                            >
                                <img
                                    class="thumbnail"
                                    alt={photo.output_name().to_owned()}
                                    src={thumbnail_url.clone()}
                                />
                                <div class="category_item_info">
                                    <h2 class="category_item_name">
                                        {category.name.clone()}
                                    </h2>
                                    if let Some((first_date, last_date)) = category.first_and_last_dates() {
                                        <div class="category_item_dates">
                                            <time
                                                datetime={first_date.to_string()}
                                            >
                                                {first_date.year_ce().1}
                                            </time>
                                            <span style={(first_date.year_ce() == last_date.year_ce()).then_some("display: none;")}>
                                                {" - "}
                                                <time
                                                    datetime={last_date.to_string()}
                                                >
                                                    {last_date.year_ce().1}
                                                </time>
                                            </span>
                                        </div>
                                    }
                                    if let Some(description) = category.config.description.clone() {
                                        <div class="category_item_description">
                                            {description}
                                        </div>
                                    }
                                </div>
                                {write_structured_data(
                                    photo_structured_data(
                                        gallery,
                                        photo,
                                        gallery.config.photo_html::<true>(&photo_path, &photo.slug()),
                                        gallery.config.photo::<true>(&photo_path, &photo.slug()),
                                        Some(thumbnail_url),
                                        false
                                    )
                                )}
                            </a>
                        })
                    }
                    Item::Page(_) => None
                }
            }).collect::<Html>()}
        </section>
    }
}

pub fn write_image(img: &RgbImage, path: &str, xmp: Option<(&Gallery, &Photo)>) -> Vec<u8> {
    let mut ret = Cursor::new(Vec::new());
    let format = ImageFormat::from_path(path).unwrap();
    img.write_to(&mut ret, format).unwrap();
    let mut buf = ret.into_inner();
    if let Some((gallery, photo)) = xmp {
        // Awaiting https://github.com/adobe/xmp-toolkit-rs/issues/265
        let file = tempfile::Builder::new()
            .suffix(&format!(".xmp.{}", format.extensions_str()[0]))
            .tempfile()
            .unwrap();
        let path = file.path();
        fs::write(&path, &buf).unwrap();
        let mut xmp_file = xmp_toolkit::XmpFile::new().unwrap();
        xmp_file
            .open_file(
                &path,
                OpenFileOptions::default()
                    .for_update()
                    .only_xmp()
                    .use_smart_handler(),
            )
            .unwrap();
        let mut xmp = XmpMeta::new().unwrap();

        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            XmpMeta::register_namespace("http://ns.useplus.org/ldf/xmp/1.0/", "plus").unwrap();
        });

        xmp.set_property(
            xmp_ns::DC,
            "title",
            &XmpValue::new(photo.output_name().to_owned()),
        )
        .unwrap();

        let author = photo
            .config
            .author
            .clone()
            .or_else(|| gallery.config.author.clone());
        let copyright_year = photo.date_time().map(|d| d.year());

        if let Some(copyright_notice) = copyright_notice(author.as_deref(), copyright_year) {
            xmp.set_property(
                xmp_ns::DC,
                "rights",
                &XmpValue::new(copyright_notice.clone()),
            )
            .unwrap();
            xmp.set_property(
                xmp_ns::PHOTOSHOP,
                "credit",
                &XmpValue::new(copyright_notice),
            )
            .unwrap();
        }

        if let Some(author) = author {
            xmp.set_property(xmp_ns::DC, "creator", &XmpValue::new(author))
                .unwrap();
        }

        if let Some(license_url) = photo
            .config
            .license_url
            .clone()
            .or_else(|| gallery.config.license_url.clone())
        {
            xmp.set_property(
                xmp_ns::XMP_RIGHTS,
                "WebStatement",
                &XmpValue::new(license_url),
            )
            .unwrap();
        }
        if let Some(licensor_url) = gallery.config.root_url.clone() {
            xmp.set_struct_field(
                "http://ns.useplus.org/ldf/xmp/1.0/",
                "plus:Licensor",
                "http://ns.useplus.org/ldf/xmp/1.0/",
                "plus:LicensorURL",
                &XmpValue::new(licensor_url),
            )
            .unwrap();
        }

        if gallery.config.disallow_ai_training {
            xmp.set_property(
                "http://ns.useplus.org/ldf/xmp/1.0/",
                "plus:DataMining",
                &XmpValue::new(
                    "http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING".to_owned(),
                ),
            )
            .unwrap();
        }

        if let Some(description) = photo.config.description.clone() {
            xmp.set_property(
                xmp_ns::IPTC_CORE,
                "AltTextAccessibility",
                &XmpValue::new(description),
            )
            .unwrap();
        }

        xmp.set_name("chillphoto").unwrap();

        assert!(xmp_file.can_put_xmp(&xmp));

        xmp_file.put_xmp(&xmp).unwrap();

        xmp_file.try_close().unwrap();

        drop(xmp_file);

        buf = fs::read(&path).unwrap();

        drop(file);
    }
    buf
}

fn copyright_notice(author: Option<&str>, copyright_year: Option<i32>) -> Option<String> {
    match (author, copyright_year) {
        (Some(a), Some(y)) => Some(format!("© {y} {a}")),
        (Some(a), None) => Some(format!("© {a}")),
        (None, Some(y)) => Some(format!("© {y}")),
        (None, None) => None,
    }
}
