use crate::{
    category_path::CategoryPath,
    gallery::{Gallery, Item, Page, RichText, RichTextFormat},
    photo::Photo,
    util::date_format,
};
use chrono::Datelike;
use image::{ImageFormat, RgbImage};
use serde::Serialize;
use sitemap_rs::{
    image::Image,
    url::{ChangeFrequency, Url},
    url_builder::UrlBuilder,
    url_set::UrlSet,
};
use std::{collections::HashMap, fmt::Write, io::Cursor, sync::LazyLock};
use yew::{classes, function_component, html, AttrValue, Html, LocalServerRenderer, Properties};

/// Must be at least 144.
const MANIFEST_ICON_RESOLUTION: u32 = 256;

impl Gallery {
    pub fn output<'a>(
        &'a self,
    ) -> HashMap<String, LazyLock<Vec<u8>, Box<dyn FnOnce() -> Vec<u8> + Send + Sync + 'a>>> {
        let config = &self.config;

        // TODO: nested pages.
        let page_items = self
            .children
            .iter()
            .filter_map(|i| i.page())
            .map(|p| (config.page_html::<true>(&CategoryPath::ROOT, &p.name), p))
            .collect::<Vec<_>>();

        let mut ret = HashMap::<
            String,
            LazyLock<Vec<u8>, Box<dyn FnOnce() -> Vec<u8> + Send + Sync + 'a>>,
        >::new();
        let mut sitemap = Vec::<Url>::new();

        let root_og_image = self.thumbnail().map(|(path, preview)| {
            (
                self.config.preview::<true>(&path, &preview.name),
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
                    let photo_path = config.photo::<false>(&path, &photo.name);
                    ret.insert(
                        photo_path.clone(),
                        LazyLock::new(Box::new(move || {
                            write_image(photo.image(&self.config), &photo_path)
                        })),
                    );
                    let preview_path = config.preview::<false>(&path, &photo.name);
                    ret.insert(
                        preview_path.clone(),
                        LazyLock::new(Box::new(move || {
                            write_image(photo.preview(&self.config), &preview_path)
                        })),
                    );
                    let thumbnail_path = config.thumbnail::<false>(&path, &photo.name);
                    ret.insert(
                        thumbnail_path.clone(),
                        LazyLock::new(Box::new(move || {
                            write_image(photo.thumbnail(&self.config), &thumbnail_path)
                        })),
                    );

                    let canonical = config.photo_html::<true>(&path, &photo.name);
                    if let Some(root) = &self.config.root_url {
                        sitemap.push(UrlBuilder::new(format!("{root}{canonical}"))
                            .change_frequency(ChangeFrequency::Monthly)
                            .images(vec![
                                Image::new(format!("{root}{}", config.photo::<true>(&path, &photo.name))),
                                Image::new(format!("{root}{}", config.preview::<true>(&path, &photo.name))),
                                Image::new(format!("{root}{}", config.thumbnail::<true>(&path, &photo.name))),
                            ]).build().unwrap())
                    }

                    let page_items = page_items.clone();
                    ret.insert(
                        config.photo_html::<false>(&path, &photo.name),
                        LazyLock::new(Box::new(move || {
                            let photo_structured_data = write_structured_data(photo_structured_data(self, photo, config.photo_html::<true>(&path, &photo.name), self.config.photo::<true>(&path, &photo.name), Some(self.config.thumbnail::<true>(&path, &photo.name)), true));

                            render_html(AppProps {
                                canonical,
                                gallery: self,
                                title: photo.name.clone().into(),
                                description: photo.config.description.clone().map(|s| s.into()),
                                head: photo_structured_data,
                                body: html! {<>
                                    <a
                                        class="preview_container"
                                        href={config.photo::<true>(&path, &photo.name)}
                                    >
                                        <img
                                            class="preview"
                                            width={photo.preview_dimensions(config).0.to_string()}
                                            height={photo.preview_dimensions(config).1.to_string()}
                                            alt={photo.config.description.as_ref().unwrap_or(&photo.name).clone()}
                                            src={config.preview::<true>(&path, &photo.name)}
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
                                                {date_format(date_time.date())}
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
                                path: path.push(photo.name.clone()).clone(),
                                relative: Some(RelativeNavigation {
                                    index: photo_index,
                                    count: num_photos,
                                    previous: photo_index
                                        .checked_sub(1)
                                        .and_then(|i| photos.get(i))
                                        .map(|p| config.photo_html::<true>(&path, &p.name)),
                                    next: photo_index
                                        .checked_add(1)
                                        .and_then(|i| photos.get(i))
                                        .map(|p| config.photo_html::<true>(&path, &p.name)),
                                }),
                                og_image: Some((self.config.preview::<true>(&path, &photo.name), photo.preview_dimensions(&self.config))),
                            })
                        })),
                    );
                }
                Item::Category(category) => {
                    let category_path = path.push(category.slug());
                    let page_items = page_items.clone();
                    ret.insert(
                        config.category_html::<false>(&path, &category.slug()),
                        LazyLock::new(Box::new(move || {
                            render_html(AppProps {
                                canonical: config.category_html::<true>(&path, &category.slug()),
                                gallery: self,
                                title: category.name.clone().into(),
                                description: category.config.description.clone().map(|d| d.into()),
                                head: Default::default(),
                                body: html!{<>
                                    {render_items(self, &category_path, &category.children)}
                                    if let Some(text) = &category.text {
                                        {rich_text_html(text)}
                                    }
                                </>},
                                sidebar: Html::default(),
                                pages: page_items,
                                path: category_path.clone(),
                                relative: None,
                                og_image: category.thumbnail(&path).map(|(path, preview)| (self.config.preview::<true>(&path, &preview.name), preview.preview_dimensions(&self.config)))
                            })
                        })),
                    );
                }
                Item::Page(page) => {
                    let page_items = page_items.clone();
                    let root_thumbnail = root_og_image.clone();
                    ret.insert(
                        config.page_html::<false>(&path, &page.name),
                        LazyLock::new(Box::new(move || {
                            render_html(AppProps {
                                canonical: config.page_html::<true>(&path, &page.name),
                                gallery: self,
                                title: page.name.clone().into(),
                                description: page.config.description.clone().map(|s| s.into()),
                                head: Default::default(),
                                body: rich_text_html(&page.text),
                                sidebar: Html::default(),
                                pages: page_items.clone(),
                                path: path.push(page.name.clone()).clone(),
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
            ret.insert(
                favicon_path.clone(),
                LazyLock::new(Box::new(move || {
                    write_image(self.favicon().unwrap(), &favicon_path)
                })),
            );
        }

        let manifest_path = config.manifest::<false>();
        ret.insert(
            manifest_path.clone(),
            LazyLock::new(Box::new(move || write_manifest(self))),
        );

        if let Some((_, thumbnail)) = self.thumbnail() {
            let manifest_path = "/manifest.png".to_owned();
            ret.insert(
                manifest_path.clone(),
                LazyLock::new(Box::new(move || {
                    write_image(
                        &thumbnail.custom_thumbnail(&self.config, MANIFEST_ICON_RESOLUTION),
                        &manifest_path,
                    )
                })),
            );
        }

        ret.insert(
            self.config.index_html::<false>(),
            LazyLock::new(Box::new(move || {
                /// https://schema.org/WebSite
                #[derive(Serialize)]
                struct WebSiteStructuredData {
                    #[serde(rename = "@type")]
                    _type: &'static str,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    url: Option<String>,
                    name: String,
                    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
                    description: Option<String>,
                    #[serde(rename = "copyrightHolder", skip_serializing_if = "Option::is_none")]
                    copyright_holder: Option<PersonStructuredData>,
                }

                let website_structured_data = write_structured_data(WebSiteStructuredData {
                    _type: "WebSite",
                    url: self.config.root_url.clone(),
                    name: self.config.title.clone(),
                    description: self.config.description.clone(),
                    copyright_holder: self.config.author.clone().map(|name| PersonStructuredData {
                        _type: "Person",
                        name,
                    }),
                });

                render_html(AppProps {
                    canonical: self.config.index_html::<true>(),
                    gallery: self,
                    title: self.config.title.clone().into(),
                    description: self.config.description.clone().map(|d| d.into()),
                    head: website_structured_data,
                    body: html! {<>
                        {render_items(self, &CategoryPath::ROOT, &self.children)}
                        if let Some(text) = &self.home_text {
                            {rich_text_html(text)}
                        }
                    </>},
                    sidebar: Html::default(),
                    pages: page_items,
                    path: CategoryPath::ROOT,
                    relative: None,
                    og_image: root_og_image.clone(),
                })
            })),
        );

        if let Some(root_url) = &self.config.root_url {
            for page in ret
                .keys()
                .filter(|k| k.ends_with('/') || k.ends_with(".html"))
                .map(|s| {
                    Url::builder(format!("{root_url}{}", s.trim_end_matches("index.html")))
                        .change_frequency(ChangeFrequency::Weekly)
                        .build()
                        .unwrap()
                })
            {
                if sitemap.iter().all(|url| url.location != page.location) {
                    sitemap.push(page);
                }
            }
            sitemap.sort_by_key(|url| {
                (
                    url.location.chars().filter(|c| *c == '/').count(),
                    url.location.clone(),
                )
            });
            let sitemap = UrlSet::new(sitemap).unwrap();
            ret.insert(
                "/sitemap.xml".to_owned(),
                LazyLock::new(Box::new(move || {
                    let mut ret = Vec::<u8>::new();
                    sitemap.write(&mut ret).unwrap();
                    ret
                })),
            );
        }

        ret.insert(
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

        ret
    }
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
        <div id="page_main_body_items">
            {items.iter().filter_map(|child| {
                match child {
                    Item::Photo(photo) => {
                        let content_url = gallery.config.photo::<true>(&category_path, &photo.name);
                        let thumbnail_url = gallery.config.thumbnail::<true>(&category_path, &photo.name);
                        let html_url = gallery.config.photo_html::<true>(&category_path, &photo.name);
                        Some(html!{
                            <a
                                class="thumbnail_container"
                                href={html_url.clone()}
                            >
                                <img
                                    title={photo.name.clone()}
                                    alt={photo.config.description.as_ref().unwrap_or(&photo.name).clone()}
                                    src={thumbnail_url.clone()}
                                    style={format!(
                                        "width: {}px; height: {}px;",
                                        gallery.config.thumbnail_resolution,
                                        gallery.config.thumbnail_resolution
                                    )}
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
                        let thumbnail_url = gallery.config.thumbnail::<true>(&photo_path, &photo.name);
                        Some(html!{
                            <a
                                class="thumbnail_container category_item"
                                href={gallery.config.category_html::<true>(&category_path, &category.slug())}
                            >
                                <img
                                    class="thumbnail"
                                    style={format!(
                                        "width: {}px; height: {}px;",
                                        gallery.config.thumbnail_resolution,
                                        gallery.config.thumbnail_resolution
                                    )}
                                    alt={photo.name.clone()}
                                    src={thumbnail_url.clone()}
                                />
                                <div class="category_item_info">
                                    <h2 class="category_item_name">
                                        {category.name.clone()}
                                    </h2>
                                    if let Some(creation_date) = category.creation_date.clone() {
                                        <time
                                            datetime={creation_date.to_string()}
                                            class="category_item_creation_date"
                                        >
                                            {date_format(creation_date)}
                                        </time>
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
                                        gallery.config.photo_html::<true>(&photo_path, &photo.name),
                                        gallery.config.photo::<true>(&photo_path, &photo.name),
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
        </div>
    }
}

// Takes around 10ms.
fn render_html(props: AppProps<'_>) -> Vec<u8> {
    let html = app(props);

    #[derive(Properties, PartialEq)]
    struct InnerAppProps {
        html: Html,
    }

    #[function_component(InnerApp)]
    fn inner_app(props: &InnerAppProps) -> Html {
        props.html.clone()
    }

    let renderer =
        LocalServerRenderer::<InnerApp>::with_props(InnerAppProps { html }).hydratable(false);
    let html = futures::executor::block_on(renderer.render());

    let mut options = markup_fmt::config::FormatOptions::default();
    options.layout.use_tabs = true;
    options.layout.indent_width = 1;
    let mut html =
        markup_fmt::format_text(&html, markup_fmt::Language::Html, &options, |code, _| {
            Ok::<_, std::convert::Infallible>(code.into())
        })
        .unwrap();

    html.insert_str(0, "<!DOCTYPE html>\n");

    html = html
        .lines()
        .filter(|l| !l.chars().all(|c| c.is_whitespace()))
        .map(|l| format!("{l}\n"))
        .collect();

    html.into_bytes()
}

pub struct AppProps<'a> {
    pub gallery: &'a Gallery,
    pub canonical: String,
    pub path: CategoryPath,
    pub title: AttrValue,
    pub description: Option<AttrValue>,
    pub og_image: Option<(String, (u32, u32))>,
    pub head: Html,
    pub body: Html,
    pub sidebar: Html,
    pub pages: Vec<(String, &'a Page)>,
    pub relative: Option<RelativeNavigation>,
}

pub fn app(props: AppProps<'_>) -> Html {
    let style = Html::from_html_unchecked(
        r#"
        :root {
            --colored-text-light-background: #4d5a41;
            --colored-text-dark-background: #e0ff28;
        }

        @view-transition {
            navigation: auto;
        }

        html {
            font-size: calc(8px + 0.8vw);
        }

        body {
            background-color: #222222;
            margin: 2rem;
            user-select: none;
            -webkit-user-drag: none;
            font-family: "Helvetica Neue", "Lucida Grande", Arial, Helvetica, sans-serif;
        }

        h1, h2, h3, h4, h5, h6 {
            font-family: Times, "Times New Roman", Georgia, serif;
        }

        a {
            text-decoration: none;
            color: var(--colored-text-light-background);
        }

        #page {
            background-color: white;;
            max-width: 60rem;
            margin: 0rem auto;
            display: flex;
            flex-direction: column;
            border-radius: 0.5rem;
            overflow: hidden;
        }

        #header, #footer {
            background-color: #dadfbb;
        }

        #header {
            display: flex;
            flex-direction: row;
            gap: 1rem;
            align-items: center;
            padding: 2.5rem 2rem;
        }

        #title {
            font-weight: normal;
            letter-spacing: 0.1rem;
            flex-grow: 1;
            margin: 0;
        }

        #relative_navigation {
            display: flex;
            flex-direction: row;
            gap: 0.5rem;
        }

        .relative_navigation_unavailable {
            opacity: 0.5;
        }

        #breadcrumbs {
            background-color: #505050;
            padding: 0.5rem;
            padding-left: 2rem;
            color: white;
        }

        .breadcrumb {
            color: var(--colored-text-dark-background);
        }

        .breadcrumb_final {
            color: white;
            font-weight: bold;
        }

        #main_and_sidebar {
            display: flex;
            flex-direction: row;
            flex-grow: 1;
            min-height: 24rem;
        }

        #page_main_body {
            flex-grow: 1;
            margin: 2rem;
        }

        #page_main_body_items {
            display: flex;
            flex-wrap: wrap;
            gap: 0.5rem;
            align-content: flex-start;
        }

        #sidebar {
            background-color: #fbfbfb;
            min-width: 18rem;
            width: 18rem;
            box-shadow: -0.25rem 0px 0.5rem 0 rgba(0, 0, 0, 0.1);
            height: 100%;
        }

        .sidebar_panel {
            border-bottom: 1px dashed darkgray;
            padding: 1rem;
        }

        .sidebar_panel_heading {
            margin: 0;
            font-size: 1.2rem;
            font-weight: normal;
            font-style: italic;
        }

        .sidebar_panel_list {
            padding-inline-start: 0.5rem;
        }

        .sidebar_panel_list_item { 
            list-style: none;
            margin-top: 0.2rem;
            font-size: 0.9rem;
        }

        .sidebar_details_panel_text {
            font-size: 0.9rem;
        }

        details.sidebar_details_panel_text > summary {
            margin-left: .2rem;
        }
  
        .sidebar_panel_list_item::before { 
            content: "\00BB"; 
        } 

        #footer {
            text-align: center;
            padding: 0.5rem;
        }

        .thumbnail_container {
            padding: 0.5rem;
            display: inline-flex;
            flex-direction: row;
            gap: 0.5rem;
            border: 1px solid #e6e6e6;
            background-color: #FBFBF8;
            height: min-content;
        }

        .category_item_info {
            width: 10rem;
        }

        .category_item_name {
            margin: 0;
            overflow-wrap: anywhere;
            font-size: 1rem;
            font-weight: normal;
            color: var(--colored-text-light-background);
            text-overflow: ellipsis;
            white-space: nowrap;
            overflow: hidden;
        }

        .category_item_creation_date {
            color: black;
            font-size: 0.75rem;
        }

        .category_item_description {
            color: #373737;
            font-size: 0.75rem;
            margin-top: 0.2rem;
        }

        .preview {
            width: 100%;
            height: auto;
        }

        .thumbnail, .preview {
            background-color: #282828;
            font-size: 0.5rem;
        }
    "#
        .into(),
    );

    let breadcrumbs = props
        .path
        .iter_paths()
        .enumerate()
        .map(|(i, path)| {
            if path != props.path {
                BreadcrumbListElement {
                    _type: "ListItem",
                    name: props
                        .gallery
                        .category(&path)
                        .map(|c| c.name.as_str())
                        .unwrap_or("Home")
                        .to_owned(),
                    position: i + 1,
                    item: Some(if path.is_root() {
                        props.gallery.config.index_html::<true>()
                    } else {
                        props.gallery.config.category_html::<true>(
                            &path.pop().unwrap(),
                            path.last_segment().unwrap(),
                        )
                    }),
                }
            } else {
                BreadcrumbListElement {
                    _type: "ListItem",
                    name: props
                        .gallery
                        .category(&path)
                        .map(|c| c.name.as_str())
                        .or(path.last_segment())
                        .unwrap_or("Home")
                        .to_owned(),
                    position: i + 1,
                    item: None,
                }
            }
        })
        .collect::<Vec<_>>();

    html! {
        <html lang="en">
            <head>
                <meta charset="UTF-8"/>
                <title>{props.title.clone()}</title>
                <meta property="og:title" content={props.title.clone()} />
                if let Some(description) = props.description.clone() {
                    <meta name="description" content={description.clone()}/>
                    <meta property="og:description" content={description}/>
                }
                if !props.gallery.config.categories.is_empty() {
                    <meta name="keywords" content={props.gallery.config.categories.join(",")}/>
                }
                if let Some(author) = props.gallery.config.author.clone() {
                    <meta name="author" content={author}/>
                }
                <meta name="generator" content="chillphoto"/>
                if props.gallery.favicon.is_some() {
                    <link rel="icon" type="image/png" href="/favicon.png"/>
                }
                if props.gallery.config.disallow_ai_training {
                    <meta name="robots" content="index,follow,DisallowAITraining,noai,noimageai"/>
                }
                <link rel="manifest" href="/manifest.json"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta property="og:type" content="website" />
                if let Some(root) = &props.gallery.config.root_url {
                    <link rel="canonical" href={format!("{root}{}", props.canonical)}/>
                    <meta property="og:url" content={format!("{root}{}", props.canonical)}/>
                    if let Some((og_image, (width, height))) = &props.og_image {
                        <meta property="og:image" content={format!(
                            "{root}{og_image}",
                        )}/>
                        <meta property="og:image:width" content={width.to_string()}/>
                        <meta property="og:image:height" content={height.to_string()}/>
                    }
                }
                if let Some(relative) = &props.relative {
                    if let Some(previous) = &relative.previous {
                        <link rel="prev" href={previous.clone()}/>
                        <link rel="prerender" href={previous.clone()}/>
                    }
                    if let Some(next) = &relative.next {
                        <link rel="next" href={next.clone()}/>
                        <link rel="prerender" href={next.clone()}/>
                    }
                }
                {write_speculation_rules(
                    props
                        .relative
                        .as_ref()
                        .map(|relative|
                            relative
                                .previous
                                .clone()
                                .into_iter()
                                .chain(relative.next.clone()).collect()
                        )
                        .unwrap_or_default()
                )}
                // Favicon
                {props.head.clone()}
                if let Some(content) = props.gallery.head_html.clone() {
                    {rich_text_html(&RichText{
                        content,
                        format: RichTextFormat::Html,
                    })}
                }
                <style>{style}</style>
            </head>
            <body>
                <div id="page">
                    <section id="header_nav" data-nosnippet={"nosnippet"}>
                        <header id="header">
                            <h1 id="title">{props.gallery.config.title.clone()}</h1>
                            if let Some(relative) = &props.relative {
                                <div id="relative_navigation">
                                    <a
                                        href={relative.previous.clone()}
                                        class={classes!(
                                            "relative_navigation_previous",
                                            relative.previous.is_none().then_some("relative_navigation_unavailable"),
                                        )}
                                    >{"Previous"}</a>
                                    <a
                                        href={relative.next.clone()}
                                        class={classes!(
                                            "relative_navigation_previous",
                                            relative.next.is_none().then_some("relative_navigation_unavailable"),
                                        )}
                                    >{"Next"}</a>
                                </div>
                            }
                        </header>
                        <nav id="breadcrumbs" data-nosnippet={"nosnippet"}>
                            {join(&breadcrumbs.iter().map(|breadcrumb| html!{
                                if let Some(href) = &breadcrumb.item {
                                    <a
                                        class={"breadcrumb"}
                                        href={href.clone()}
                                    >{breadcrumb.name.clone()}</a>
                                } else {
                                    <span
                                        class={"breadcrumb breadcrumb_final"}
                                    >{breadcrumb.name.clone()}</span>
                                }
                            }).collect::<Vec<_>>(), &html!{{" » "}})}
                            if let Some(relative) = &props.relative {
                                {format!(" ({}/{})", relative.index + 1, relative.count)}
                            }
                            if breadcrumbs.len() > 1 {
                                if let Some(root) = &props.gallery.config.root_url {
                                    {write_structured_data(BreadcrumbList{
                                        _type: "BreadcrumbList",
                                        item_list_element: breadcrumbs.into_iter().skip(1).map(|mut b| {
                                            // Don't do Home.
                                            b.position -= 1;
                                            b.item = b.item.map(|item| format!("{root}{item}"));
                                            b
                                        }).collect(),
                                    })}
                                }
                            }
                        </nav>
                    </section>
                    <section id="main_and_sidebar">
                        <main id="page_main_body">
                            {props.body.clone()}
                        </main>
                        <section id="sidebar_section" data-nosnippet={"nosnippet"}>
                            <aside id="sidebar">
                                if !props.pages.is_empty() {
                                    <div class="sidebar_panel">
                                        <h2 class="sidebar_panel_heading">{"Pages"}</h2>
                                        <ul class="sidebar_panel_list">
                                            {props.pages.iter().map(|(href, page)| html!{
                                                <li class="sidebar_panel_list_item">
                                                    <a
                                                        class="sidebar_panel_list_link"
                                                        href={href.clone()}
                                                    >{page.name.clone()}</a>
                                                </li>
                                            }).collect::<Html>()}
                                        </ul>
                                    </div>
                                }
                                {props.sidebar.clone()}
                            </aside>
                        </section>
                    </section>
                    <section id="footer_section" data-nosnippet={"nosnippet"}>
                        <footer id="footer">
                            {join(&props.gallery.config.author.as_ref().map(|author| {
                                {html!{<>
                                    {"Published by "}
                                    if let Some(href) = props.gallery.config.author_url.clone() {
                                        <a {href} target="_blank">{author}</a>
                                    } else {
                                        {author}
                                    }
                                </>}}
                            }).into_iter()
                                .chain(std::iter::once(html!{<>
                                    {"Powered by "}
                                    <a
                                        href="https://github.com/finnbear/chillphoto"
                                        target="_blank"
                                    >{"chillphoto"}</a>
                                </>}))
                            .collect::<Vec<_>>(), &html!{{" | "}})}
                        </footer>
                    </section>
                </div>
            </body>
        </html>
    }
}

// TODO: wait for `slice_concat_ext` stabilization.
fn join<T: Clone>(slice: &[T], sep: &T) -> Vec<T> {
    let mut iter = slice.iter();
    let first = match iter.next() {
        Some(first) => first,
        None => return vec![],
    };
    let size = slice.len() * 2 - 1;
    let mut result = Vec::with_capacity(size);
    result.extend_from_slice(std::slice::from_ref(first));

    for v in iter {
        result.push(sep.clone());
        result.extend_from_slice(std::slice::from_ref(v))
    }
    result
}

pub fn write_image(img: &RgbImage, path: &str) -> Vec<u8> {
    let mut ret = Cursor::new(Vec::new());
    img.write_to(&mut ret, ImageFormat::from_path(path).unwrap())
        .unwrap();
    ret.into_inner()
}

pub fn rich_text_html(text: &RichText) -> Html {
    match text.format {
        RichTextFormat::PlainText => text
            .content
            .lines()
            .map(|line| {
                html! {<>
                    {line}
                    <br/>
                </>}
            })
            .collect(),
        RichTextFormat::Markdown => {
            let mut options = markdown::Options::gfm();
            options.compile.allow_dangerous_html = true;
            Html::from_html_unchecked(
                markdown::to_html_with_options(&text.content, &options)
                    .unwrap()
                    .into(),
            )
        }
        RichTextFormat::Html => Html::from_html_unchecked(text.content.clone().into()),
    }
}

fn write_manifest(gallery: &Gallery) -> Vec<u8> {
    #[derive(Serialize)]
    struct Manifest {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        display: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        categories: Vec<String>,
        start_url: String,
        handle_links: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        icons: Vec<Icon>,
    }

    #[derive(Serialize)]
    struct Icon {
        src: String,
        sizes: String,
    }

    let manifest = Manifest {
        name: gallery.config.title.clone(),
        description: gallery.config.description.clone(),
        display: "standalone".to_owned(),
        categories: gallery.config.categories.clone(),
        start_url: "/".to_owned(),
        handle_links: "not-preferred".to_owned(),
        icons: gallery
            .thumbnail()
            .map(|(_, _)| Icon {
                src: "/manifest.png".to_owned(),
                sizes: format!("{}x{}", MANIFEST_ICON_RESOLUTION, MANIFEST_ICON_RESOLUTION),
            })
            .into_iter()
            .collect::<Vec<_>>(),
    };

    serde_json::to_string(&manifest).unwrap().into_bytes()
}

pub fn write_structured_data<T: Serialize>(data: T) -> Html {
    #[derive(Serialize)]
    struct Context<T> {
        #[serde(rename = "@context")]
        context: &'static str,
        #[serde(flatten)]
        data: T,
    }

    Html::from_html_unchecked(
        format!(
            "<script type=\"application/ld+json\">\n{}\n</script>",
            serde_json::to_string_pretty(&Context {
                context: "https://schema.org",
                data,
            })
            .unwrap()
        )
        .into(),
    )
}

/// https://developer.chrome.com/docs/web-platform/prerender-pages
pub fn write_speculation_rules(urls: Vec<String>) -> Html {
    #[derive(Serialize)]
    struct SpeculationRules {
        prerender: Vec<Rule>,
    }

    #[derive(Serialize)]
    #[serde(untagged)]
    enum Rule {
        List {
            urls: Vec<String>,
            eagerness: String,
        },
        Document {
            #[serde(rename = "where")]
            _where: Expr,
            eagerness: String,
        },
    }

    #[derive(Serialize)]
    enum Expr {
        #[serde(rename = "and")]
        And(Vec<Expr>),
        #[serde(rename = "href_matches")]
        HrefMatches(String),
        #[serde(rename = "not")]
        Not(Box<Expr>),
        #[serde(rename = "selector_matches")]
        SelectorMatches(String),
    }

    Html::from_html_unchecked(
        format!(
            "<script type=\"speculationrules\">\n{}\n</script>",
            serde_json::to_string_pretty(&SpeculationRules {
                prerender: (!urls.is_empty())
                    .then_some(Rule::List {
                        urls,
                        eagerness: "immediate".to_owned(),
                    })
                    .into_iter()
                    .chain(std::iter::once(Rule::Document {
                        _where: Expr::And(vec![
                            Expr::HrefMatches("/*".to_owned()),
                            Expr::Not(Box::new(Expr::SelectorMatches(
                                "[rel~=nofollow]".to_owned()
                            ))),
                        ]),
                        eagerness: "moderate".to_owned()
                    }))
                    .collect()
            })
            .unwrap()
        )
        .into(),
    )
}

/// https://schema.org/Person
#[derive(Clone, Serialize)]
struct PersonStructuredData {
    #[serde(rename = "@type")]
    _type: &'static str,
    name: String,
}

/// https://schema.org/ImageObject
#[derive(Clone, Serialize)]
struct PhotoStructuredData {
    #[serde(rename = "@type")]
    _type: &'static str,
    #[serde(rename = "contentUrl")]
    content_url: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "dateCreated", skip_serializing_if = "Option::is_none")]
    date_created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator: Option<PersonStructuredData>,
    #[serde(rename = "copyrightHolder", skip_serializing_if = "Option::is_none")]
    copyright_holder: Option<PersonStructuredData>,
    #[serde(rename = "copyrightYear", skip_serializing_if = "Option::is_none")]
    copyright_year: Option<i32>,
    #[serde(rename = "creditText", skip_serializing_if = "Option::is_none")]
    credit_text: Option<String>,
    #[serde(rename = "copyrightNotice", skip_serializing_if = "Option::is_none")]
    copyright_notice: Option<String>,
    #[serde(rename = "acquireLicensePage", skip_serializing_if = "Option::is_none")]
    acquire_license_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail: Option<Box<PhotoStructuredData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "representativeOfPage", skip_serializing_if = "is_false")]
    representative_of_page: bool,
    width: u32,
    height: u32,
    #[serde(rename = "locationCreated")]
    location_created: Option<PlaceStructuredData>,
    #[serde(rename = "contentLocation")]
    content_location: Option<PlaceStructuredData>,
}

/// https://schema.org/Place
#[derive(Clone, Serialize)]
struct PlaceStructuredData {
    #[serde(rename = "@type")]
    _type: &'static str,
    name: String,
}

fn is_false(b: &bool) -> bool {
    !b
}

fn photo_structured_data(
    gallery: &Gallery,
    photo: &Photo,
    html_url: String,
    content_url: String,
    thumbnail_url: Option<String>,
    representative_of_page: bool,
) -> PhotoStructuredData {
    let author = photo
        .config
        .author
        .as_ref()
        .or(gallery.config.author.as_ref());
    let license = photo
        .config
        .license_url
        .as_ref()
        .or(gallery.config.license_url.as_ref());
    let location = photo
        .config
        .location
        .as_ref()
        .map(|name| PlaceStructuredData {
            _type: "Place",
            name: name.clone(),
        });

    let author_person = author.cloned().map(|name| PersonStructuredData {
        _type: "Person",
        name,
    });
    let copyright_year = photo.date_time().map(|d| d.year());
    let (width, height) = if thumbnail_url.is_some() {
        photo.image_dimensions(&gallery.config)
    } else {
        (
            gallery.config.thumbnail_resolution,
            gallery.config.thumbnail_resolution,
        )
    };
    PhotoStructuredData {
        _type: "ImageObject",
        content_url,
        name: photo.name.clone(),
        description: photo.config.description.clone(),
        date_created: photo.date_time().map(|d| d.date().to_string()),
        creator: author_person.clone(),
        copyright_holder: author_person.clone(),
        copyright_year,
        copyright_notice: match (author.as_ref(), copyright_year) {
            (Some(a), Some(y)) => Some(format!("© {y} {a}")),
            (Some(a), None) => Some(format!("© {a}")),
            (None, Some(y)) => Some(format!("© {y}")),
            (None, None) => None,
        },
        credit_text: author.cloned(),
        license: license.cloned(),
        acquire_license_page: gallery.config.acquire_license_url.clone(),
        thumbnail: thumbnail_url.map(|content_url| {
            Box::new(photo_structured_data(
                gallery,
                photo,
                html_url.clone(),
                content_url,
                None,
                false,
            ))
        }),
        url: gallery
            .config
            .root_url
            .as_ref()
            .map(|root| format!("{root}{html_url}")),
        representative_of_page,
        width,
        height,
        location_created: location.clone(),
        content_location: location,
    }
}

#[derive(Serialize)]
struct BreadcrumbList {
    #[serde(rename = "@type")]
    _type: &'static str,
    #[serde(rename = "itemListElement")]
    item_list_element: Vec<BreadcrumbListElement>,
}

#[derive(Serialize)]
struct BreadcrumbListElement {
    #[serde(rename = "@type")]
    _type: &'static str,
    position: usize,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    item: Option<String>,
}
