//! Ports of the graphql-js `jsutils` helpers that schema building and
//! validation use, and [`JsValue`], the JavaScript values graphql-js coerces
//! input values to.

pub mod capitalize;
pub mod did_you_mean;
pub mod format_list;
pub mod inspect;
pub mod natural_compare;
pub mod path;
pub mod print_path_array;
pub mod suggestion_list;

use crate::collections::IndexMap;

/// A JavaScript value as graphql-js input coercion produces and consumes it
/// (`coerceInputValue`, `coerceInputLiteral`, `valueFromASTUntyped`, the
/// `default.value` of an argument). `undefined` is `Option::None` where a
/// value may be missing.
#[derive(Clone, Debug, PartialEq)]
pub enum JsValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    /// A JS array.
    List(Vec<JsValue>),
    /// A JS object (`Object.create(null)` maps). Keys are GraphQL names,
    /// which are never integer-like, so insertion order is JS key order.
    Object(IndexMap<String, JsValue>),
}
