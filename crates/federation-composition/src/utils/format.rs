//! Port of `src/utils/format.ts`.

/// `andList(items, commaBeforeConjunction = true, wrapper?)`.
/// Given `[A, B, C]`: `commaBeforeConjunction` true gives `A, B, and C`, false gives `A, B and C`.
pub fn and_list(items: &[String], comma_before_conjunction: bool, wrapper: Option<&str>) -> String {
    format_list("and", items, comma_before_conjunction, wrapper)
}

fn format_list(conjunction: &str, items: &[String], comma_before_conjunction: bool, wrapper: Option<&str>) -> String {
    match items.len() {
        0 => String::new(),
        1 => with_wrapper(&items[0], wrapper),
        2 => format!(
            "{} {} {}",
            with_wrapper(&items[0], wrapper),
            conjunction,
            with_wrapper(&items[1], wrapper)
        ),
        n => {
            let all_but_last: Vec<String> = items[..n - 1].iter().map(|item| with_wrapper(item, wrapper)).collect();
            let last_item = with_wrapper(&items[n - 1], wrapper);
            format!(
                "{}{}{} {}",
                all_but_last.join(", "),
                if comma_before_conjunction { ", " } else { " " },
                conjunction,
                last_item
            )
        }
    }
}

fn with_wrapper(text: &str, wrapper: Option<&str>) -> String {
    // `if (!wrapper)`: an empty wrapper counts as no wrapper.
    match wrapper {
        Some(w) if !w.is_empty() => format!("{w}{text}{w}"),
        _ => text.to_string(),
    }
}
