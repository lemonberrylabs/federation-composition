//! WebAssembly bindings of `federation_composition` for the TypeScript shim
//! in `wasm/`. Everything JS-specific lives here and in the shim.
//!
//! Conventions shared with `wasm/runtime.ts`:
//!
//! * Structured arguments and results are JSON strings. AST nodes use the
//!   graphql-js JSON form (`graphql_js` serde); the shim strips `loc`.
//! * A function that throws in TypeScript returns `Err` with a JSON string,
//!   [`ThrownError`], which the shim rethrows as `Error` or `GraphQLError`.
//! * A Rust panic traps the instance. The panic hook records the payload
//!   first; the shim reads it with [`take_panic_message`], throws an `Error`
//!   with that message and loads a fresh instance.
//! * GraphQL errors that are results (composition errors) are JSON
//!   `{ message, extensions? }` objects inside the result.

use std::cell::RefCell;

use serde::Serialize;
use wasm_bindgen::prelude::*;

mod exports;

pub use exports::*;

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
    /// `throw new Error(message)`.
    Error { message: String },
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
            message: message.into(),
        }
    }

    pub(crate) fn graphql(error: graphql_js::GraphQLError) -> Self {
        ThrownError::GraphQL {
            message: error.message,
            extensions: error.extensions.into_iter().collect(),
        }
    }
}

impl From<ThrownError> for JsValue {
    fn from(e: ThrownError) -> JsValue {
        JsValue::from_str(&serde_json::to_string(&e).unwrap())
    }
}

pub(crate) type JsResult<T> = Result<T, JsValue>;

/// Parses a JSON argument sent by the shim.
pub(crate) fn from_json<T: serde::de::DeserializeOwned>(what: &str, json: &str) -> JsResult<T> {
    serde_json::from_str(json).map_err(|e| {
        ThrownError::error(format!("composition-wasm: invalid {what} argument: {e}")).into()
    })
}

/// Serializes a result for the shim.
pub(crate) fn to_json<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).expect("results serialize")
}

/// The error of an export that is not connected to `federation_composition` yet.
pub(crate) fn not_wired<T>(js_name: &str) -> JsResult<T> {
    Err(ThrownError::error(format!(
        "Not wired yet: {js_name} (crates/composition-wasm does not call federation_composition yet)"
    ))
    .into())
}

/// A GraphQL error as the shim rebuilds it: `{ message, extensions? }`.
pub(crate) fn graphql_error_json(error: &graphql_js::GraphQLError) -> serde_json::Value {
    serde_json::to_value(error).unwrap()
}
