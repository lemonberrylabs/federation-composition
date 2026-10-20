//! graphql-js `jsutils/capitalize`.

/// graphql-js `capitalize`: `str.charAt(0).toUpperCase() + str.slice(1).toLowerCase()`.
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out: String = first.to_uppercase().collect();
            out.push_str(&chars.as_str().to_lowercase());
            out
        }
    }
}
