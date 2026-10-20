//! Ports of the graphql-js validation rules in `specifiedSDLRules`. Each rule
//! `Rule(context): ASTVisitor` is a struct whose `new(context)` is the rule
//! body before `return { ... }` and whose `Visitor` impl is the returned
//! visitor object.

pub mod known_argument_names_rule;
pub mod known_directives_rule;
pub mod known_type_names_rule;
pub mod lone_schema_definition_rule;
pub mod possible_type_extensions_rule;
pub mod provided_required_arguments_rule;
pub mod unique_argument_definition_names_rule;
pub mod unique_argument_names_rule;
pub mod unique_directive_names_rule;
pub mod unique_directives_per_location_rule;
pub mod unique_enum_value_names_rule;
pub mod unique_field_definition_names_rule;
pub mod unique_input_field_names_rule;
pub mod unique_operation_types_rule;
pub mod unique_type_names_rule;

use crate::collections::IndexMap;

/// graphql-js `groupBy(list, keyFn)` (an `AccumulatorMap`).
pub(crate) fn group_by<'a, T>(list: &'a [T], key_fn: impl Fn(&'a T) -> &'a str) -> IndexMap<&'a str, Vec<&'a T>> {
    let mut result: IndexMap<&'a str, Vec<&'a T>> = IndexMap::default();
    for item in list {
        result.entry(key_fn(item)).or_default().push(item);
    }
    result
}
