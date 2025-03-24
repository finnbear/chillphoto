use crate::{
    config::{Config, OutputConfig},
    gallery::{Gallery, Item, PageFormat},
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
                                    src={config.output.preview::<true>(&path, &photo.name)}
                                />
                            </a>
                        },
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
                            class="thumbnail_container"
                            href={config.output.category_html::<true>(&category_path, &category.name)}
                        >
                            <img src={config.output.thumbnail::<true>(&photo_path, &photo.name)}/>
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
}

#[function_component(App)]
pub fn app(props: &AppProps) -> Html {
    let style = Html::from_html_unchecked(
        r#"
        body {
            background-color: #222222;
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
            text-decoration: none;
        }

        #main_and_sidebar {
            display: flex;
            flex-direction: row;
        }

        #page_main_body {
            margin: 2rem;
        }

        #sidebar {
        
        }

        #footer {
            text-align: center;
            padding: 0.5rem;
        }

        #footer > a {
            text-decoration: none;
        }

        .thumbnail_container {
            padding: 0.5rem;
            display: inline-block;
            margin: 0.25rem;
            border: 1px solid #e6e6e6;
            background-color: #FBFBF8;
        }

        .preview {
            width: 100%;
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
