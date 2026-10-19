//! Helpers reproducing JavaScript string semantics (UTF-16 code units) on
//! UTF-8 Rust strings.

/// graphql-js `isWhiteSpace`.
pub(crate) fn is_white_space(code: u32) -> bool {
    code == 0x0009 || code == 0x0020
}

/// JavaScript `str.length`: the number of UTF-16 code units.
pub(crate) fn js_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    if bytes.is_ascii() {
        return bytes.len();
    }
    // One unit per char, plus one more for every 4-byte (supplementary) char.
    bytes
        .iter()
        .map(|&b| ((b & 0xC0) != 0x80) as usize + (b >= 0xF0) as usize)
        .sum()
}

/// JavaScript `str.slice(start, start + units)` where `start` is the byte
/// offset of a char boundary. A surrogate pair cut in half becomes U+FFFD,
/// which is what a lone surrogate turns into when a JS string is converted to
/// UTF-8.
pub(crate) fn utf16_slice(s: &str, start: usize, units: usize) -> String {
    let mut out = String::new();
    let mut remaining = units;
    for c in s[start..].chars() {
        if remaining == 0 {
            break;
        }
        let n = c.len_utf16();
        if n <= remaining {
            out.push(c);
            remaining -= n;
        } else {
            out.push('\u{FFFD}');
            break;
        }
    }
    out
}
