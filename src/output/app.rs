use crate::{
    gallery::{CategoryPath, Gallery, Page, RichText, RichTextFormat},
    output::{
        paginate, rich_text_html, write_speculation_rules, write_structured_data, BreadcrumbList,
        BreadcrumbListElement, PersonStructuredData, RelativeNavigation,
        SearchActionStructuredData, WebSiteStructuredData,
    },
    util::join,
};
use yew::{classes, function_component, html, AttrValue, Html, LocalServerRenderer, Properties};

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
    pub index: bool,
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

        img, summary {
            user-select: none;
        }

        a {
            text-decoration: none;
            color: var(--colored-text-light-background);
        }

        p a, span a, footer a, li a:not(.sidebar_panel_list_link), details a {
            text-decoration: underline;
        }

        #page {
            background-color: white;
            max-width: 60rem;
            margin: 0rem auto;
            display: flex;
            flex-direction: column;
            border-radius: 0.5rem;
            overflow: hidden;
        }

        @media (max-width: 600px) {
            body {
                margin: 0;
            }

            #page {
                border-radius: 0;
                margin: 0;
                max-width: initial;
            }
        }

        #header, #footer {
            background-color: #dadfbb;
        }

        #header {
            display: flex;
            flex-direction: row;
            gap: 1rem;
            align-items: center;
            padding: 2.25rem 2rem;
        }

        #title {
            font-size: 1.5rem;
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
            padding: 0.5rem 2rem;
            color: white;
            display: flex;
            flex-direction: row;
            gap: 0.25rem;
            align-items: center;
            white-space: nowrap;
        }

        .breadcrumb {
            color: var(--colored-text-dark-background);
        }

        .breadcrumb_final {
            color: white;
            font-weight: bold;
        }

        #search_form {
            margin-left: auto;
        }

        @media (max-width: 600px) {
            #search_form {
                display: none;
            }
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

        #page_main_body > :first-child {
            margin-top: 0;
        }

        #page_main_body > :last-child {
            margin-bottom: 0;
        }

        #page_main_body_items, #page_main_body_search_results {
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
            margin-top: 0.25rem;
            font-size: 0.9rem;
        }

        .sidebar_details_panel_text {
            font-size: 0.9rem;
        }

        details.sidebar_details_panel_text > summary {
            margin-left: .2rem;
        }

        details > summary {
            cursor: pointer;
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

        .category_item_dates {
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

        .thumbnail {
            width: 6rem;
            height: 6rem;
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
                let page = paginate(
                    props.gallery.children(&path).unwrap(),
                    props
                        .gallery
                        .category(&path)
                        .map(|c| c.config.items_per_page)
                        .unwrap_or(props.gallery.config.items_per_page),
                )
                .position(|page| {
                    page.items
                        .iter()
                        .any(|i| i.slug() == props.path.iter_segments().nth(path.len()).unwrap())
                })
                .unwrap_or_default(/* for pages */);
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
                        props.gallery.config.index_html::<true>(page)
                    } else {
                        props.gallery.config.category_html::<true>(
                            &path.pop().unwrap(),
                            path.last_segment().unwrap(),
                            page,
                        )
                    }),
                }
            } else {
                BreadcrumbListElement {
                    _type: "ListItem",
                    name: if path.is_root() {
                        "Home".to_owned()
                    } else if path.len() == 1 && path.last_segment() == Some("search") {
                        "Search".to_owned()
                    } else {
                        props.gallery.item_name(&path).to_owned()
                    },
                    position: i + 1,
                    item: None,
                }
            }
        })
        .collect::<Vec<_>>();

    let web_site_structured_data =
        write_structured_data(WebSiteStructuredData {
            _type: "WebSite",
            url: props.gallery.config.root_url.clone(),
            name: props.gallery.config.title.clone(),
            description: props.gallery.config.description.clone(),
            copyright_holder: props.gallery.config.author.clone().map(|name| {
                PersonStructuredData {
                    _type: "Person",
                    name,
                }
            }),
            potential_action: props.gallery.config.root_url.as_ref().map(|root_url| {
                SearchActionStructuredData {
                    _type: "SearchAction",
                    target: format!("{root_url}/search?&query={{query}}"),
                    query: "required",
                }
            }),
        });

    let mut robots_meta = if props.index {
        "index,follow".to_owned()
    } else {
        "noindex".to_owned()
    };

    if props.gallery.config.disallow_ai_training {
        robots_meta.push_str(",DisallowAITraining,noai,noimageai");
    }

    html! {
        <html lang="en">
            <head>
                <meta charset="UTF-8"/>
                <title>{props.title.clone()}</title>
                <meta property="og:title" content={props.title.clone()}/>
                <meta property="og:site_name" content={props.gallery.config.title.clone()}/>
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
                <meta name="robots" content={robots_meta}/>
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
                {web_site_structured_data}
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
                                        aria-disabled={relative.previous.is_none().then_some("true")}
                                    >{"Previous"}</a>
                                    <a
                                        href={relative.next.clone()}
                                        class={classes!(
                                            "relative_navigation_previous",
                                            relative.next.is_none().then_some("relative_navigation_unavailable"),
                                        )}
                                        aria-disabled={relative.next.is_none().then_some("true")}
                                    >{"Next"}</a>
                                </div>
                            }
                        </header>
                        <nav id="breadcrumbs" aria-label="Breadcrumb" data-nosnippet={"nosnippet"}>
                            {join(&breadcrumbs.iter().map(|breadcrumb| html!{
                                if let Some(href) = &breadcrumb.item {
                                    <a
                                        class={"breadcrumb"}
                                        href={href.clone()}
                                    >{breadcrumb.name.clone()}</a>
                                } else {
                                    <span
                                        class={"breadcrumb breadcrumb_final"}
                                        aria-current="page"
                                    >{breadcrumb.name.clone()}</span>
                                }
                            }).collect::<Vec<_>>(), &html!{{"»"}})}
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
                            <form id="search_form" action="/search/" method="get">
                                <input id="search_query" type="search" name="query" minlength={1} aria-label="Search query"/>
                                <button id="search_button" type="submit">{"Search"}</button>
                            </form>
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

// Takes around 10ms.
pub fn render_html(props: AppProps<'_>) -> Vec<u8> {
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
