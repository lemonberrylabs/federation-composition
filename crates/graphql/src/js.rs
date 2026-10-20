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

/// JavaScript `String(n)` for a number.
pub(crate) fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n == 0.0 {
        return "0".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if n < 0.0 {
        return format!("-{}", number_to_string(-n));
    }

    // Shortest round-trip digits and exponent: `{:e}` gives "d.ddde±x".
    let formatted = format!("{n:e}");
    let (mantissa, exponent) = formatted.split_once('e').expect("{:e} has an exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    // n = 0.digits × 10^point
    let point = exponent.parse::<i32>().expect("valid exponent") + 1;

    if k <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - k) as usize))
    } else if 0 < point && point <= 21 {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let e = point - 1;
        let sign = if e < 0 { '-' } else { '+' };
        if k == 1 {
            format!("{digits}e{sign}{}", e.abs())
        } else {
            format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], e.abs())
        }
    }
}
