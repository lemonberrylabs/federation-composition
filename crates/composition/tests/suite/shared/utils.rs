//! Port of `__tests__/shared/utils.ts`, plus the string helpers of the testkit
//! (`normalizeErrorMessage`, which is `strip-indent` 4.1.1 + `trim`, and template literals).

use std::fmt::Display;

use graphql_js::Document;

/// The `graphql` template tag: `parse(source, { noLocation: true })`. The Rust AST carries no
/// locations. Panics (the TS throws) on a syntax error.
pub fn graphql(source: &str) -> Document {
    match graphql_js::parse(source) {
        Ok(document) => document,
        Err(error) => panic!("graphql`...` threw: {}\n{source}", error.message),
    }
}

/// A template literal's value: `template` with every `${name}` replaced by the matching value.
/// Use through `tpl!`, `graphql!` or `normalize_error_message!`.
///
/// `template` is the cooked TS template text: `\\` is `\`, `` \` `` is a backtick and `\${` is a
/// literal `${`. A literal `${` cannot be written in `template` (it is taken for a missing
/// argument and panics); build such a string with `format!` or `concat` instead.
pub fn interpolate(template: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut used = vec![false; args.len()];
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find('}').unwrap_or_else(|| panic!("unterminated ${{ in template:\n{template}"));
        let name = after[..end].trim();
        let Some(index) = args.iter().position(|(n, _)| *n == name) else {
            panic!("template has no value for ${{{name}}}:\n{template}");
        };
        used[index] = true;
        result.push_str(&args[index].1.to_string());
        rest = &after[end + 1..];
    }
    result.push_str(rest);
    if let Some(index) = used.iter().position(|u| !u) {
        panic!("template has no ${{{}}}:\n{template}", args[index].0);
    }
    result
}

/// `normalizeErrorMessage(literals)`: `stripIndent(message).trim()`.
pub fn normalize_error_message(message: &str) -> String {
    js_trim(&strip_indent(message)).to_string()
}

/// JS `\s`: WhiteSpace and LineTerminator code points (ECMAScript), which is Unicode
/// `White_Space` plus U+FEFF.
pub fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

/// JS `String.prototype.trim`.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// The lines of `s` as JS's `^` in a `/m` regex sees them: split after each line terminator
/// (`\n`, `\r`, U+2028, U+2029), with the terminator kept at the end of its line.
fn js_lines(s: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, c) in s.char_indices() {
        if matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            let end = i + c.len_utf8();
            lines.push(&s[start..end]);
            start = end;
        }
    }
    lines.push(&s[start..]);
    lines
}

/// `strip-indent` 4.1.1:
/// ```js
/// const match = string.match(/^[ \t]*(?=\S)/gm);
/// if (!match) return string;
/// let minIndent = Infinity; for (const indent of match) minIndent = Math.min(minIndent, indent.length);
/// if (minIndent === 0 || minIndent === Infinity) return string;
/// return string.replace(new RegExp(`^[ \\t]{${minIndent}}`, 'gm'), '');
/// ```
pub fn strip_indent(string: &str) -> String {
    let lines = js_lines(string);
    // `^[ \t]*(?=\S)`: the leading spaces and tabs of each line that has a non-whitespace
    // character right after them.
    let min_indent = lines
        .iter()
        .filter_map(|line| {
            let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
            line[indent..].chars().next().filter(|c| !is_js_whitespace(*c)).map(|_| indent)
        })
        .min();
    let min_indent = match min_indent {
        None | Some(0) => return string.to_string(),
        Some(n) => n,
    };
    lines
        .iter()
        .map(|line| {
            let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
            if leading >= min_indent {
                &line[min_indent..]
            } else {
                line
            }
        })
        .collect()
}
