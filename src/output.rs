use crate::{
    config::{Config, OutputConfig},
    gallery::{Gallery, Item},
    photo::Photo,
    util::{add_trailing_slash_if_nonempty, remove_dir_contents},
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{fs, sync::Arc};
use yew::{
    function_component, html, AttrValue, Html, LocalServerRenderer, Properties, ServerRenderer,
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
                        title: photo.name.clone().into(),
                        head: Default::default(),
                        body: html! {
                            <a href={config.output.photo::<true>(&path, &photo.name)}>
                                <img src={config.output.preview::<true>(&path, &photo.name)}/>
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
                        title: category.name.clone().into(),
                        head: Default::default(),
                        body: render_items(&category_path, &category.children, &config),
                    },
                    &config.output.category_html::<false>(&path, &category.name),
                )
            }
        });

        render_html(
            AppProps {
                title: "Gallery".into(),
                head: Default::default(),
                body: render_items("", &self.children, &config),
            },
            &config.output.index_html::<false>(),
        )
    }
}

fn render_items(category_path: &str, items: &[Item], config: &Config) -> Html {
    html! {
        (items.iter().map(|child| {
            match child {
                Item::Photo(photo) => {
                    html!{
                        <a href={config.output.photo_html::<true>(&category_path, &photo.name)}>
                            <img src={config.output.thumbnail::<true>(&category_path, &photo.name)}/>
                        </a>
                    }
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
                            println!("{path:?}");
                            representative = Some((path.join("/"), photo.clone()));
                        }
                    });
                    let (photo_path, photo) = representative.unwrap();
                    println!("{photo_path}");
                    html!{
                        <a href={config.output.category_html::<true>(&category_path, &category.name)}>
                            <img src={config.output.thumbnail::<true>(&photo_path, &photo.name)}/>
                        </a>
                    }
                }
            }
        }).collect::<Html>())
    }
}

fn render_html(props: AppProps, path: &str) {
    use std::ops::Deref;
    let renderer = LocalServerRenderer::<App>::with_props(props).hydratable(false);
    let html = futures::executor::block_on(renderer.render());
    fs::write(path, html).expect(path);
}

#[derive(Properties, PartialEq)]
pub struct AppProps {
    #[prop_or("chillphoto".into())]
    pub title: AttrValue,
    #[prop_or_default]
    pub head: Html,
    pub body: Html,
}

#[function_component(App)]
pub fn app(props: &AppProps) -> Html {
    html! {
        <html>
            <head>
                <title>{props.title.clone()}</title>
                {props.head.clone()}
            </head>
            <body>
                {props.body.clone()}
            </body>
        </html>
    }
}

/*
#[function_component(Gallery)]
pub fn gallery() -> Html {

}
*/
