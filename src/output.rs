use crate::{
    config::{Config, OutputConfig},
    gallery::{Gallery, Item, Page, PageFormat},
    photo::Photo,
    util::{add_trailing_slash_if_nonempty, remove_dir_contents},
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::borrow::Borrow;
use std::{fs, sync::Arc};
use yew::{
    function_component, html, virtual_dom::VText, AttrValue, Html, LocalServerRenderer, Properties,
    ServerRenderer,
};

impl Gallery {
    pub fn output(&self, config: &Config) {
        if fs::exists(&config.output.path).unwrap() {
            remove_dir_contents(&config.output.path).expect("failed to clear output directory");
        }

        let mut pages = Vec::new();
        let page_items = self
            .children
            .iter()
            .filter_map(|i| i.page().cloned())
            .map(|p| (String::new(), p))
            .collect::<Vec<_>>();

        self.visit_items(|path, item| {
            let path = path.join("/");
            pages.push((path, item.clone()));
        });

        pages.into_par_iter().for_each(|(path, item)| match item {
            Item::Photo(photo) => {
                fs::create_dir_all(&config.output.subdirectory(&path))
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
                        config: config.clone(),
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
                        pages: Vec::new(),
                    },
                    &config.output.photo_html::<false>(&path, &photo.name),
                )
            }
            Item::Category(category) => {
                let category_path =
                    format!("{}{}", add_trailing_slash_if_nonempty(&path), category.name);
                fs::create_dir_all(&config.output.subdirectory(&category_path))
                    .expect("faailed to create item directory");
                render_html(
                    AppProps {
                        config: config.clone(),
                        title: category.name.clone().into(),
                        description: category.description.clone().map(|d| d.into()),
                        head: Default::default(),
                        body: render_items(&category_path, &category.children, &config),
                        pages: page_items.clone(),
                    },
                    &config.output.category_html::<false>(&path, &category.name),
                )
            }
            Item::Page(page) => {
                let body = match page.format {
                    PageFormat::PlainText => Html::VText(VText::new(page.content)),
                    PageFormat::Markdown => Html::from_html_unchecked(
                        markdown::to_html_with_options(&page.content, &markdown::Options::gfm())
                            .unwrap()
                            .into(),
                    ),
                    PageFormat::Html => Html::from_html_unchecked(page.content.into()),
                };

                render_html(
                    AppProps {
                        config: config.clone(),
                        title: page.name.clone().into(),
                        description: None,
                        head: Default::default(),
                        body,
                        pages: page_items.clone(),
                    },
                    &config.output.page_html::<false>(&path, &page.name),
                )
            }
        });

        render_html(
            AppProps {
                title: "Gallery".into(),
                description: config.gallery.description.clone().map(|d| d.into()),
                head: Default::default(),
                body: render_items("", &self.children, &config),
                pages: page_items,
                config: config.clone(),
            },
            &config.output.index_html::<false>(),
        )
    }
}

fn render_items(category_path: &str, items: &[Item], config: &Config) -> Html {
    html! {
        (items.iter().filter_map(|child| {
            match child {
                Item::Photo(photo) => {
                    Some(html!{
                        <a
                            class="thumbnail_container"
                            href={config.output.photo_html::<true>(&category_path, &photo.name)}
                        >
                            <img
                                src={config.output.thumbnail::<true>(&category_path, &photo.name)}
                                style={format!(
                                    "width: {}; height: {};",
                                    config.thumbnail.resolution,
                                    config.thumbnail.resolution
                                )}
                                class="thumbnail"
                            />
                        </a>
                    })
                }
                Item::Category(category) => {
                    let mut representative = Option::<(String, Photo)>::None;
                    let mut category_path_2 = vec![];
                    if !category_path.is_empty() {
                        category_path_2.push(category_path.to_owned());
                    }
                    category.visit_items(&category_path_2, |path, item| {
                        if representative.is_some() {
                            return;
                        }
                        if let Item::Photo(photo) = item {
                            representative = Some((path.join("/"), photo.clone()));
                        }
                    });
                    let (photo_path, photo) = representative.unwrap();
                    Some(html!{
                        <a
                            class="thumbnail_container category_item"
                            href={config.output.category_html::<true>(&category_path, &category.name)}
                        >
                            <img
                                class="thumbnail"
                                style={format!(
                                    "width: {}; height: {};",
                                    config.thumbnail.resolution,
                                    config.thumbnail.resolution
                                )}
                                src={config.output.thumbnail::<true>(&photo_path, &photo.name)}
                            />
                            <div class="category_item_info">
                                <h2 class="category_item_name">
                                    {category.name.clone()}
                                </h2>
                                if let Some(creation_date) = category.creation_date.clone() {
                                    <div class="category_item_creation_date">
                                        {creation_date}
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
    pub config: Config,
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
        }

        a {
            text-decoration: none;
            color: #82996F;
        }

        #page {
            background-color: #fbfbfb;
            margin: 2rem;
            display: flex;
            flex-direction: column;
            border-radius: 0.75rem;
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

        #nav {
            background-color: #505050;
            padding: 0.5rem;
            padding-left: 2rem;
        }

        #nav > a {
            color: white;
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
            padding: 0.5rem;
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
                if let Some(author) = props.config.gallery.author.clone() {
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
                        <h1 id="title">{props.config.gallery.title.clone()}</h1>
                    </header>
                    <nav id="nav">
                        <a href="/">{"Home"}</a>
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
                                        {props.pages.iter().map(|(path, page)| html!{
                                            <li class="sidebar_panel_list_item">
                                                <a class="sidebar_panel_list_link" href="/">{page.name.clone()}</a>
                                            </li>
                                        }).collect::<Html>()}
                                    </ul>
                                </div>
                            }
                        </aside>
                    </div>
                    <footer id="footer">
                    {join(&props.config.gallery.author.as_ref().map(|author| {
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
