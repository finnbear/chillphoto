use image::RgbImage;
use std::{path::PathBuf, sync::OnceLock};

mod category;
mod category_path;
mod config;
mod exif;
mod item;
mod order;
mod page;
mod photo;
mod rich_text;
mod static_file;

pub use category::*;
pub use category_path::*;
pub use config::*;
pub use exif::*;
pub use item::*;
pub use order::*;
pub use page::*;
pub use photo::*;
pub use rich_text::*;
pub use static_file::*;

#[derive(Debug)]
pub struct Gallery {
    pub children: Vec<Item>,
    pub favicon: Option<(Vec<u8>, OnceLock<RgbImage>)>,
    pub config: GalleryConfig,
    pub head_html: Option<String>,
    pub home_text: Option<RichText>,
    pub static_files: Vec<StaticFile>,
    /// Path to top level of gallery source files in file system.
    pub root: PathBuf,
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
                    if photo.slug() == path.last_segment().unwrap() {
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
                    src_key: names.join("/"),
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
