//! graphql-js `jsutils/printPathArray`.

use super::path::PathItem;

/// graphql-js `printPathArray`: `""`, or ` at .a[0].b`.
pub fn print_path_array(path: &[PathItem]) -> String {
    if path.is_empty() {
        return String::new();
    }
    let keys: String = path
        .iter()
        .map(|key| match key {
            PathItem::Index(index) => format!("[{index}]"),
            PathItem::Key(key) => format!(".{key}"),
        })
        .collect();
    format!(" at {keys}")
}
