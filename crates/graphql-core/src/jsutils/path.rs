//! graphql-js `jsutils/Path`, as the flattened array `pathToArray` returns.

use std::fmt;

/// One key of a graphql-js `Path`: a field name or a list index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathItem {
    Key(String),
    Index(usize),
}

impl fmt::Display for PathItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathItem::Key(key) => f.write_str(key),
            PathItem::Index(index) => write!(f, "{index}"),
        }
    }
}

/// graphql-js `addPath(prev, key, typename)` followed by `pathToArray`.
pub fn add_path(prev: &[PathItem], key: PathItem) -> Vec<PathItem> {
    let mut path = prev.to_vec();
    path.push(key);
    path
}
