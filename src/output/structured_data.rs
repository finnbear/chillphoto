use crate::{
    gallery::{Gallery, Photo},
    output::copyright_notice,
};
use chrono::Datelike;
use serde::Serialize;
use yew::Html;

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
pub struct PersonStructuredData {
    #[serde(rename = "@type")]
    pub _type: &'static str,
    pub name: String,
}

/// https://schema.org/ImageObject
#[derive(Clone, Serialize)]
pub struct PhotoStructuredData {
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
pub struct PlaceStructuredData {
    #[serde(rename = "@type")]
    _type: &'static str,
    name: String,
}

fn is_false(b: &bool) -> bool {
    !b
}

pub fn photo_structured_data(
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
        name: photo.output_name().to_owned(),
        description: photo.config.description.clone(),
        date_created: photo.date_time().map(|d| d.date().to_string()),
        creator: author_person.clone(),
        copyright_holder: author_person.clone(),
        copyright_year,
        copyright_notice: copyright_notice(author.map(|a| a.as_str()), copyright_year),
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
pub struct BreadcrumbList {
    #[serde(rename = "@type")]
    pub _type: &'static str,
    #[serde(rename = "itemListElement")]
    pub item_list_element: Vec<BreadcrumbListElement>,
}

#[derive(Serialize)]
pub struct BreadcrumbListElement {
    #[serde(rename = "@type")]
    pub _type: &'static str,
    pub position: usize,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
}

#[derive(Serialize)]
pub struct SearchActionStructuredData {
    #[serde(rename = "@type")]
    pub _type: &'static str,
    pub target: String,
    pub query: &'static str,
    #[serde(rename = "query-input")]
    pub query_input: &'static str,
}

/// https://schema.org/WebSite
#[derive(Serialize)]
pub struct WebSiteStructuredData {
    #[serde(rename = "@type")]
    pub _type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub name: String,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "copyrightHolder", skip_serializing_if = "Option::is_none")]
    pub copyright_holder: Option<PersonStructuredData>,
    #[serde(rename = "potentialAction")]
    pub potential_action: Option<SearchActionStructuredData>,
}
