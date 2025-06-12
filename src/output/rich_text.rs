use crate::gallery::{RichText, RichTextFormat};
use yew::{html, Html};

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
        RichTextFormat::Markdown => {
            let mut options = markdown::Options::gfm();
            options.compile.allow_dangerous_html = true;
            Html::from_html_unchecked(
                markdown::to_html_with_options(&text.content, &options)
                    .unwrap()
                    .into(),
            )
        }
        RichTextFormat::Html => Html::from_html_unchecked(text.content.clone().into()),
    }
}
