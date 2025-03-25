use std::fmt::Display;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CategoryPath {
    segments: Vec<String>,
}

impl CategoryPath {
    pub const ROOT: Self = Self {
        segments: Vec::new(),
    };

    pub fn new(path: &str) -> Self {
        if path == "" {
            Self {
                segments: Vec::new(),
            }
        } else {
            Self {
                segments: path.split('/').map(|s| s.to_owned()).collect(),
            }
        }
    }

    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn push(&self, segment: String) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment);
        Self { segments }
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = &str> {
        self.segments.iter().map(|s| s.as_str())
    }

    /// Starts at root.
    pub fn iter_paths(&self) -> impl Iterator<Item = Self> + '_ {
        let mut ret = Self::ROOT;
        std::iter::once(ret.clone()).chain(self.iter_segments().map(move |segment| {
            ret = std::mem::take(&mut ret).push(segment.to_owned());
            ret.clone()
        }))
    }

    pub fn to_string_without_leading_slash(&self) -> String {
        self.to_string()
    }

    pub fn to_string_with_leading_slash(&self) -> String {
        format!("/{self}")
    }

    pub fn last_segment(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }
}

impl Display for CategoryPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.segments.is_empty() {
            f.write_str(&self.segments.join("/"))
        } else {
            Ok(())
        }
    }
}
