use crate::{
    category_path::CategoryPath,
    gallery::{Gallery, Item, Page, RichText, RichTextFormat},
    photo::Photo,
};
use chrono::Datelike;
use image::{ImageFormat, RgbImage};
use serde::Serialize;
use std::{collections::HashMap, fmt::Write, io::Cursor, sync::LazyLock};
use yew::{classes, function_component, html, AttrValue, Html, LocalServerRenderer, Properties};

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

                    let page_items = page_items.clone();
                    ret.insert(
                        config.photo_html::<false>(&path, &photo.name),
                        LazyLock::new(Box::new(move || {
                            /// https://schema.org/Person
                            #[derive(Serialize)]
                            struct PersonStructuredData {
                                #[serde(rename = "@type")]
                                _type: &'static str,
                                name: String,
                            }

                            /// https://schema.org/ImageObject
                            #[derive(Serialize)]
                            struct PhotoStructuredData {
                                #[serde(rename = "@context")]
                                context: &'static str,
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
                                copyright_holder: Option<String>,
                                #[serde(rename = "copyrightYear", skip_serializing_if = "Option::is_none")]
                                copyright_year: Option<i32>,
                                #[serde(skip_serializing_if = "Option::is_none")]
                                license: Option<String>,
                            }

                            let author = photo.config.author.as_ref().or(self.config.author.as_ref());
                            let license = photo.config.license_url.as_ref().or(self.config.license_url.as_ref());

                            let photo_structured_data = [self.config.thumbnail::<true>(&path, &photo.name), self.config.preview::<true>(&path, &photo.name), self.config.photo::<true>(&path, &photo.name)].into_iter().map(|content_url| {
                                write_structured_data(PhotoStructuredData{
                                    context: "https://schema.org",
                                    _type: "ImageObject",
                                    content_url,
                                    name: photo.name.clone(),
                                    description: photo.config.alt_text.clone(),
                                    date_created: photo.date().map(|d| d.to_string()),
                                    creator: author.cloned().map(|name| PersonStructuredData{
                                        _type: "Person",
                                        name,
                                    }),
                                    copyright_holder: author.cloned(),
                                    copyright_year: photo.date().map(|d| d.year()),
                                    license: license.cloned(),
                                })
                            }).collect::<Html>();

                            render_html(AppProps {
                                gallery: self,
                                title: photo.name.clone().into(),
                                description: photo.config.alt_text.clone().map(|s| s.into()),
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
                                            alt={photo.config.alt_text.as_ref().unwrap_or(&photo.name).clone()}
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
                                gallery: self,
                                title: category.name.clone().into(),
                                description: category.config.description.clone().map(|d| d.into()),
                                head: Default::default(),
                                body: html!{<>
                                    {render_items(self, &category_path, &category.children)}
                                    if let Some(text) = &category.text {
                                        <br/>
                                        {rich_text_html(text)}
                                    }
                                </>},
                                pages: page_items,
                                path: category_path.clone(),
                                relative: None,
                            })
                        })),
                    );
                }
                Item::Page(page) => {
                    let page_items = page_items.clone();
                    ret.insert(
                        config.page_html::<false>(&path, &page.name),
                        LazyLock::new(Box::new(move || {
                            render_html(AppProps {
                                gallery: self,
                                title: page.name.clone().into(),
                                description: None,
                                head: Default::default(),
                                body: rich_text_html(&page.text),
                                pages: page_items.clone(),
                                path: path.push(page.name.clone()).clone(),
                                relative: None,
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

        ret.insert(
            self.config.index_html::<false>(),
            LazyLock::new(Box::new(move || {
                /// https://schema.org/WebSite
                #[derive(Serialize)]
                struct WebSiteStructuredData {
                    #[serde(rename = "@context")]
                    context: &'static str,
                    #[serde(rename = "@type")]
                    _type: &'static str,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    url: Option<String>,
                    name: String,
                    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
                    description: Option<String>,
                    #[serde(rename = "copyrightHolder", skip_serializing_if = "Option::is_none")]
                    copyright_holder: Option<String>,
                }

                let website_structured_data = write_structured_data(WebSiteStructuredData {
                    context: "https://schema.org",
                    _type: "WebSite",
                    url: self.config.root_url.clone(),
                    name: self.config.title.clone(),
                    description: self.config.description.clone(),
                    copyright_holder: self.config.author.clone(),
                });

                render_html(AppProps {
                    gallery: self,
                    title: self.config.title.clone().into(),
                    description: self.config.description.clone().map(|d| d.into()),
                    head: website_structured_data,
                    body: render_items(self, &CategoryPath::ROOT, &self.children),
                    pages: page_items,
                    path: CategoryPath::ROOT,
                    relative: None,
                })
            })),
        );

        if let Some(root_url) = &self.config.root_url {
            let mut pages = ret
                .keys()
                .filter(|k| k.ends_with('/') || k.ends_with(".html"))
                .map(|s| format!("{root_url}{}", s.trim_end_matches("index.html")))
                .collect::<Vec<_>>();
            pages.sort();
            let sitemap = pages.join("\n");
            ret.insert(
                "/sitemap.txt".to_owned(),
                LazyLock::new(Box::new(move || sitemap.into_bytes())),
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
                    writeln!(robots_txt, "Sitemap: {url}/sitemap.txt").unwrap();
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
        {items.iter().filter_map(|child| {
            match child {
                Item::Photo(photo) => {
                    Some(html!{
                        <a
                            class="thumbnail_container"
                            href={gallery.config.photo_html::<true>(&category_path, &photo.name)}
                        >
                            <img
                                title={photo.name.clone()}
                                alt={photo.config.alt_text.as_ref().unwrap_or(&photo.name).clone()}
                                src={gallery.config.thumbnail::<true>(&category_path, &photo.name)}
                                style={format!(
                                    "width: {}px; height: {}px;",
                                    gallery.config.thumbnail_resolution,
                                    gallery.config.thumbnail_resolution
                                )}
                                class="thumbnail"
                            />
                        </a>
                    })
                }
                Item::Category(category) => {
                    let mut representative = Option::<(CategoryPath, &Photo)>::None;
                    category.visit_items(&category_path, |path, item| {
                        if let Item::Photo(photo) = item {
                            if representative.is_none() || category.config.thumbnail.as_ref() == Some(&photo.name) {
                                representative = Some((path.clone(), photo));
                            }
                        }
                    });
                    let (photo_path, photo) = representative?;
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
                                src={gallery.config.thumbnail::<true>(&photo_path, &photo.name)}
                            />
                            <div class="category_item_info">
                                <h2 class="category_item_name">
                                    {category.name.clone()}
                                </h2>
                                if let Some(creation_date) = category.creation_date.clone() {
                                    <div class="category_item_creation_date">
                                        {format!("{}", creation_date.format("%-d %b, %C%y"))}
                                    </div>
                                }
                                if let Some(description) = category.config.description.clone() {
                                    <div class="category_item_description">
                                        {description}
                                    </div>
                                }
                            </div>
                        </a>
                    })
                }
                Item::Page(_) => None
            }
        }).collect::<Html>()}
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

    html.into_bytes()
}

pub struct AppProps<'a> {
    pub gallery: &'a Gallery,
    pub path: CategoryPath,
    pub title: AttrValue,
    pub description: Option<AttrValue>,
    pub head: Html,
    pub body: Html,
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
            margin: 2rem;
            flex-grow: 1;
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

    html! {
        <html lang="en">
            <head>
                <meta charset="UTF-8"/>
                <title>{props.title.clone()}</title>
                if let Some(description) = props.description.clone() {
                    <meta name="description" content={description}/>
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
                    <meta name="robots" content="DisallowAITraining"/>
                }
                <link rel="manifest" href="/manifest.json"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                if let Some(relative) = &props.relative {
                    if let Some(previous) = &relative.previous {
                        <link rel="prerender" href={previous.clone()}/>
                    }
                    if let Some(next) = &relative.next {
                        <link rel="prerender" href={next.clone()}/>
                    }
                }
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
                    <nav id="breadcrumbs">
                        {join(&props.path.iter_paths().map(|path| if path != props.path {
                            html!{
                                <a
                                    class={"breadcrumb"}
                                    href={if path.is_root() {
                                        props.gallery.config.index_html::<true>()
                                    } else {
                                        props.gallery.config.category_html::<true>(&path.pop().unwrap(), path.last_segment().unwrap())
                                    }}
                                >{props.gallery.category(&path).map(|c| c.name.as_str()).unwrap_or("Home").to_owned()}</a>
                            }
                        } else {
                            html!{
                                <span
                                    class={"breadcrumb breadcrumb_final"}
                                >{props.gallery.category(&path).map(|c| c.name.as_str()).or(path.last_segment()).unwrap_or("Home").to_owned()}</span>
                            }
                        }).collect::<Vec<_>>(), &html!{{" » "}})}
                        if let Some(relative) = &props.relative {
                            {format!(" ({}/{})", relative.index + 1, relative.count)}
                        }
                    </nav>
                    <div id="main_and_sidebar">
                        <main id="page_main_body">
                            {props.body.clone()}
                        </main>
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
                        </aside>
                    </div>
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
        RichTextFormat::Markdown => Html::from_html_unchecked(
            markdown::to_html_with_options(&text.content, &markdown::Options::gfm())
                .unwrap()
                .into(),
        ),
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
    }

    let manifest = Manifest {
        name: gallery.config.title.clone(),
        description: gallery.config.description.clone(),
        display: "standalone".to_owned(),
        categories: gallery.config.categories.clone(),
        start_url: "/".to_owned(),
        handle_links: "not-preferred".to_owned(),
    };

    serde_json::to_string(&manifest).unwrap().into_bytes()
}

pub fn write_structured_data(data: impl Serialize) -> Html {
    Html::from_html_unchecked(
        format!(
            r#"
        <script type="application/ld+json">
        {}
        </script>
    "#,
            serde_json::to_string_pretty(&data).unwrap()
        )
        .into(),
    )
}
