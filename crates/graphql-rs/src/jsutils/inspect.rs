//! graphql-js `jsutils/inspect`, for the values it is applied to while
//! building and validating schemas: [`JsValue`]s. GraphQL types, fields and
//! arguments are inspected through their `toJSON`, which is their `Display`.

use super::JsValue;
use crate::collections::IndexMap;
use crate::js::number_to_string;

const MAX_ARRAY_LENGTH: usize = 10;
const MAX_RECURSIVE_DEPTH: usize = 2;

/// graphql-js `inspect(value)`.
pub fn inspect(value: &JsValue) -> String {
    format_value(value, 0)
}

/// `seen` is the length of graphql-js `seenValues`.
fn format_value(value: &JsValue, seen: usize) -> String {
    match value {
        JsValue::String(s) => serde_json::to_string(s).expect("strings serialize"),
        JsValue::Number(n) => number_to_string(*n),
        JsValue::Boolean(b) => b.to_string(),
        JsValue::Null => "null".to_string(),
        // NOTE(port): values here are plain arrays and `Object.create(null)`
        // objects: none has `toJSON`, none can be circular.
        JsValue::List(array) => format_array(array, seen + 1),
        JsValue::Object(object) => format_object(object, seen + 1),
    }
}

fn format_object(object: &IndexMap<String, JsValue>, seen: usize) -> String {
    if object.is_empty() {
        return "{}".to_string();
    }
    if seen > MAX_RECURSIVE_DEPTH {
        // `getObjectTag` of an `Object.create(null)` object.
        return "[Object]".to_string();
    }
    let properties: Vec<String> = object
        .iter()
        .map(|(key, value)| format!("{key}: {}", format_value(value, seen)))
        .collect();
    format!("{{ {} }}", properties.join(", "))
}

fn format_array(array: &[JsValue], seen: usize) -> String {
    if array.is_empty() {
        return "[]".to_string();
    }
    if seen > MAX_RECURSIVE_DEPTH {
        return "[Array]".to_string();
    }
    let len = MAX_ARRAY_LENGTH.min(array.len());
    let remaining = array.len() - len;
    let mut items: Vec<String> = array[..len]
        .iter()
        .map(|item| format_value(item, seen))
        .collect();
    if remaining == 1 {
        items.push("... 1 more item".to_string());
    } else if remaining > 1 {
        items.push(format!("... {remaining} more items"));
    }
    format!("[{}]", items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspects_like_graphql_js() {
        assert_eq!(inspect(&JsValue::String("a\"b".into())), r#""a\"b""#);
        assert_eq!(inspect(&JsValue::Number(1.5)), "1.5");
        let nested = JsValue::List(vec![JsValue::List(vec![JsValue::List(vec![
            JsValue::Null,
        ])])]);
        assert_eq!(inspect(&nested), "[[[Array]]]");
        let mut o = IndexMap::default();
        o.insert("a".to_string(), JsValue::Boolean(true));
        assert_eq!(inspect(&JsValue::Object(o)), "{ a: true }");
        let long = JsValue::List((0..12).map(|i| JsValue::Number(i as f64)).collect());
        assert_eq!(
            inspect(&long),
            "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, ... 2 more items]"
        );
    }
}
