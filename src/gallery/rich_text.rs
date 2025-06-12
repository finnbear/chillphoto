#[derive(Debug, Clone, PartialEq)]
pub struct RichText {
    pub content: String,
    pub format: RichTextFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichTextFormat {
    PlainText,
    Markdown,
    Html,
}
