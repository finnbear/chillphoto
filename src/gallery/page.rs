use crate::gallery::{PageConfig, RichText};

#[derive(Debug)]
pub struct Page {
    pub name: String,
    pub text: RichText,
    pub config: PageConfig,
    pub src_key: String,
}

impl Page {
    pub fn slug(&self) -> String {
        self.config
            .slug
            .clone()
            .unwrap_or_else(|| self.name.to_lowercase().replace(' ', "-"))
    }
}
