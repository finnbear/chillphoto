use crate::gallery::{Category, Page, Photo};

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
            Self::Photo(photo) => photo.slug(),
            Self::Page(page) => page.slug(),
        }
    }
}
