//! WebAssembly bindings of `federation_composition_rs` for the TypeScript shim
//! in `npm/src` (the npm package @lemonberrylabs/federation-composition-wasm). Everything JS-specific lives here and in the shim.
//!
//! Conventions shared with `npm/src/runtime.ts`:
//!
//! * Structured arguments and results are JSON. AST nodes use the graphql-js
//!   JSON form (`graphql_rs` serde); the shim leaves out `loc` and the
//!   deserializer ignores any `loc` left in. Arguments that carry documents
//!   are UTF-8 bytes (`&[u8]`, a `Uint8Array` from `encodeJSON` in the shim):
//!   wasm-bindgen copies those, while it re-encodes a `&str` argument one
//!   character at a time in JS. A composition's supergraph document is
//!   returned as bytes too, which the shim decodes only when it is read.
//!   Small arguments and other results are strings.
//! * A function that throws in TypeScript returns `Err` with a JSON string,
//!   `ThrownError`, which the shim rethrows as `Error` (or `TypeError`...) or
//!   `GraphQLError`. A JS exception thrown by a callback passed in (such as
//!   `resolveName`) is rethrown unchanged.
//! * A Rust panic traps the instance. The panic hook records the payload
//!   first; the shim reads it with [`take_panic_message`], throws an `Error`
//!   with that message and loads a fresh instance.
//! * GraphQL errors that are results (composition errors) are JSON
//!   `{ message, extensions? }` objects inside the result.

use std::cell::RefCell;

use serde::Serialize;
use wasm_bindgen::prelude::*;

mod convert;
mod exports;

pub use exports::*;

// This crate is the final WebAssembly artifact, so it picks the allocator (the library crates
// never do). Talc instead of the default dlmalloc: measured on the corpus through the shim,
// 2-3% faster (huge-schema 304 -> 298 ms, test-suite geomean 0.302 -> 0.294 ms); rlsf was
// slower than dlmalloc. Talc needs no `getrandom`, and this constructor needs no `unsafe`.
#[cfg(all(target_family = "wasm", not(target_feature = "atomics")))]
#[global_allocator]
static ALLOCATOR: talc::wasm::WasmDynamicTalc = talc::wasm::new_wasm_dynamic_allocator();

thread_local! {
    static LAST_PANIC: RefCell<Option<PanicInfo>> = const { RefCell::new(None) };
}

#[derive(Serialize)]
struct PanicInfo {
    message: String,
    location: Option<String>,
}

#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_owned()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Rust panic with a non-string payload".to_owned()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        LAST_PANIC.with(|p| *p.borrow_mut() = Some(PanicInfo { message, location }));
    }));
}

/// The message and location of the last panic, as JSON
/// `{ message, location }`, or `undefined`. Clears it.
#[wasm_bindgen]
pub fn take_panic_message() -> Option<String> {
    LAST_PANIC
        .with(|p| p.borrow_mut().take())
        .map(|info| serde_json::to_string(&info).unwrap())
}

/// What a function that throws in TypeScript throws.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ThrownError {
    /// `throw new Error(message)`, or a runtime error (`name` is then e.g. `"TypeError"`).
    Error { name: String, message: String },
    /// `throw new GraphQLError(message, { extensions })`.
    #[serde(rename = "graphql")]
    GraphQL {
        message: String,
        #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
        extensions: serde_json::Map<String, serde_json::Value>,
    },
}

impl ThrownError {
    pub(crate) fn error(message: impl Into<String>) -> Self {
        ThrownError::Error {
            name: "Error".to_owned(),
            message: message.into(),
        }
    }

    pub(crate) fn graphql(error: graphql_rs::GraphQLError) -> Self {
        ThrownError::GraphQL {
            message: error.message,
            extensions: error.extensions.into_iter().collect(),
        }
    }
}

impl From<federation_composition_rs::__internal::utils::js::JsError> for ThrownError {
    fn from(e: federation_composition_rs::__internal::utils::js::JsError) -> Self {
        use federation_composition_rs::__internal::utils::js::JsError;
        match e {
            JsError::Error { name, message } => ThrownError::Error {
                name: name.to_owned(),
                message,
            },
            JsError::GraphQL(e) => ThrownError::graphql(e),
        }
    }
}

/// A thrown error of the library (its `JsError`, or the public `Error` that wraps one) as the
/// value the shim rethrows.
pub(crate) fn thrown(e: impl Into<federation_composition_rs::__internal::utils::js::JsError>) -> JsValue {
    ThrownError::from(e.into()).into()
}

impl From<ThrownError> for JsValue {
    fn from(e: ThrownError) -> JsValue {
        JsValue::from_str(&serde_json::to_string(&e).unwrap())
    }
}

pub(crate) type JsResult<T> = Result<T, JsValue>;

/// The deepest nesting of arrays and objects a JSON argument may have: enough for a document at
/// the parser's nesting limit (`graphql_rs::MAX_NESTING_DEPTH`), whose deepest levels (object
/// values and selection sets) take three JSON levels each, inside the arguments' own structure.
/// serde_json's default limit, 128 JSON levels, would reject documents graphql-js and the Rust
/// parser accept (a list value 60 levels deep).
pub(crate) const MAX_JSON_DEPTH: usize = 3 * graphql_rs::MAX_NESTING_DEPTH + 64;

/// Parses a JSON argument sent by the shim, either a string (`json.as_bytes()`) or the UTF-8
/// bytes of one (`encodeJSON` in the shim, for documents).
///
/// An argument nested deeper than [`MAX_JSON_DEPTH`] is an `Error`, found before parsing (which
/// recurses once per level), and the parser's own recursion limit is off.
pub(crate) fn from_json<T: serde::de::DeserializeOwned>(what: &str, json: &[u8]) -> JsResult<T> {
    let invalid = |e: &dyn std::fmt::Display| -> JsValue {
        ThrownError::error(format!("composition-wasm: invalid {what} argument: {e}")).into()
    };
    if json_depth_exceeds(json, MAX_JSON_DEPTH) {
        return Err(invalid(&format_args!("nested deeper than {MAX_JSON_DEPTH} levels")));
    }
    // Checking the whole input once and parsing a `&str` is faster than `from_slice`, which
    // checks every string it reads (~5% of a large document's parse). Invalid UTF-8 (which
    // `TextEncoder` never produces) still goes through `from_slice`, which reports where it is.
    let parsed = match std::str::from_utf8(json) {
        Ok(json) => deserialize(serde_json::Deserializer::from_str(json)),
        Err(_) => deserialize(serde_json::Deserializer::from_slice(json)),
    };
    parsed.map_err(|e| invalid(&e))
}

/// `serde_json::from_str` without its recursion limit ([`from_json`] has checked the depth).
fn deserialize<'de, R: serde_json::de::Read<'de>, T: serde::de::DeserializeOwned>(
    mut deserializer: serde_json::Deserializer<R>,
) -> serde_json::Result<T> {
    deserializer.disable_recursion_limit();
    let value = T::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

/// Whether arrays and objects nest deeper than `max_depth` in `json` (brackets in strings do not
/// count). Malformed JSON is left for the parser to report.
fn json_depth_exceeds(json: &[u8], max_depth: usize) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in json {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'[' | b'{' => {
                depth += 1;
                if depth > max_depth {
                    return true;
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

/// Rejects a document nested deeper than the parser allows, as the library's public functions
/// do (`graphql_rs::check_nesting_depth`), before the binding recurses into it. Within
/// [`MAX_JSON_DEPTH`] a document can be up to that many levels deep.
pub(crate) fn check_nesting_depth(document: &graphql_rs::Document) -> JsResult<()> {
    graphql_rs::check_nesting_depth(document, graphql_rs::MAX_NESTING_DEPTH)
        .map_err(|error| ThrownError::graphql(error).into())
}

/// Serializes a result for the shim.
pub(crate) fn to_json<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).expect("results serialize")
}

/// Serializes a result for the shim as UTF-8 bytes, which reach JS as a `Uint8Array` that the
/// shim decodes only if it needs the value (the supergraph document of a composition).
pub(crate) fn to_json_bytes<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("results serialize")
}

/// A GraphQL error as the shim rebuilds it: `{ message, extensions? }`.
pub(crate) fn graphql_error_json(error: &graphql_rs::GraphQLError) -> serde_json::Value {
    serde_json::to_value(error).unwrap()
}
