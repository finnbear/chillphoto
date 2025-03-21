use std::{fs, sync::Arc};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use yew::{function_component, html, AttrValue, Html, LocalServerRenderer, Properties, ServerRenderer};
use crate::{
    config::{Config, OutputConfig},
    gallery::{Gallery, Item},
};

impl Gallery {
    pub fn output(&self, config: &Config) {
        let string = render_html(AppProps { head: Default::default(), body: html!{
            <p>{"Hello"}</p>
        }, title: AttrValue::from("foo") });
        println!("{string}");

        let mut pages = Vec::new();

        self.visit_items(|path, item| {
            let path = path.join("/");
            pages.push((path, item));
        });

        pages.into_par_iter().for_each(|(path, item)| {
            fs::create_dir_all(&config.output.subdirectory(&path))
                .expect("faailed to create item directory");
            match item {
                Item::Photo(photo) => {
                    photo.image.save(config.output.photo(&path, &photo.name)).unwrap();
                    photo
                        .preview
                        .save(config.output.preview(&path, &photo.name))
                        .unwrap();
                    photo
                        .thumbnail
                        .save(config.output.thumbnail(&path, &photo.name))
                        .unwrap();
                }
                Item::Category(category) => {}
            }
        });
    }
}

fn render_html(props: AppProps) -> String {
    use std::ops::Deref;
    let renderer = LocalServerRenderer::<App>::with_props(props).hydratable(false);
    futures::executor::block_on(renderer.render())
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
    html!{
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
