use std::sync::OnceLock;

use crate::{
    category_path::CategoryPath,
    config::{CategoryConfig, GalleryConfig, PageConfig},
    photo::Photo,
    static_file::StaticFile,
};
use chrono::NaiveDate;
use image::RgbImage;

#[derive(Debug)]
pub struct Gallery {
    pub children: Vec<Item>,
    pub favicon: Option<(Vec<u8>, OnceLock<RgbImage>)>,
    pub config: GalleryConfig,
    pub head_html: Option<String>,
    pub home_text: Option<RichText>,
    pub static_files: Vec<StaticFile>,
}

impl Gallery {
    pub fn favicon(&self) -> Option<&RgbImage> {
        self.favicon.as_ref().map(|(input, output)| {
            output.get_or_init(|| {
                image::load_from_memory(input)
                    .expect("failed to load favicon")
                    .to_rgb8()
            })
        })
    }

    pub fn thumbnail(&self) -> Option<(CategoryPath, &Photo)> {
        let mut ret = Option::<(CategoryPath, &Photo)>::None;
        self.visit_items(|path, item| {
            if ret.is_some() {
                return;
            }
            if let Some(category) = item.category() {
                ret = category.thumbnail(path);
            }
        });
        ret
    }

    pub fn children(&self, path: &CategoryPath) -> Option<&Vec<Item>> {
        if path.is_root() {
            return Some(&self.children);
        }
        self.category(path).map(|c| &c.children)
    }

    pub fn item_name(&self, path: &CategoryPath) -> &str {
        if path.is_root() {
            return self.config.title.as_str();
        }
        let parent_path = path.pop().unwrap();
        let children = self.children(&parent_path).unwrap();
        for child in children {
            match child {
                Item::Category(category) => {
                    if category.slug() == path.last_segment().unwrap() {
                        return &category.name;
                    }
                }
                Item::Photo(photo) => {
                    if photo.output_slug() == path.last_segment().unwrap() {
                        return photo.output_name();
                    }
                }
                Item::Page(page) => {
                    if page.slug() == path.last_segment().unwrap() {
                        return &page.name;
                    }
                }
            }
        }
        panic!("{path} not found");
    }

    pub fn category(&self, path: &CategoryPath) -> Option<&Category> {
        if path.is_root() {
            return None;
        }
        let mut segments = path.iter_segments();
        let first_segment = segments.next().unwrap();
        let mut current = self
            .children
            .iter()
            .filter_map(|i| i.category())
            .find(|c| c.slug().as_str() == first_segment)?;

        for segment in segments {
            current = current
                .children
                .iter()
                .filter_map(|i| i.category())
                .find(|c| c.slug().as_str() == segment)?;
        }

        Some(current)
    }

    pub fn visit_items<'a>(&'a self, mut visitor: impl FnMut(&CategoryPath, &'a Item)) {
        visit_children_items(&CategoryPath::ROOT, &self.children, &mut visitor);
    }

    pub fn visit_items_mut(&mut self, mut visitor: impl FnMut(&CategoryPath, &mut Item)) {
        visit_children_items_mut(&CategoryPath::ROOT, &mut self.children, &mut visitor);
    }

    pub fn get_or_create_category(
        &mut self,
        names: &[String],
        path: &CategoryPath,
    ) -> &mut Vec<Item> {
        let mut current_items = &mut self.children;

        for (category_name, _category_slug) in names.iter().zip(path.iter_segments()) {
            let position = current_items.iter().position(|item| {
                if let Item::Category(cat) = item {
                    cat.name == *category_name
                } else {
                    false
                }
            });

            let category = if let Some(index) = position {
                current_items[index].category_mut().unwrap()
            } else {
                current_items.push(Item::Category(Category {
                    name: category_name.to_string(),
                    text: None,
                    children: Vec::new(),
                    config: CategoryConfig::default(),
                }));

                current_items.last_mut().unwrap().category_mut().unwrap()
            };

            current_items = &mut category.children;
        }

        current_items
    }
}

pub fn visit_children_items<'a>(
    path: &CategoryPath,
    children: &'a [Item],
    visitor: &mut impl FnMut(&CategoryPath, &'a Item),
) {
    for child in children {
        visitor(&path, child);
        if let Some(category) = child.category() {
            let path = path.push(category.slug());
            visit_children_items(&path, &category.children, visitor);
        }
    }
}

pub fn visit_children_items_mut(
    path: &CategoryPath,
    children: &mut [Item],
    visitor: &mut impl FnMut(&CategoryPath, &mut Item),
) {
    for child in children {
        visitor(&path, child);
        if let Some(category) = child.category_mut() {
            let path = path.push(category.slug());
            visit_children_items_mut(&path, &mut category.children, visitor);
        }
    }
}

#[derive(Debug)]
pub enum Item {
    Category(Category),
    Photo(Photo),
    Page(Page),
}

impl Item {
    pub fn photo(&self) -> Option<&Photo> {
        if let Self::Photo(photo) = self {
            Some(photo)
        } else {
            None
        }
    }

    pub fn category(&self) -> Option<&Category> {
        if let Self::Category(category) = self {
            Some(category)
        } else {
            None
        }
    }

    pub fn category_mut(&mut self) -> Option<&mut Category> {
        if let Self::Category(category) = self {
            Some(category)
        } else {
            None
        }
    }

    pub fn page(&self) -> Option<&Page> {
        if let Self::Page(page) = self {
            Some(page)
        } else {
            None
        }
    }

    pub fn slug(&self) -> String {
        match self {
            Self::Category(category) => category.slug(),
            Self::Photo(photo) => photo.output_slug(),
            Self::Page(page) => page.slug(),
        }
    }
}

#[derive(Debug)]
pub struct Category {
    pub name: String,
    pub text: Option<RichText>,
    pub children: Vec<Item>,
    pub config: CategoryConfig,
}

impl Category {
    pub fn slug(&self) -> String {
        self.name.replace(' ', "-")
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

#[derive(Debug, Clone, PartialEq)]
pub struct RichText {
    pub content: String,
    pub format: RichTextFormat,
}

#[derive(Debug)]
pub struct Page {
    pub name: String,
    pub text: RichText,
    pub config: PageConfig,
}

impl Page {
    pub fn slug(&self) -> String {
        self.name.replace(' ', "-")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichTextFormat {
    PlainText,
    Markdown,
    Html,
}
