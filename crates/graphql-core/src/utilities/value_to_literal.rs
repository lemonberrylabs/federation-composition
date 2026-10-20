//! Port of graphql-js `utilities/valueToLiteral`: `defaultScalarValueToLiteral`, which
//! [`GraphQLArgument::default_value`](crate::GraphQLArgument::default_value) uses for the
//! defaults given as values. (The typed `valueToLiteral` is not ported.)

use crate::ast::{
    BooleanValue, ConstValue, FloatValue, IntValue, ListValue, NullValue, ObjectField, ObjectValue, StringValue,
};
use crate::js::number_to_string;
use crate::jsutils::JsValue;

/// graphql-js `defaultScalarValueToLiteral(value)`.
pub fn default_scalar_value_to_literal(value: &JsValue) -> ConstValue {
    match value {
        JsValue::Null => ConstValue::Null(NullValue {}),
        JsValue::Boolean(value) => ConstValue::Boolean(BooleanValue { value: *value }),
        JsValue::String(value) => ConstValue::String(StringValue::new(value.clone(), false)),
        JsValue::Number(value) => {
            if !value.is_finite() {
                return ConstValue::Null(NullValue {});
            }
            let string_value = number_to_string(*value);
            // `/^-?(?:0|[1-9][0-9]*)$/`
            let digits = string_value.strip_prefix('-').unwrap_or(&string_value);
            let is_int = digits == "0"
                || (!digits.is_empty() && !digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit()));
            if is_int {
                ConstValue::Int(IntValue { value: string_value })
            } else {
                ConstValue::Float(FloatValue { value: string_value })
            }
        }
        JsValue::List(items) => ConstValue::List(ListValue {
            values: items.iter().map(default_scalar_value_to_literal).collect(),
        }),
        JsValue::Object(object) => ConstValue::Object(ObjectValue {
            fields: object
                .iter()
                .map(|(name, value)| ObjectField::new(name.clone(), default_scalar_value_to_literal(value)))
                .collect(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::IndexMap;
    use crate::printer::print;

    fn literal(value: JsValue) -> String {
        print(&default_scalar_value_to_literal(&value))
    }

    #[test]
    fn default_scalar_value_to_literal_like_graphql_js() {
        assert_eq!(literal(JsValue::Null), "null");
        assert_eq!(literal(JsValue::Boolean(false)), "false");
        assert_eq!(literal(JsValue::String("a \"b\"".to_string())), r#""a \"b\"""#);
        assert_eq!(literal(JsValue::Number(-0.0)), "0");
        assert_eq!(literal(JsValue::Number(-12.0)), "-12");
        assert_eq!(literal(JsValue::Number(1.5)), "1.5");
        assert_eq!(literal(JsValue::Number(1e21)), "1e+21");
        assert_eq!(literal(JsValue::Number(f64::INFINITY)), "null");
        assert_eq!(literal(JsValue::Number(f64::NAN)), "null");
        let object: IndexMap<String, JsValue> = [
            (
                "b".to_string(),
                JsValue::List(vec![JsValue::Number(1.0), JsValue::Null]),
            ),
            ("a".to_string(), JsValue::Boolean(true)),
        ]
        .into_iter()
        .collect();
        assert_eq!(literal(JsValue::Object(object)), "{ b: [1, null], a: true }");
    }
}
