//! JavaScript semantics the TypeScript sources rely on implicitly.
//!
//! Not a port of a TS module: these are the runtime behaviors (`String.prototype.localeCompare`,
//! `Number.prototype.toString`) that the TS code gets from the JS engine.
//! Both are checked against Node in `tests/js_semantics.rs`.

use std::cmp::Ordering;

/// ASCII characters in ICU root collation order (what V8's `localeCompare` uses).
/// Characters in the same group differ only by case (a tertiary difference).
/// Generated with Node: printable ASCII sorted by `localeCompare`, grouped by
/// `Intl.Collator(undefined, { sensitivity: "base" })`.
const ASCII_COLLATION_GROUPS: [&str; 69] = [
    " ", "_", "-", ",", ";", ":", "!", "?", ".", "'", "\"", "(", ")", "[", "]", "{", "}", "@", "*",
    "/", "\\", "&", "#", "%", "`", "^", "+", "<", "=", ">", "|", "~", "$", "0", "1", "2", "3", "4",
    "5", "6", "7", "8", "9", "aA", "bB", "cC", "dD", "eE", "fF", "gG", "hH", "iI", "jJ", "kK",
    "lL", "mM", "nN", "oO", "pP", "qQ", "rR", "sS", "tT", "uU", "vV", "wW", "xX", "yY", "zZ",
];

const fn primary_weights() -> [u16; 128] {
    let mut weights = [0u16; 128];
    let mut group = 0;
    while group < ASCII_COLLATION_GROUPS.len() {
        let bytes = ASCII_COLLATION_GROUPS[group].as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            weights[bytes[i] as usize] = group as u16 + 1;
            i += 1;
        }
        group += 1;
    }
    weights
}

const PRIMARY: [u16; 128] = primary_weights();

/// Primary weight of a character. Printable ASCII uses ICU's order; anything else
/// (control characters, non-ASCII) sorts after ASCII by code point.
/// NOTE(port): non-ASCII collation is approximate. The inputs compared with
/// `localeCompare` in `src/` are subgraph ids, type and field names, and auth scope
/// strings, which are ASCII in practice.
fn primary(c: char) -> u32 {
    match PRIMARY.get(c as usize) {
        Some(&w) if w != 0 => w as u32,
        _ => 1000 + c as u32,
    }
}

/// `a.localeCompare(b)` in Node's default (en-US) locale: compare primary weights
/// across the whole string first, then case (lowercase before uppercase) left to right.
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    let primary_order = a.chars().map(primary).cmp(b.chars().map(primary));
    if primary_order != Ordering::Equal {
        return primary_order;
    }
    for (x, y) in a.chars().zip(b.chars()) {
        if x != y {
            return match (x.is_ascii_lowercase(), y.is_ascii_lowercase()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => x.cmp(&y),
            };
        }
    }
    Ordering::Equal
}

/// `String(n)` for a JS number (ECMAScript `Number::toString` with radix 10).
pub fn number_to_string(n: f64) -> String {
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
        format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
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
