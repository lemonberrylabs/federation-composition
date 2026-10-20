//! Port of `src/utils/graphql.ts`.

/// `extractNamedTypeName("[User!]")` is `Some("User")`; `None` for an empty result.
pub fn extract_named_type_name(type_str: &str) -> Option<String> {
    if type_str.is_empty() {
        return None;
    }
    let type_name = type_str.replace(['!', '[', ']'], "");
    if type_name.is_empty() {
        None
    } else {
        Some(type_name)
    }
}
