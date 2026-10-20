//! Port of graphql-js `utilities/valueFromASTUntyped`.

use crate::ast::Value;
use crate::collections::IndexMap;
use crate::jsutils::JsValue;
use crate::specified::{js_parse_float, js_parse_int};

/// graphql-js `valueFromASTUntyped(valueNode)` without variables: a
/// variable is `undefined` (`None`).
pub fn value_from_ast_untyped(value_node: &Value) -> Option<JsValue> {
    Some(match value_node {
        Value::Null(_) => JsValue::Null,
        Value::Int(int) => JsValue::Number(js_parse_int(&int.value)),
        Value::Float(float) => JsValue::Number(js_parse_float(&float.value)),
        Value::String(s) => JsValue::String(s.value.clone()),
        Value::Enum(e) => JsValue::String(e.value.clone()),
        Value::Boolean(b) => JsValue::Boolean(b.value),
        // NOTE(port): only a variable is `undefined`, and the values this
        // crate coerces are constant (SDL), so items and fields are never
        // `undefined`; one would be kept as `null` here.
        Value::List(list) => JsValue::List(
            list.values
                .iter()
                .map(|node| value_from_ast_untyped(node).unwrap_or(JsValue::Null))
                .collect(),
        ),
        Value::Object(object) => {
            // `keyValMap`: a repeated field name keeps its first position and
            // its last value.
            let mut map = IndexMap::default();
            for field in &object.fields {
                let value = value_from_ast_untyped(&field.value).unwrap_or(JsValue::Null);
                map.insert(field.name.value.clone(), value);
            }
            JsValue::Object(map)
        }
        Value::Variable(_) => return None,
    })
}
