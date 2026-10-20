//! Port of graphql-js `language/blockString`.

use crate::js::{is_white_space, js_len};

/// graphql-js `dedentBlockStringLines`.
pub fn dedent_block_string_lines(lines: &[String]) -> Vec<&str> {
    let mut common_indent = usize::MAX;
    let mut first_non_empty_line: Option<usize> = None;
    let mut last_non_empty_line: isize = -1;
    for (i, line) in lines.iter().enumerate() {
        let indent = leading_whitespace(line);
        if indent == line.len() {
            continue;
        }
        first_non_empty_line.get_or_insert(i);
        last_non_empty_line = i as isize;
        if i != 0 && indent < common_indent {
            common_indent = indent;
        }
    }
    let start = first_non_empty_line.unwrap_or(0);
    let end = (last_non_empty_line + 1) as usize;
    if start >= end {
        return Vec::new();
    }
    lines[start..end]
        .iter()
        .enumerate()
        .map(|(j, line)| {
            if j + start == 0 {
                line.as_str()
            } else {
                // The sliced-off prefix is ASCII whitespace, so byte and
                // UTF-16 offsets agree.
                &line[common_indent.min(line.len())..]
            }
        })
        .collect()
}

fn leading_whitespace(s: &str) -> usize {
    s.bytes().take_while(|&b| b == b'\t' || b == b' ').count()
}

/// graphql-js `isPrintableAsBlockString`.
// graphql-js: exported by `language/blockString`; nothing in the port calls it.
#[allow(dead_code)]
pub fn is_printable_as_block_string(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let mut is_empty_line = true;
    let mut has_indent = false;
    let mut has_common_indent = true;
    let mut seen_non_empty_line = false;
    for c in value.chars() {
        match c as u32 {
            0x0000..=0x0008 | 0x000b | 0x000c | 0x000e | 0x000f => return false,
            0x000d => return false,
            10 => {
                if is_empty_line && !seen_non_empty_line {
                    return false;
                }
                seen_non_empty_line = true;
                is_empty_line = true;
                has_indent = false;
            }
            9 | 32 => {
                has_indent |= is_empty_line;
            }
            _ => {
                has_common_indent &= has_indent;
                is_empty_line = false;
            }
        }
    }
    if is_empty_line {
        return false;
    }
    if has_common_indent && seen_non_empty_line {
        return false;
    }
    true
}

/// graphql-js `printBlockString(value, { minimize })`.
pub fn print_block_string(value: &str, minimize: bool) -> String {
    let escaped_value = value.replace("\"\"\"", "\\\"\"\"");
    let lines = split_lines(&escaped_value);
    let is_single_line = lines.len() == 1;
    let force_leading_new_line = lines.len() > 1
        && lines[1..]
            .iter()
            .all(|line| line.is_empty() || is_white_space(line.as_bytes()[0] as u32));
    let has_trailing_triple_quotes = escaped_value.ends_with("\\\"\"\"");
    let has_trailing_quote = value.ends_with('"') && !has_trailing_triple_quotes;
    let has_trailing_slash = value.ends_with('\\');
    let force_trailing_newline = has_trailing_quote || has_trailing_slash;
    let print_as_multiple_lines = !minimize
        && (!is_single_line
            || js_len(value) > 70
            || force_trailing_newline
            || force_leading_new_line
            || has_trailing_triple_quotes);
    let mut result = String::with_capacity(escaped_value.len() + 8);
    let skip_leading_new_line = is_single_line && value.as_bytes().first().is_some_and(|&b| is_white_space(b as u32));
    if (print_as_multiple_lines && !skip_leading_new_line) || force_leading_new_line {
        result.push('\n');
    }
    result.push_str(&escaped_value);
    if print_as_multiple_lines || force_trailing_newline {
        result.push('\n');
    }
    format!("\"\"\"{result}\"\"\"")
}

/// `str.split(/\r\n|[\n\r]/g)`.
fn split_lines(s: &str) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                lines.push(&s[start..i]);
                i += if bytes.get(i + 1) == Some(&b'\n') { 2 } else { 1 };
                start = i;
            }
            b'\n' => {
                lines.push(&s[start..i]);
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    lines.push(&s[start..]);
    lines
}
