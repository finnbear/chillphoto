use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct RichText {
    pub content: String,
    pub format: RichTextFormat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RichTextFormat {
    #[default]
    Markdown,
    PlainText,
    Html,
}

impl RichTextFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::PlainText => "txt",
            Self::Html => "html",
        }
    }
}
