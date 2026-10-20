//! Port of graphql-js `execution/values`: `getDirectiveValues` and
//! `getArgumentValues`, as schema building uses them (directives applied in
//! SDL, so without variables).

use crate::ast::{Directive, Value};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::jsutils::path::PathItem;
use crate::jsutils::print_path_array::print_path_array;
use crate::jsutils::JsValue;
use crate::r#type::definition::{is_required_argument, DefaultInput, GraphQLArgument, TypeLookup};
use crate::r#type::directives::GraphQLDirective;
use crate::utilities::coerce_input_value::{coerce_default_value, coerce_input_literal};
use crate::utilities::validate_input_value::{validate_input_literal, validate_input_value};

/// graphql-js `getArgumentValues(def, node)` for a directive node, without
/// variables.
pub fn get_argument_values(
    def: &GraphQLDirective,
    node: &Directive,
    types: &dyn TypeLookup,
) -> Result<IndexMap<String, JsValue>, GraphQLError> {
    let mut coerced_values = IndexMap::default();
    // `new Map(argumentNodes.map(arg => [arg.name.value, arg]))`: the last
    // argument of a name wins.
    let mut arg_node_map = IndexMap::default();
    for arg in &node.arguments {
        arg_node_map.insert(arg.name.value.as_str(), arg);
    }
    for arg_def in &def.args {
        coerce_argument(
            &mut coerced_values,
            &arg_def.name,
            arg_def,
            arg_node_map
                .get(arg_def.name.as_str())
                .map(|arg| &arg.value),
            types,
        )?;
    }
    Ok(coerced_values)
}

/// graphql-js `coerceArgument`, without variables.
fn coerce_argument(
    coerced_values: &mut IndexMap<String, JsValue>,
    arg_name: &str,
    arg_def: &GraphQLArgument,
    argument_value: Option<&Value>,
    types: &dyn TypeLookup,
) -> Result<(), GraphQLError> {
    let arg_type = &arg_def.ty;
    let Some(value_node) = argument_value else {
        if is_required_argument(arg_def) {
            return Err(GraphQLError::new(format!(
                "{} of required type \"{arg_type}\" was not provided.",
                print_argument_or_fragment_variable(arg_def)
            )));
        }
        return maybe_use_default_value(coerced_values, arg_name, arg_def, types);
    };
    if let Value::Variable(_) = value_node {
        // No variable has a value.
        if !is_required_argument(arg_def) {
            return maybe_use_default_value(coerced_values, arg_name, arg_def, types);
        }
    }
    let Some(coerced_value) = coerce_input_literal(value_node, arg_type, types)? else {
        // The `onError` callback throws the first error, with its message
        // prefixed.
        let mut errors: Vec<(GraphQLError, Vec<PathItem>)> = Vec::new();
        validate_input_literal(
            value_node,
            arg_type,
            &mut |error, path| errors.push((error, path)),
            false,
            types,
        );
        if let Some((mut error, path)) = errors.into_iter().next() {
            error.message = format!(
                "{} has invalid value{}: {}",
                print_argument_or_fragment_variable(arg_def),
                print_path_array(&path),
                error.message
            );
            return Err(error);
        }
        return Err(GraphQLError::new("Invalid argument"));
    };
    coerced_values.insert(arg_name.to_string(), coerced_value);
    Ok(())
}

/// graphql-js `maybeUseDefaultValue` for an argument: its `onError` throws.
fn maybe_use_default_value(
    coerced_values: &mut IndexMap<String, JsValue>,
    name: &str,
    input_value: &GraphQLArgument,
    types: &dyn TypeLookup,
) -> Result<(), GraphQLError> {
    match coerce_default_value(input_value, types) {
        Ok(Some(coerced_default_value)) => {
            coerced_values.insert(name.to_string(), coerced_default_value);
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(error) => {
            if input_value.default.is_none() {
                return Err(error);
            }
            // `validateDefaultInput`; the first reported error is thrown.
            let mut first: Option<(GraphQLError, Vec<PathItem>)> = None;
            let mut on_error = |error, path| {
                if first.is_none() {
                    first = Some((error, path));
                }
            };
            match &input_value.default {
                Some(DefaultInput::Literal(literal)) => {
                    validate_input_literal(literal, &input_value.ty, &mut on_error, false, types)
                }
                Some(DefaultInput::Value(value)) => {
                    validate_input_value(Some(value), &input_value.ty, &mut on_error, false, types)
                }
                None => {}
            }
            let (error, path) = first.unwrap_or((error, Vec::new()));
            Err(GraphQLError::new(format!(
                "{} has invalid default value{}: {}",
                print_argument_or_fragment_variable(input_value),
                print_path_array(&path),
                error.message
            )))
        }
    }
}

/// graphql-js `printArgumentOrFragmentVariable` for an argument.
fn print_argument_or_fragment_variable(arg_def: &GraphQLArgument) -> String {
    format!("Argument \"{arg_def}\"")
}

/// graphql-js `getDirectiveValues(directiveDef, node)`: the argument values of
/// the first directive named like `directive_def` among `directives` (the
/// node's `directives`), or `None` (`undefined`) when there is none.
pub fn get_directive_values(
    directive_def: &GraphQLDirective,
    directives: &[Directive],
    types: &dyn TypeLookup,
) -> Result<Option<IndexMap<String, JsValue>>, GraphQLError> {
    let directive_node = directives
        .iter()
        .find(|directive| directive.name.value == directive_def.name);
    match directive_node {
        Some(directive_node) => get_argument_values(directive_def, directive_node, types).map(Some),
        None => Ok(None),
    }
}
