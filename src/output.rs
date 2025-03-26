use crate::{
    category_path::CategoryPath,
    config::{Config, OutputConfig},
    gallery::{Gallery, Item, Page, PageFormat},
    photo::Photo,
    util::{add_trailing_slash_if_nonempty, remove_dir_contents},
    CONFIG,
};
use chrono::NaiveDate;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::borrow::Borrow;
use std::{fs, sync::Arc};
use yew::{
    function_component, html, virtual_dom::VText, AttrValue, Html, LocalServerRenderer, Properties,
    ServerRenderer,
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
                                    src={config.output.preview::<true>(&path, &photo.name)}
                                />
                            </a>
                        },
                        pages: page_items.clone(),
                        path: path.push(photo.name.clone()).clone(),
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
                    },
                    &config.output.page_html::<false>(&path, &page.name),
                )
            }
        });

        render_html(
            AppProps {
                gallery: self.clone(),
                title: "Gallery".into(),
                description: CONFIG.gallery.description.clone().map(|d| d.into()),
                head: Default::default(),
                body: render_items(&CategoryPath::ROOT, &self.children),
                pages: page_items,
                path: CategoryPath::ROOT,
            },
            &CONFIG.output.index_html::<false>(),
        )
    }
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
    let html = markup_fmt::format_text(&html, markup_fmt::Language::Html, &options, |code, _| {
        Ok::<_, std::convert::Infallible>(code.into())
    })
    .unwrap();

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
}

#[function_component(App)]
pub fn app(props: &AppProps) -> Html {
    let style = Html::from_html_unchecked(
        r#"
        body {
            background-color: #222222;
            margin: 2rem;
        }

        a {
            text-decoration: none;
            color: #82996F;
        }

        #page {
            background-color: #fbfbfb;
            max-width: 80rem;
            margin: 0rem auto;
            display: flex;
            flex-direction: column;
            border-radius: 0.5rem;
            overflow: hidden;
        }

        #header, #footer {
            background-color: #dadfbb;
            padding-left: 2rem;
        }

        #title {
            font-weight: normal;
            letter-spacing: 0.1rem;
        }

        #breadcrumbs {
            background-color: #505050;
            padding: 0.5rem;
            padding-left: 2rem;
            color: white;
        }

        .breadcrumb {
            color: #D1E079;
        }

        .breadcrumb_final {
            color: white;
            font-weight: bold;
        }

        #main_and_sidebar {
            display: flex;
            flex-direction: row;
            flex-grow: 1;
        }

        #page_main_body {
            margin: 2rem;
            flex-grow: 1;
        }

        #sidebar {
            width: 20rem;
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

        #footer {
            text-align: center;
            padding: 0.5rem;
        }

        .thumbnail_container {
            padding: 0.5rem;
            display: inline-flex;
            flex-direction: row;
            gap: 0.5rem;
            margin: 0.25rem;
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
            color: #82996F;
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
        <html>
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
                // Favicon
                {props.head.clone()}
                <style>{style}</style>
            </head>
            <body>
                <div id="page">
                    <header id="header">
                        <h1 id="title">{CONFIG.gallery.title.clone()}</h1>
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
                            {author}
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
