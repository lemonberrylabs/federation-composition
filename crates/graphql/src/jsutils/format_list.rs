//! graphql-js `jsutils/formatList`.

/// graphql-js `orList`: `A`, `A or B`, `A, B, or C`.
pub fn or_list(items: &[String]) -> String {
    format_list("or", items)
}

/// graphql-js `andList`: `A`, `A and B`, `A, B, and C`.
pub fn and_list(items: &[String]) -> String {
    format_list("and", items)
}

fn format_list(conjunction: &str, items: &[String]) -> String {
    // TS: invariant(items.length !== 0); every caller passes a non-empty list.
    assert!(!items.is_empty(), "Unexpected invariant triggered.");
    match items.len() {
        1 => return items[0].clone(),
        2 => return format!("{} {conjunction} {}", items[0], items[1]),
        _ => {}
    }
    let all_but_last = &items[..items.len() - 1];
    let last_item = &items[items.len() - 1];
    format!("{}, {conjunction} {last_item}", all_but_last.join(", "))
}
