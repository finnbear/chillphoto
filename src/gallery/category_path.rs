use std::fmt::Display;

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct CategoryPath {
    /// Each corresponds to a slug with hyphens,
    /// not the original name with spaces.
    segments: Vec<String>,
}

impl CategoryPath {
    pub const ROOT: Self = Self {
        segments: Vec::new(),
    };

    #[track_caller]
    pub fn new(path: &str) -> Self {
        if path == "" {
            Self {
                segments: Vec::new(),
            }
        } else {
            Self {
                segments: path
                    .split('/')
                    .map(|s| {
                        assert!(!s.contains(' '));
                        s.to_owned()
                    })
                    .collect(),
            }
        }
    }

    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn pop(&self) -> Option<Self> {
        let mut ret = self.clone();
        if ret.segments.pop().is_some() {
            Some(ret)
        } else {
            None
        }
    }

    /// `segment` must not have spaces.
    #[track_caller]
    pub fn push(&self, segment: String) -> Self {
        assert!(!segment.contains(' '), "{}", segment);
        let mut segments = self.segments.clone();
        segments.push(segment);
        Self { segments }
    }

    pub fn len(&self) -> usize {
        self.segments.len()
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

    #[allow(unused)]
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
