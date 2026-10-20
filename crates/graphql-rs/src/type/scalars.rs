//! Port of graphql-js `type/scalars`: the specified scalar types and their
//! input coercion. (Output coercion and `valueToLiteral` are not used by
//! schema building or validation.)

use crate::ast::Value;
use crate::error::GraphQLError;
use crate::jsutils::inspect::inspect;
use crate::jsutils::JsValue;
use crate::printer::print;
use crate::specified::{js_parse_float, js_parse_int, specified_scalar_type, GRAPHQL_MAX_INT, GRAPHQL_MIN_INT};

use super::definition::{GraphQLScalarType, GraphQLScalarTypeConfig, ScalarKind};

/// The names of graphql-js `specifiedScalarTypes`, in graphql-js order.
pub const SPECIFIED_SCALAR_TYPE_NAMES: [&str; 5] = ["String", "Int", "Float", "Boolean", "ID"];

fn specified_scalar(name: &str, kind: ScalarKind) -> GraphQLScalarType {
    let description = specified_scalar_type(name).map(|t| t.description.to_string());
    GraphQLScalarType::with_kind(
        GraphQLScalarTypeConfig {
            name: name.to_string(),
            description,
            ..Default::default()
        },
        kind,
    )
    .expect("specified scalar names are valid")
}

/// graphql-js `GraphQLInt`.
pub fn graphql_int() -> GraphQLScalarType {
    specified_scalar("Int", ScalarKind::Int)
}

/// graphql-js `GraphQLFloat`.
pub fn graphql_float() -> GraphQLScalarType {
    specified_scalar("Float", ScalarKind::Float)
}

/// graphql-js `GraphQLString`.
pub fn graphql_string() -> GraphQLScalarType {
    specified_scalar("String", ScalarKind::String)
}

/// graphql-js `GraphQLBoolean`.
pub fn graphql_boolean() -> GraphQLScalarType {
    specified_scalar("Boolean", ScalarKind::Boolean)
}

/// graphql-js `GraphQLID`.
pub fn graphql_id() -> GraphQLScalarType {
    specified_scalar("ID", ScalarKind::Id)
}

/// graphql-js `specifiedScalarTypes`.
pub fn specified_scalar_types() -> Vec<GraphQLScalarType> {
    vec![
        graphql_string(),
        graphql_int(),
        graphql_float(),
        graphql_boolean(),
        graphql_id(),
    ]
}

/// graphql-js `isSpecifiedScalarType` (by name, as graphql-js compares).
// graphql-js: exported by `type/scalars`; the port calls the version in `specified`.
#[allow(dead_code)]
pub fn is_specified_scalar_type(name: &str) -> bool {
    SPECIFIED_SCALAR_TYPE_NAMES.contains(&name)
}

/// JS `Number.isInteger`.
fn is_integer(value: f64) -> bool {
    value.is_finite() && value.trunc() == value
}

/// The `coerceInputValue` of the specified scalar `kind`.
pub(crate) fn coerce_input_value(kind: ScalarKind, input_value: &JsValue) -> Result<JsValue, GraphQLError> {
    match kind {
        ScalarKind::Int => {
            if let JsValue::Number(value) = input_value {
                return coerce_int_from_number(*value);
            }
            Err(GraphQLError::new(format!(
                "Int cannot represent non-integer value: {}",
                inspect(input_value)
            )))
        }
        ScalarKind::Float => {
            if let JsValue::Number(value) = input_value {
                return coerce_float_from_number(*value);
            }
            Err(GraphQLError::new(format!(
                "Float cannot represent non numeric value: {}",
                inspect(input_value)
            )))
        }
        ScalarKind::String => match input_value {
            JsValue::String(_) => Ok(input_value.clone()),
            _ => Err(GraphQLError::new(format!(
                "String cannot represent a non string value: {}",
                inspect(input_value)
            ))),
        },
        ScalarKind::Boolean => match input_value {
            JsValue::Boolean(_) => Ok(input_value.clone()),
            _ => Err(GraphQLError::new(format!(
                "Boolean cannot represent a non boolean value: {}",
                inspect(input_value)
            ))),
        },
        ScalarKind::Id => match input_value {
            JsValue::String(_) => Ok(input_value.clone()),
            JsValue::Number(value) => coerce_id_from_number(*value),
            _ => Err(GraphQLError::new(format!(
                "ID cannot represent value: {}",
                inspect(input_value)
            ))),
        },
        ScalarKind::Custom => unreachable!("not a specified scalar"),
    }
}

/// The `coerceInputLiteral` of the specified scalar `kind`.
pub(crate) fn coerce_input_literal(kind: ScalarKind, value_node: &Value) -> Result<JsValue, GraphQLError> {
    match kind {
        ScalarKind::Int => {
            let Value::Int(int) = value_node else {
                return Err(GraphQLError::new(format!(
                    "Int cannot represent non-integer value: {}",
                    print(value_node)
                )));
            };
            let num = js_parse_int(&int.value);
            if num > GRAPHQL_MAX_INT as f64 || num < GRAPHQL_MIN_INT as f64 {
                return Err(GraphQLError::new(format!(
                    "Int cannot represent non 32-bit signed integer value: {}",
                    int.value
                )));
            }
            Ok(JsValue::Number(num))
        }
        ScalarKind::Float => match value_node {
            Value::Float(f) => Ok(JsValue::Number(js_parse_float(&f.value))),
            Value::Int(i) => Ok(JsValue::Number(js_parse_float(&i.value))),
            _ => Err(GraphQLError::new(format!(
                "Float cannot represent non numeric value: {}",
                print(value_node)
            ))),
        },
        ScalarKind::String => match value_node {
            Value::String(s) => Ok(JsValue::String(s.value.clone())),
            _ => Err(GraphQLError::new(format!(
                "String cannot represent a non string value: {}",
                print(value_node)
            ))),
        },
        ScalarKind::Boolean => match value_node {
            Value::Boolean(b) => Ok(JsValue::Boolean(b.value)),
            _ => Err(GraphQLError::new(format!(
                "Boolean cannot represent a non boolean value: {}",
                print(value_node)
            ))),
        },
        ScalarKind::Id => match value_node {
            Value::String(s) => Ok(JsValue::String(s.value.clone())),
            Value::Int(i) => Ok(JsValue::String(i.value.clone())),
            _ => Err(GraphQLError::new(format!(
                "ID cannot represent a non-string and non-integer value: {}",
                print(value_node)
            ))),
        },
        ScalarKind::Custom => unreachable!("not a specified scalar"),
    }
}

fn coerce_int_from_number(value: f64) -> Result<JsValue, GraphQLError> {
    if !is_integer(value) {
        return Err(GraphQLError::new(format!(
            "Int cannot represent non-integer value: {}",
            inspect(&JsValue::Number(value))
        )));
    }
    if value > GRAPHQL_MAX_INT as f64 || value < GRAPHQL_MIN_INT as f64 {
        return Err(GraphQLError::new(format!(
            "Int cannot represent non 32-bit signed integer value: {}",
            inspect(&JsValue::Number(value))
        )));
    }
    Ok(JsValue::Number(value))
}

fn coerce_float_from_number(value: f64) -> Result<JsValue, GraphQLError> {
    if !value.is_finite() {
        return Err(GraphQLError::new(format!(
            "Float cannot represent non numeric value: {}",
            inspect(&JsValue::Number(value))
        )));
    }
    Ok(JsValue::Number(value))
}

fn coerce_id_from_number(value: f64) -> Result<JsValue, GraphQLError> {
    if !is_integer(value) {
        return Err(GraphQLError::new(format!(
            "ID cannot represent value: {}",
            inspect(&JsValue::Number(value))
        )));
    }
    Ok(JsValue::String(crate::js::number_to_string(value)))
}
