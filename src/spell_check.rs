use harper_core::{Dialect, linting::{LintGroup, Linter}, Document, FstDictionary};
use crate::RichTextFormat;
use std::sync::Arc;

pub struct SpellChecker {
    dictionary: Arc<FstDictionary>,
    lints: LintGroup,
}

impl Default for SpellChecker {
    fn default() -> Self {
        let dictionary = FstDictionary::curated();
        Self{
            lints: LintGroup::new_curated(dictionary.clone(), Dialect::American),
            dictionary,
        }
    }
}

impl SpellChecker {
    pub fn spell_check(&mut self, text: &str, format: RichTextFormat) {
        let document = Document::new_markdown_default(text, &self.dictionary);
        for lint in self.lints.lint(&document) {
            println!("{lint:?}");
        }
    }
}
