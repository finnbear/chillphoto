use crate::photo::Photo;

#[derive(Debug)]
pub struct Gallery {
    pub children: Vec<Item>,
}

impl Gallery {
    pub fn visit_items<'a>(&'a self, mut visitor: impl FnMut(&[String], &'a Item)) {
        visit_children_items(&mut Vec::<String>::new(), &self.children, &mut visitor);
    }

    pub fn visit_items_mut(&mut self, mut visitor: impl FnMut(&[String], &mut Item)) {
        visit_children_items_mut(&mut Vec::<String>::new(), &mut self.children, &mut visitor);
    }

    pub fn get_or_create_category(&mut self, path: &[&str]) -> &mut Vec<Item> {
        let mut current_items = &mut self.children;

        for category_name in path {
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
                    description: None,
                    children: Vec::new(),
                }));

                current_items.last_mut().unwrap().category_mut().unwrap()
            };

            current_items = &mut category.children;
        }

        current_items
    }
}

pub fn visit_children_items<'a>(
    path: &mut Vec<String>,
    children: &'a [Item],
    visitor: &mut impl FnMut(&[String], &'a Item),
) {
    for child in children {
        visitor(&path, child);
        if let Some(category) = child.category() {
            path.push(category.name.clone());
            visit_children_items(path, &category.children, visitor);
            path.pop().unwrap();
        }
    }
}

pub fn visit_children_items_mut(
    path: &mut Vec<String>,
    children: &mut [Item],
    visitor: &mut impl FnMut(&[String], &mut Item),
) {
    for child in children {
        visitor(&path, child);
        if let Some(category) = child.category_mut() {
            path.push(category.name.clone());
            visit_children_items_mut(path, &mut category.children, visitor);
            path.pop().unwrap();
        }
    }
}

#[derive(Clone, Debug)]
pub enum Item {
    Category(Category),
    Photo(Photo),
    Page(Page),
}

impl Item {
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
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub description: Option<String>,
    pub children: Vec<Item>,
}

impl Category {
    pub fn visit_items<'a>(
        &'a self,
        path: &[String],
        mut visitor: impl FnMut(&[String], &'a Item),
    ) {
        let mut path = path.to_owned();
        path.push(self.name.clone());
        visit_children_items(&mut path, &self.children, &mut visitor);
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub format: PageFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum PageFormat {
    PlainText,
    Markdown,
    Html,
}
