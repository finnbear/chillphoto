use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub struct RichText {
    pub content: String,
    pub format: RichTextFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RichTextFormat {
    PlainText,
    Markdown,
    Html,
}
