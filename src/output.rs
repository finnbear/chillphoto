use crate::{
    category_path::CategoryPath,
    config::{Config, OutputConfig},
    gallery::{Gallery, Item, Page, PageFormat},
    photo::Photo,
    util::{add_trailing_slash_if_nonempty, remove_dir_contents},
    CONFIG,
};
use chrono::NaiveDate;
use core::num;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::borrow::Borrow;
use std::{fs, sync::Arc};
use yew::{
    classes, function_component, html, virtual_dom::VText, AttrValue, Html, LocalServerRenderer,
    Properties, ServerRenderer,
};

impl Gallery {
    pub fn output(&self) {
        let config = &*CONFIG;
        if fs::exists(&config.output.path).unwrap() {
            remove_dir_contents(&config.output.path).expect("failed to clear output directory");
        }

        let mut pages = Vec::new();
        /// TODO: nested pages.
        let page_items = self
            .children
            .iter()
            .filter_map(|i| i.page().cloned())
            .map(|p| {
                (
                    config
                        .output
                        .page_html::<true>(&CategoryPath::ROOT, &p.name),
                    p,
                )
            })
            .collect::<Vec<_>>();

        self.visit_items(|path, item| {
            pages.push((path.clone(), item.clone()));
        });

        pages.into_par_iter().for_each(|(path, item)| match item {
            Item::Photo(photo) => {
                let mut num_photos = 0usize;
                let mut photo_index = Option::<usize>::None;
                let photos = self
                    .category(&path)
                    .unwrap()
                    .children
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
                fs::create_dir_all(
                    &config
                        .output
                        .subdirectory(&path.to_string_without_leading_slash()),
                )
                .expect("faailed to create item directory");
                photo
                    .image
                    .save(config.output.photo::<false>(&path, &photo.name))
                    .unwrap();
                photo
                    .preview
                    .save(config.output.preview::<false>(&path, &photo.name))
                    .unwrap();
                photo
                    .thumbnail
                    .save(config.output.thumbnail::<false>(&path, &photo.name))
                    .unwrap();
                render_html(
                    AppProps {
                        gallery: self.clone(),
                        title: photo.name.clone().into(),
                        description: photo.description.clone().map(|d| d.into()),
                        head: Default::default(),
                        body: html! {
                            <a
                                class="preview_container"
                                href={config.output.photo::<true>(&path, &photo.name)}
                            >
                                <img
                                    class="preview"
                                    width={photo.preview.width().to_string()}
                                    height={photo.preview.height().to_string()}
                                    alt={photo.name.clone()}
                                    src={config.output.preview::<true>(&path, &photo.name)}
                                />
                            </a>
                        },
                        pages: page_items.clone(),
                        path: path.push(photo.name.clone()).clone(),
                        relative: Some(RelativeNavigation {
                            index: photo_index,
                            count: num_photos,
                            previous: photo_index
                                .checked_sub(1)
                                .and_then(|i| photos.get(i))
                                .map(|p| config.output.photo_html::<true>(&path, &p.name)),
                            next: photo_index
                                .checked_add(1)
                                .and_then(|i| photos.get(i))
                                .map(|p| config.output.photo_html::<true>(&path, &p.name)),
                        }),
                    },
                    &config.output.photo_html::<false>(&path, &photo.name),
                )
            }
            Item::Category(category) => {
                let category_path = path.push(category.slug());
                fs::create_dir_all(
                    &config
                        .output
                        .subdirectory(&category_path.to_string_without_leading_slash()),
                )
                .expect("faailed to create item directory");
                render_html(
                    AppProps {
                        gallery: self.clone(),
                        title: category.name.clone().into(),
                        description: category.description.clone().map(|d| d.into()),
                        head: Default::default(),
                        body: render_items(&category_path, &category.children),
                        pages: page_items.clone(),
                        path: category_path.clone(),
                        relative: None,
                    },
                    &config
                        .output
                        .category_html::<false>(&path, &category.slug()),
                )
            }
            Item::Page(page) => {
                let body = match page.format {
                    PageFormat::PlainText => page
                        .content
                        .lines()
                        .map(|line| {
                            html! {<>
                                {line}
                                <br/>
                            </>}
                        })
                        .collect(),
                    PageFormat::Markdown => Html::from_html_unchecked(
                        markdown::to_html_with_options(&page.content, &markdown::Options::gfm())
                            .unwrap()
                            .into(),
                    ),
                    PageFormat::Html => Html::from_html_unchecked(page.content.into()),
                };

                render_html(
                    AppProps {
                        gallery: self.clone(),
                        title: page.name.clone().into(),
                        description: None,
                        head: Default::default(),
                        body,
                        pages: page_items.clone(),
                        path: path.push(page.name.clone()).clone(),
                        relative: None,
                    },
                    &config.output.page_html::<false>(&path, &page.name),
                )
            }
        });

        render_html(
            AppProps {
                gallery: self.clone(),
                title: config.gallery.title.clone().into(),
                description: CONFIG.gallery.description.clone().map(|d| d.into()),
                head: Default::default(),
                body: render_items(&CategoryPath::ROOT, &self.children),
                pages: page_items,
                path: CategoryPath::ROOT,
                relative: None,
            },
            &CONFIG.output.index_html::<false>(),
        )
    }
}

#[derive(PartialEq)]
pub struct RelativeNavigation {
    index: usize,
    count: usize,
    previous: Option<String>,
    next: Option<String>,
}

fn render_items(category_path: &CategoryPath, items: &[Item]) -> Html {
    html! {
        (items.iter().filter_map(|child| {
            match child {
                Item::Photo(photo) => {
                    Some(html!{
                        <a
                            class="thumbnail_container"
                            href={CONFIG.output.photo_html::<true>(&category_path, &photo.name)}
                        >
                            <img
                                alt={photo.name.clone()}
                                src={CONFIG.output.thumbnail::<true>(&category_path, &photo.name)}
                                style={format!(
                                    "width: {}; height: {};",
                                    CONFIG.thumbnail.resolution,
                                    CONFIG.thumbnail.resolution
                                )}
                                class="thumbnail"
                            />
                        </a>
                    })
                }
                Item::Category(category) => {
                    let mut representative = Option::<(CategoryPath, Photo)>::None;
                    category.visit_items(&category_path, |path, item| {
                        if representative.is_some() {
                            return;
                        }
                        if let Item::Photo(photo) = item {
                            representative = Some((path.clone(), photo.clone()));
                        }
                    });
                    let (photo_path, photo) = representative.unwrap();
                    Some(html!{
                        <a
                            class="thumbnail_container category_item"
                            href={CONFIG.output.category_html::<true>(&category_path, &category.slug())}
                        >
                            <img
                                class="thumbnail"
                                style={format!(
                                    "width: {}; height: {};",
                                    CONFIG.thumbnail.resolution,
                                    CONFIG.thumbnail.resolution
                                )}
                                alt={photo.name.clone()}
                                src={CONFIG.output.thumbnail::<true>(&photo_path, &photo.name)}
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
                                if let Some(description) = category.description.clone() {
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
        }).collect::<Html>())
    }
}

fn render_html(props: AppProps, path: &str) {
    use std::ops::Deref;
    let renderer = LocalServerRenderer::<App>::with_props(props).hydratable(false);
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

    fs::write(path, html).expect(path);
}

#[derive(Properties, PartialEq)]
pub struct AppProps {
    pub gallery: Gallery,
    pub path: CategoryPath,
    #[prop_or("chillphoto".into())]
    pub title: AttrValue,
    #[prop_or(None)]
    pub description: Option<AttrValue>,
    #[prop_or_default]
    pub head: Html,
    pub body: Html,
    pub pages: Vec<(String, Page)>,
    pub relative: Option<RelativeNavigation>,
}

#[function_component(App)]
pub fn app(props: &AppProps) -> Html {
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
            background-color: #fbfbfb;
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
        }

        #sidebar {
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
            color: black;
            font-size: 0.5rem;
        }

        .preview {
            width: 100%;
            height: auto;
        }

        .thumbnail, .preview {
            background-color: #282828;
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
                if let Some(author) = CONFIG.gallery.author.clone() {
                    <meta name="author" content={author}/>
                }
                <meta name="generator" content="chillphoto"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                // Favicon
                {props.head.clone()}
                <style>{style}</style>
            </head>
            <body>
                <div id="page">
                    <header id="header">
                        <h1 id="title">{CONFIG.gallery.title.clone()}</h1>
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
                                    href={path.to_string_with_leading_slash()}
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
                        {join(&CONFIG.gallery.author.as_ref().map(|author| {
                            {html!{<>
                                {"Published by "}
                                if let Some(href) = CONFIG.gallery.author_url.clone() {
                                    <a {href}>{author}</a>
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
