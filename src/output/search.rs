use chrono::Datelike;
use serde::Serialize;
use yew::{html, Html};

use crate::gallery::{Gallery, Item};

pub fn render_search(gallery: &Gallery) -> Html {
    let mut items = Vec::<SearchItem>::new();

    gallery.visit_items(|path, item| {
        let photo = if let Item::Photo(photo) = item {
            photo
        } else {
            return;
        };

        items.push(SearchItem {
            categories: path
                .iter_paths()
                .filter_map(|p| gallery.category(&p))
                .map(|c| c.name.clone())
                .collect(),
            name: photo.output_name().to_string(),
            page_text_content: photo.text.as_ref().map(|t| t.content.clone()),
            location: photo.config.location.clone(),
            description: photo.config.description.clone(),
            path: gallery.config.photo_html::<true>(&path, &photo.slug()),
            thumbnail_path: gallery.config.thumbnail::<true>(&path, &photo.slug()),
            month: photo.date_time().map(|d| d.date().format("%B").to_string()),
            year: photo.date_time().map(|d| d.date().year_ce().1.to_string()),
        });
    });

    let json = serde_json::to_string(&items).unwrap();
    let search_script_template = include_str!("search.js");

    // curl https://cdn.jsdelivr.net/npm/@leeoniya/ufuzzy@1.0.18/dist/uFuzzy.iife.min.js | openssl dgst -sha384 -binary | openssl base64 -A
    /*
    <script
        src="https://cdn.jsdelivr.net/npm/@leeoniya/ufuzzy@1.0.18/dist/uFuzzy.iife.min.js"
        integrity="sha384-iJi701vti4rlEmxekv29ZaHh9sTi0fLz/eImPLSbsifWyR6c/WIStzTEEz0UGdAj
    ></script>
    */
    let script = Html::from_html_unchecked(
        format!(
            r#"
        <script>
            const INPUT_ITEMS = {json};
            {search_script_template}
        </script>
    "#
        )
        .into(),
    );

    html! {<>
        <section id="page_main_body_search_results" data-nosnippet="nosnippet">
            // Dynamically replaced by JavaScript
            <p>{"Loading..."}</p>
        </section>
        {script}
    </>}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchItem {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    categories: Vec<String>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_text_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    path: String,
    thumbnail_path: String,
    month: Option<String>,
    year: Option<String>,
}
