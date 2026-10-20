//! WebAssembly bindings of `federation_composition_rs` for the TypeScript shim
//! in `wasm/`. Everything JS-specific lives here and in the shim.
//!
//! Conventions shared with `wasm/runtime.ts`:
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
//!   [`ThrownError`], which the shim rethrows as `Error` (or `TypeError`...) or
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

impl From<federation_composition_rs::utils::js::JsError> for ThrownError {
    fn from(e: federation_composition_rs::utils::js::JsError) -> Self {
        use federation_composition_rs::utils::js::JsError;
        match e {
            JsError::Error { name, message } => ThrownError::Error {
                name: name.to_owned(),
                message,
            },
            JsError::GraphQL(e) => ThrownError::graphql(e),
        }
    }
}

/// A crate `JsError` as the value the shim rethrows.
pub(crate) fn thrown(e: federation_composition_rs::utils::js::JsError) -> JsValue {
    ThrownError::from(e).into()
}

impl From<ThrownError> for JsValue {
    fn from(e: ThrownError) -> JsValue {
        JsValue::from_str(&serde_json::to_string(&e).unwrap())
    }
}

pub(crate) type JsResult<T> = Result<T, JsValue>;

/// Parses a JSON argument sent by the shim, either a string (`json.as_bytes()`) or the UTF-8
/// bytes of one (`encodeJSON` in the shim, for documents).
pub(crate) fn from_json<T: serde::de::DeserializeOwned>(what: &str, json: &[u8]) -> JsResult<T> {
    // Checking the whole input once and parsing a `&str` is faster than `from_slice`, which
    // checks every string it reads (~5% of a large document's parse). Invalid UTF-8 (which
    // `TextEncoder` never produces) still goes through `from_slice`, which reports where it is.
    let parsed = match std::str::from_utf8(json) {
        Ok(json) => serde_json::from_str(json),
        Err(_) => serde_json::from_slice(json),
    };
    parsed.map_err(|e| ThrownError::error(format!("composition-wasm: invalid {what} argument: {e}")).into())
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
