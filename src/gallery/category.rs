use chrono::NaiveDate;

use crate::gallery::{visit_children_items, CategoryConfig, CategoryPath, Item, Photo, RichText};

#[derive(Debug)]
pub struct Category {
    pub name: String,
    pub text: Option<RichText>,
    pub children: Vec<Item>,
    pub config: CategoryConfig,
    /// foo/bar baz/Quxx
    pub src_key: String,
}

impl Category {
    pub fn slug(&self) -> String {
        self.config
            .slug
            .clone()
            .unwrap_or_else(|| self.name.to_lowercase().replace(' ', "-"))
    }

    pub fn thumbnail(&self, path: &CategoryPath) -> Option<(CategoryPath, &Photo)> {
        let mut ret = Option::<(CategoryPath, &Photo)>::None;
        self.visit_items(&path, |path, item| match item {
            Item::Photo(photo) => {
                if ret.is_none() || self.config.thumbnail.as_ref() == Some(&photo.name) {
                    ret = Some((path.clone(), photo));
                }
            }
            Item::Category(category) => {
                if ret.is_none() || self.config.thumbnail.as_ref() == Some(&category.name) {
                    ret = category.thumbnail(path);
                }
            }
            _ => {}
        });
        ret
    }

    pub fn first_and_last_dates(&self) -> Option<(NaiveDate, NaiveDate)> {
        let mut first_date = Option::<NaiveDate>::None;
        let mut last_date = Option::<NaiveDate>::None;

        self.visit_items(&CategoryPath::ROOT, |_, child| {
            if let Item::Photo(photo) = child {
                if let Some(date_time) = photo.date_time() {
                    let date = date_time.date();
                    if first_date.map(|fd| date < fd).unwrap_or(true) {
                        first_date = Some(date);
                    }
                    if last_date.map(|fd| date > fd).unwrap_or(true) {
                        last_date = Some(date);
                    }
                }
            }
        });

        first_date.zip(last_date)
    }

    pub fn visit_items<'a>(
        &'a self,
        path: &CategoryPath,
        mut visitor: impl FnMut(&CategoryPath, &'a Item),
    ) {
        let path = path.push(self.slug());
        visit_children_items(&path, &self.children, &mut visitor);
    }
}
