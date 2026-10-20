//! Port of `src/utils/helpers.ts`.

use std::hash::Hash;

use crate::utils::collections::IndexSet;

use crate::utils::js::{JsError, JsResult};

/// `isDefined(value)`. In Rust an `Option` is checked with `is_some()`; kept for call-site parity.
pub fn is_defined<T>(value: &Option<T>) -> bool {
    value.is_some()
}

/// `ensureValue(value, message)`: throws `new Error(message)` when the value is missing.
pub fn ensure_value<T>(value: Option<T>, message: &str) -> JsResult<T> {
    value.ok_or_else(|| JsError::error(message))
}

/// `mathMax(firstValue, secondValue)`. `Math.max` returns `NaN` when either side is `NaN`.
pub fn math_max(first_value: f64, second_value: Option<f64>) -> f64 {
    match second_value {
        None => first_value,
        Some(second) => {
            if first_value.is_nan() || second.is_nan() {
                f64::NAN
            } else {
                first_value.max(second)
            }
        }
    }
}

/// `mathMaxNullable(left, right)`.
pub fn math_max_nullable(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    if let Some(left) = left {
        return Some(math_max(left, right));
    }
    if let Some(right) = right {
        return Some(math_max(right, left));
    }
    None
}

/// `nullableArrayUnion(left, right)`: `None` when neither side is an array.
pub fn nullable_array_union<T: Hash + Eq + Clone>(left: Option<&[T]>, right: Option<&[T]>) -> Option<Vec<T>> {
    if left.is_none() && right.is_none() {
        return None;
    }
    let mut unique_set: IndexSet<T> = IndexSet::default();
    for v in left.into_iter().flatten().chain(right.into_iter().flatten()) {
        unique_set.insert(v.clone());
    }
    Some(unique_set.into_iter().collect())
}
