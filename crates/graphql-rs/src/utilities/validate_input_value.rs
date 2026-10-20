//! Port of graphql-js `utilities/validateInputValue` (`validateInputValue`,
//! `validateInputLiteral`). `validateInputLiteral` is ported for the static
//! case (no variables), the only one schema building and validation use.

use crate::ast::Value;
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::jsutils::did_you_mean::did_you_mean;
use crate::jsutils::inspect::inspect;
use crate::jsutils::path::{add_path, PathItem};
use crate::jsutils::suggestion_list::suggestion_list;
use crate::jsutils::JsValue;
use crate::printer::print;
use crate::r#type::definition::{
    is_required_input_field, GraphQLInputObjectType, GraphQLNamedType, GraphQLType, TypeLookup,
};

/// The graphql-js `onError(error, path)` callback.
pub type OnError<'e> = dyn FnMut(GraphQLError, Vec<PathItem>) + 'e;

/// graphql-js `validateInputValue(inputValue, type, onError, hideSuggestions)`;
/// `None` is `undefined`.
pub fn validate_input_value(
    input_value: Option<&JsValue>,
    ty: &GraphQLType,
    on_error: &mut OnError<'_>,
    hide_suggestions: bool,
    types: &dyn TypeLookup,
) {
    validate_input_value_impl(input_value, ty, on_error, hide_suggestions, &[], types)
}

fn validate_input_value_impl(
    input_value: Option<&JsValue>,
    ty: &GraphQLType,
    on_error: &mut OnError<'_>,
    hide_suggestions: bool,
    path: &[PathItem],
    types: &dyn TypeLookup,
) {
    if let GraphQLType::NonNull(of_type) = ty {
        match input_value {
            None => {
                report_invalid_value(
                    on_error,
                    format!("Expected a value of non-null type \"{ty}\" to be provided."),
                    path,
                );
                return;
            }
            Some(JsValue::Null) => {
                report_invalid_value(
                    on_error,
                    format!("Expected value of non-null type \"{ty}\" not to be null."),
                    path,
                );
                return;
            }
            Some(_) => {}
        }
        return validate_input_value_impl(input_value, of_type, on_error, hide_suggestions, path, types);
    }
    let input_value = match input_value {
        None | Some(JsValue::Null) => return,
        Some(value) => value,
    };
    if let GraphQLType::List(of_type) = ty {
        match input_value {
            JsValue::List(items) => {
                for (index, item_value) in items.iter().enumerate() {
                    validate_input_value_impl(
                        Some(item_value),
                        of_type,
                        on_error,
                        hide_suggestions,
                        &add_path(path, PathItem::Index(index)),
                        types,
                    );
                }
            }
            _ => validate_input_value_impl(Some(input_value), of_type, on_error, hide_suggestions, path, types),
        }
        return;
    }
    let Some(named_type) = types.lookup_type(ty.named_type()) else {
        return;
    };
    match named_type {
        GraphQLNamedType::InputObject(input_object) => {
            let JsValue::Object(object) = input_value else {
                report_invalid_value(
                    on_error,
                    format!(
                        "Expected value of type \"{ty}\" to be an object, found: {}.",
                        inspect(input_value)
                    ),
                    path,
                );
                return;
            };
            let field_defs = input_object.fields();
            for field in field_defs.values() {
                match object.get(&field.name) {
                    None => {
                        if is_required_input_field(field) {
                            report_invalid_value(
                                on_error,
                                format!(
                                    "Expected value of type \"{ty}\" to include required field \"{}\", found: {}.",
                                    field.name,
                                    inspect(input_value)
                                ),
                                path,
                            );
                        }
                    }
                    Some(field_value) => validate_input_value_impl(
                        Some(field_value),
                        &field.ty,
                        on_error,
                        hide_suggestions,
                        &add_path(path, PathItem::Key(field.name.clone())),
                        types,
                    ),
                }
            }
            let mut fields: Vec<&str> = Vec::new();
            for field_name in object.keys() {
                if !field_defs.contains_key(field_name) {
                    let suggestion = if hide_suggestions {
                        String::new()
                    } else {
                        let names: Vec<&str> = field_defs.keys().map(String::as_str).collect();
                        did_you_mean(None, &suggestion_list(field_name, &names))
                    };
                    let tail = if suggestion.is_empty() {
                        ", found".to_string()
                    } else {
                        format!(".{suggestion} Found")
                    };
                    report_invalid_value(
                        on_error,
                        format!(
                            "Expected value of type \"{ty}\" not to include unknown field \"{field_name}\"{tail}: {}.",
                            inspect(input_value)
                        ),
                        path,
                    );
                    continue;
                }
                fields.push(field_name);
            }
            if input_object.is_one_of {
                if fields.len() != 1 {
                    report_invalid_value(on_error, get_one_of_input_object_error_message(input_object), path);
                }
                // `inputValue[fields[0]]`; with no field it is `undefined`.
                if let Some(field) = fields.first() {
                    if object.get(*field) == Some(&JsValue::Null) {
                        report_invalid_value(
                            on_error,
                            get_one_of_input_object_error_message(input_object),
                            &add_path(path, PathItem::Key(field.to_string())),
                        );
                    }
                }
            }
        }
        GraphQLNamedType::Scalar(scalar) => {
            // Coercion errors are `GraphQLError`s; a result is never
            // `undefined` for a defined input value.
            if let Err(error) = scalar.coerce_input_value(input_value) {
                on_error(error, path.to_vec());
            }
        }
        GraphQLNamedType::Enum(enum_type) => {
            if let Err(error) = enum_type.coerce_input_value(input_value, hide_suggestions) {
                on_error(error, path.to_vec());
            }
        }
        // TS: assertLeafType(type); never reached for input types.
        _ => {}
    }
}

fn report_invalid_value(on_error: &mut OnError<'_>, message: String, path: &[PathItem]) {
    on_error(GraphQLError::new(message), path.to_vec());
}

/// graphql-js `validateInputLiteral(valueNode, type, onError, undefined,
/// undefined, hideSuggestions)`: the static case, without variables.
pub fn validate_input_literal(
    value_node: &Value,
    ty: &GraphQLType,
    on_error: &mut OnError<'_>,
    hide_suggestions: bool,
    types: &dyn TypeLookup,
) {
    validate_input_literal_impl(value_node, ty, on_error, hide_suggestions, &[], types)
}

fn validate_input_literal_impl(
    value_node: &Value,
    ty: &GraphQLType,
    on_error: &mut OnError<'_>,
    hide_suggestions: bool,
    path: &[PathItem],
    types: &dyn TypeLookup,
) {
    if let Value::Variable(_) = value_node {
        // `context.static`.
        return;
    }
    if let GraphQLType::NonNull(of_type) = ty {
        if let Value::Null(_) = value_node {
            report_invalid_literal(
                on_error,
                format!("Expected value of non-null type \"{ty}\" not to be null."),
                path,
            );
            return;
        }
        return validate_input_literal_impl(value_node, of_type, on_error, hide_suggestions, path, types);
    }
    if let Value::Null(_) = value_node {
        return;
    }
    if let GraphQLType::List(of_type) = ty {
        match value_node {
            Value::List(list) => {
                for (index, item_node) in list.values.iter().enumerate() {
                    validate_input_literal_impl(
                        item_node,
                        of_type,
                        on_error,
                        hide_suggestions,
                        &add_path(path, PathItem::Index(index)),
                        types,
                    );
                }
            }
            _ => validate_input_literal_impl(value_node, of_type, on_error, hide_suggestions, path, types),
        }
        return;
    }
    let Some(named_type) = types.lookup_type(ty.named_type()) else {
        return;
    };
    match named_type {
        GraphQLNamedType::InputObject(input_object) => {
            let Value::Object(object) = value_node else {
                report_invalid_literal(
                    on_error,
                    format!(
                        "Expected value of type \"{ty}\" to be an object, found: {}.",
                        print(value_node)
                    ),
                    path,
                );
                return;
            };
            let field_defs = input_object.fields();
            // `keyMap`: the last field of a name wins.
            let mut field_nodes = IndexMap::default();
            for field in &object.fields {
                field_nodes.insert(field.name.value.as_str(), field);
            }
            for field in field_defs.values() {
                match field_nodes.get(field.name.as_str()) {
                    None => {
                        if is_required_input_field(field) {
                            report_invalid_literal(
                                on_error,
                                format!(
                                    "Expected value of type \"{ty}\" to include required field \"{}\", found: {}.",
                                    field.name,
                                    print(value_node)
                                ),
                                path,
                            );
                        }
                    }
                    Some(field_node) => validate_input_literal_impl(
                        &field_node.value,
                        &field.ty,
                        on_error,
                        hide_suggestions,
                        &add_path(path, PathItem::Key(field.name.clone())),
                        types,
                    ),
                }
            }
            let mut known_fields = Vec::new();
            for field_node in &object.fields {
                let field_name = &field_node.name.value;
                if !field_defs.contains_key(field_name) {
                    let suggestion = if hide_suggestions {
                        String::new()
                    } else {
                        let names: Vec<&str> = field_defs.keys().map(String::as_str).collect();
                        did_you_mean(None, &suggestion_list(field_name, &names))
                    };
                    let tail = if suggestion.is_empty() {
                        ", found".to_string()
                    } else {
                        format!(".{suggestion} Found")
                    };
                    report_invalid_literal(
                        on_error,
                        format!(
                            "Expected value of type \"{ty}\" not to include unknown field \"{field_name}\"{tail}: {}.",
                            print(value_node)
                        ),
                        path,
                    );
                } else {
                    known_fields.push(field_node);
                }
            }
            if input_object.is_one_of {
                if known_fields.len() != 1 {
                    report_invalid_literal(on_error, get_one_of_input_object_error_message(input_object), path);
                    return;
                }
                if let Value::Null(_) = known_fields[0].value {
                    let field_name = &known_fields[0].name.value;
                    report_invalid_literal(
                        on_error,
                        get_one_of_input_object_error_message(input_object),
                        &add_path(path, PathItem::Key(field_name.clone())),
                    );
                }
            }
        }
        GraphQLNamedType::Scalar(scalar) => {
            let result = if scalar.has_coerce_input_literal() {
                scalar.coerce_input_literal(value_node).map(Some)
            } else {
                scalar.parse_literal(value_node)
            };
            match result {
                // `catch (error) { if (error instanceof GraphQLError) ... }`:
                // scalar coercion throws only `GraphQLError`s.
                Err(error) => on_error(error, path.to_vec()),
                Ok(None) => report_invalid_literal(
                    on_error,
                    format!("Expected value of type \"{ty}\", found: {}.", print(value_node)),
                    path,
                ),
                Ok(Some(_)) => {}
            }
        }
        GraphQLNamedType::Enum(enum_type) => {
            // NOTE(port): what the enum can throw (its own errors, or an
            // error thrown while resolving its values) is a `GraphQLError`.
            if let Err(error) = enum_type.coerce_input_literal(value_node, hide_suggestions) {
                on_error(error, path.to_vec());
            }
        }
        // TS: assertLeafType(type); never reached for input types.
        _ => {}
    }
}

fn report_invalid_literal(on_error: &mut OnError<'_>, message: String, path: &[PathItem]) {
    on_error(GraphQLError::new(message), path.to_vec());
}

/// graphql-js `getOneOfInputObjectErrorMessage`.
fn get_one_of_input_object_error_message(ty: &GraphQLInputObjectType) -> String {
    format!(
        "Within OneOf Input Object type \"{}\", exactly one field must be specified, and the value for that field must be non-null.",
        ty.name
    )
}
