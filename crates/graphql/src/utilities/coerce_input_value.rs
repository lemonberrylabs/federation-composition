//! Port of graphql-js `utilities/coerceInputValue` (`coerceInputValue`,
//! `coerceInputLiteral`, `coerceDefaultValue`), without variables: the values
//! coerced while building a schema are SDL values, which are constant.
//!
//! `Ok(None)` is graphql-js returning `undefined` (the value is invalid);
//! `Err` is a throw (`coerceDefaultValue`'s invariant).

use crate::ast::Value;
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::jsutils::JsValue;
use crate::printer::print;
use crate::r#type::definition::{
    is_required_input_field, DefaultInput, GraphQLInputValue, GraphQLNamedType, GraphQLType,
    TypeLookup,
};

/// graphql-js `coerceInputValue(inputValue, type)`.
pub fn coerce_input_value(
    input_value: &JsValue,
    ty: &GraphQLType,
    types: &dyn TypeLookup,
) -> Result<Option<JsValue>, GraphQLError> {
    if let GraphQLType::NonNull(of_type) = ty {
        if *input_value == JsValue::Null {
            return Ok(None);
        }
        return coerce_input_value(input_value, of_type, types);
    }
    if *input_value == JsValue::Null {
        return Ok(Some(JsValue::Null));
    }
    if let GraphQLType::List(of_type) = ty {
        let JsValue::List(items) = input_value else {
            return Ok(coerce_input_value(input_value, of_type, types)?
                .map(|coerced_item| JsValue::List(vec![coerced_item])));
        };
        let mut coerced_value = Vec::with_capacity(items.len());
        for item_value in items {
            let Some(coerced_item) = coerce_input_value(item_value, of_type, types)? else {
                return Ok(None);
            };
            coerced_value.push(coerced_item);
        }
        return Ok(Some(JsValue::List(coerced_value)));
    }
    let Some(named_type) = types.lookup_type(ty.named_type()) else {
        return Ok(None);
    };
    match named_type {
        GraphQLNamedType::InputObject(input_object) => {
            let JsValue::Object(input_value) = input_value else {
                return Ok(None);
            };
            let mut coerced_value = IndexMap::default();
            let field_defs = input_object.get_fields()?;
            let mut defined_field_count = 0;
            for field_name in input_value.keys() {
                defined_field_count += 1;
                if !field_defs.contains_key(field_name) {
                    return Ok(None);
                }
            }
            for field in field_defs.values() {
                match input_value.get(&field.name) {
                    None => {
                        if is_required_input_field(field) {
                            return Ok(None);
                        }
                        if let Some(coerced_default_value) = coerce_default_value(field, types)? {
                            coerced_value.insert(field.name.clone(), coerced_default_value);
                        }
                    }
                    Some(field_value) => {
                        let Some(coerced_field) =
                            coerce_input_value(field_value, &field.ty, types)?
                        else {
                            return Ok(None);
                        };
                        coerced_value.insert(field.name.clone(), coerced_field);
                    }
                }
            }
            if input_object.is_one_of {
                if defined_field_count != 1 || coerced_value.len() != 1 {
                    return Ok(None);
                }
                if coerced_value.values().next() == Some(&JsValue::Null) {
                    return Ok(None);
                }
            }
            Ok(Some(JsValue::Object(coerced_value)))
        }
        // `try { return leafType.coerceInputValue(inputValue) } catch {}`.
        GraphQLNamedType::Scalar(scalar) => Ok(scalar.coerce_input_value(input_value).ok()),
        GraphQLNamedType::Enum(enum_type) => {
            Ok(enum_type.coerce_input_value(input_value, false).ok())
        }
        // TS: assertLeafType(type) throws for the output types, which are
        // never input value types.
        _ => Err(GraphQLError::new(format!(
            "Expected {} to be a GraphQL leaf type.",
            named_type.name()
        ))),
    }
}

/// graphql-js `coerceInputLiteral(valueNode, type)` without variables.
pub fn coerce_input_literal(
    value_node: &Value,
    ty: &GraphQLType,
    types: &dyn TypeLookup,
) -> Result<Option<JsValue>, GraphQLError> {
    if let Value::Variable(_) = value_node {
        // No variable has a value: `undefined`, or `null` for a nullable type
        // (`coercedVariableValue == null && isNonNullType(type)` returns
        // `undefined`; otherwise the `undefined` value itself is returned).
        return Ok(None);
    }
    if let GraphQLType::NonNull(of_type) = ty {
        if let Value::Null(_) = value_node {
            return Ok(None);
        }
        return coerce_input_literal(value_node, of_type, types);
    }
    if let Value::Null(_) = value_node {
        return Ok(Some(JsValue::Null));
    }
    if let GraphQLType::List(of_type) = ty {
        let Value::List(list) = value_node else {
            return Ok(coerce_input_literal(value_node, of_type, types)?
                .map(|item_value| JsValue::List(vec![item_value])));
        };
        let mut coerced_value = Vec::with_capacity(list.values.len());
        for item_node in &list.values {
            let item_value = match coerce_input_literal(item_node, of_type, types)? {
                Some(item_value) => item_value,
                None => {
                    // A variable without a value is `null` in a list of a
                    // nullable type.
                    if matches!(item_node, Value::Variable(_)) && !of_type.is_non_null() {
                        JsValue::Null
                    } else {
                        return Ok(None);
                    }
                }
            };
            coerced_value.push(item_value);
        }
        return Ok(Some(JsValue::List(coerced_value)));
    }
    let Some(named_type) = types.lookup_type(ty.named_type()) else {
        return Ok(None);
    };
    match named_type {
        GraphQLNamedType::InputObject(input_object) => {
            let Value::Object(object) = value_node else {
                return Ok(None);
            };
            let mut coerced_value = IndexMap::default();
            let field_defs = input_object.get_fields()?;
            let has_undefined_field = object
                .fields
                .iter()
                .any(|field| !field_defs.contains_key(&field.name.value));
            if has_undefined_field {
                return Ok(None);
            }
            // `new Map(fields.map(f => [f.name.value, f]))`: the last field of
            // a name wins.
            let mut field_nodes = IndexMap::default();
            for field in &object.fields {
                field_nodes.insert(field.name.value.as_str(), field);
            }
            for field in field_defs.values() {
                let field_node = field_nodes.get(field.name.as_str());
                match field_node {
                    Some(field_node) if !matches!(field_node.value, Value::Variable(_)) => {
                        let Some(field_value) =
                            coerce_input_literal(&field_node.value, &field.ty, types)?
                        else {
                            return Ok(None);
                        };
                        coerced_value.insert(field.name.clone(), field_value);
                    }
                    // Missing, or a variable without a value.
                    _ => {
                        if is_required_input_field(field) {
                            return Ok(None);
                        }
                        if let Some(coerced_default_value) = coerce_default_value(field, types)? {
                            coerced_value.insert(field.name.clone(), coerced_default_value);
                        }
                    }
                }
            }
            if input_object.is_one_of {
                if field_nodes.len() != 1 || coerced_value.len() != 1 {
                    return Ok(None);
                }
                for (field_name, field_node) in &field_nodes {
                    if matches!(field_node.value, Value::Null(_))
                        || coerced_value.get(*field_name) == Some(&JsValue::Null)
                    {
                        return Ok(None);
                    }
                }
            }
            Ok(Some(JsValue::Object(coerced_value)))
        }
        // `try { ... } catch {}`. NOTE(port): `replaceVariables` returns the
        // node unchanged: SDL values are constant.
        GraphQLNamedType::Scalar(scalar) => Ok(if scalar.has_coerce_input_literal() {
            scalar.coerce_input_literal(value_node).ok()
        } else {
            scalar.parse_literal(value_node).ok().flatten()
        }),
        GraphQLNamedType::Enum(enum_type) => {
            Ok(enum_type.coerce_input_literal(value_node, false).ok())
        }
        _ => Err(GraphQLError::new(format!(
            "Expected {} to be a GraphQL leaf type.",
            named_type.name()
        ))),
    }
}

/// graphql-js `coerceDefaultValue(inputValue)`.
///
/// NOTE(port): graphql-js memoizes the result on the input value; coercion
/// is pure, so it is computed again instead.
pub fn coerce_default_value(
    input_value: &dyn GraphQLInputValue,
    types: &dyn TypeLookup,
) -> Result<Option<JsValue>, GraphQLError> {
    let Some(default_input) = input_value.default_input() else {
        // The legacy `defaultValue` is never set.
        return Ok(None);
    };
    let coerced_default_value = match default_input {
        DefaultInput::Literal(literal) => {
            coerce_input_literal(literal, input_value.input_type(), types)?
        }
        DefaultInput::Value(value) => coerce_input_value(value, input_value.input_type(), types)?,
    };
    match coerced_default_value {
        Some(value) => Ok(Some(value)),
        None => {
            // NOTE(port): graphql-js `inspect`s the literal AST node object
            // (`{ kind: ..., value: ... }`); it is printed here. Unreachable
            // for schemas built from SDL: defaults are validated first.
            let found = match default_input {
                DefaultInput::Literal(literal) => print(literal),
                DefaultInput::Value(value) => crate::jsutils::inspect::inspect(value),
            };
            Err(GraphQLError::new(format!(
                "Expected value of type \"{}\" to be valid, found: {found}.",
                input_value.input_type()
            )))
        }
    }
}
