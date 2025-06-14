use std::cmp::Reverse;

use chrono::NaiveDateTime;

use crate::gallery::{Item, Page};

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub enum Order {
    Category {
        order: Reverse<i64>,
        name: String,
    },
    Photo {
        order: Reverse<i64>,
        date: Reverse<Option<NaiveDateTime>>,
        name: String,
    },
    Page {
        order: Reverse<i64>,
        name: String,
    },
}

impl Order {
    pub fn new_page(page: &Page) -> Self {
        Self::Page {
            order: Reverse(page.config.order),
            name: page.name.clone(),
        }
    }

    pub fn new(item: &Item) -> Self {
        match item {
            Item::Category(category) => Self::Category {
                order: Reverse(category.config.order),
                name: category.name.clone(),
            },
            Item::Photo(photo) => Self::Photo {
                order: Reverse(photo.config.order),
                date: Reverse(photo.date_time()),
                name: photo.output_name().to_owned(),
            },
            Item::Page(page) => Self::new_page(page),
        }
    }
}
