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
    " ", "_", "-", ",", ";", ":", "!", "?", ".", "'", "\"", "(", ")", "[", "]", "{", "}", "@", "*", "/", "\\", "&",
    "#", "%", "`", "^", "+", "<", "=", ">", "|", "~", "$", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "aA",
    "bB", "cC", "dD", "eE", "fF", "gG", "hH", "iI", "jJ", "kK", "lL", "mM", "nN", "oO", "pP", "qQ", "rR", "sS", "tT",
    "uU", "vV", "wW", "xX", "yY", "zZ",
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

/// A value thrown by the TypeScript code (`throw new Error(..)`, `throw new GraphQLError(..)`,
/// or a runtime `TypeError`), carried as the `Err` of [`JsResult`].
///
/// A `catch` in `src/` that inspects the thrown value (`error instanceof GraphQLError`,
/// `String(error)`) does the same on this enum. A throw nothing in `src/` catches escapes to
/// the public API as an `Err`; the WebAssembly shim rethrows it as a JS error with the same
/// message.
#[derive(Clone, Debug, PartialEq)]
pub enum JsError {
    /// `new Error(message)` (`name` is `"Error"`) or a runtime error such as `TypeError`.
    Error {
        /// The constructor name, `"Error"` or `"TypeError"`.
        name: &'static str,
        /// `error.message`.
        message: String,
    },
    /// `new GraphQLError(..)`.
    GraphQL(graphql_rs::GraphQLError),
}

/// `Result` of a function whose TypeScript counterpart can throw.
pub type JsResult<T> = Result<T, JsError>;

impl JsError {
    /// `new Error(message)`.
    pub fn error(message: impl Into<String>) -> Self {
        JsError::Error {
            name: "Error",
            message: message.into(),
        }
    }

    /// A runtime `TypeError` (e.g. reading a property of `undefined`).
    pub fn type_error(message: impl Into<String>) -> Self {
        JsError::Error {
            name: "TypeError",
            message: message.into(),
        }
    }

    /// `error.message`.
    pub fn message(&self) -> &str {
        match self {
            JsError::Error { message, .. } => message,
            JsError::GraphQL(e) => &e.message,
        }
    }

    /// `String(error)`: `"Error: message"` for an `Error`, the message for a `GraphQLError`
    /// (graphql-js `toString()` without locations).
    pub fn to_js_string(&self) -> String {
        match self {
            JsError::Error { name, message } => {
                if message.is_empty() {
                    name.to_string()
                } else {
                    format!("{name}: {message}")
                }
            }
            JsError::GraphQL(e) => e.message.clone(),
        }
    }

    /// `error instanceof GraphQLError ? error : new GraphQLError(String(error))`.
    pub fn into_graphql_error(self) -> graphql_rs::GraphQLError {
        match self {
            JsError::GraphQL(e) => e,
            other => graphql_rs::GraphQLError::new(other.to_js_string()),
        }
    }
}

impl From<graphql_rs::GraphQLError> for JsError {
    fn from(e: graphql_rs::GraphQLError) -> Self {
        JsError::GraphQL(e)
    }
}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_js_string())
    }
}

impl std::error::Error for JsError {}

/// JavaScript `parseFloat(s)`: the longest prefix of `s` (after leading whitespace) that is a
/// decimal literal, or `NaN`.
pub fn parse_float(s: &str) -> f64 {
    let t = s.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    for (prefix, value) in [
        ("Infinity", f64::INFINITY),
        ("+Infinity", f64::INFINITY),
        ("-Infinity", f64::NEG_INFINITY),
    ] {
        if t.starts_with(prefix) {
            return value;
        }
    }
    let bytes = t.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let mut mantissa_digits = end - digits_start;
    if end < bytes.len() && bytes[end] == b'.' {
        let mut frac_end = end + 1;
        while frac_end < bytes.len() && bytes[frac_end].is_ascii_digit() {
            frac_end += 1;
        }
        mantissa_digits += frac_end - end - 1;
        if mantissa_digits > 0 {
            end = frac_end;
        }
    }
    if mantissa_digits == 0 {
        return f64::NAN;
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut exp_end = end + 1;
        if exp_end < bytes.len() && (bytes[exp_end] == b'+' || bytes[exp_end] == b'-') {
            exp_end += 1;
        }
        let exp_digits_start = exp_end;
        while exp_end < bytes.len() && bytes[exp_end].is_ascii_digit() {
            exp_end += 1;
        }
        if exp_end > exp_digits_start {
            end = exp_end;
        }
    }
    t[..end].parse::<f64>().unwrap_or(f64::NAN)
}

/// JavaScript `parseInt(s, 10)`: optional sign and the longest run of decimal digits after
/// leading whitespace, or `NaN`.
pub fn parse_int(s: &str) -> f64 {
    let t = s.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    let (negative, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let digits: &str = &rest[..rest.bytes().take_while(u8::is_ascii_digit).count()];
    if digits.is_empty() {
        return f64::NAN;
    }
    let value = digits.parse::<f64>().unwrap_or(f64::NAN);
    if negative {
        -value
    } else {
        value
    }
}
