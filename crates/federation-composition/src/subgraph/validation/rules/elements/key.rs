//! Port of `src/subgraph/validation/rules/elements/key.ts`.

use std::rc::Rc;

use crate::graphql::printer::print;
use crate::subgraph::helpers::{
    get_fields_argument, parse_fields, validate_directive_against_original, visit_fields, InterceptDirectiveInfo,
    InterceptFieldInfo, Intercepts, ObjectOrInterface, VisitFieldsInterceptor,
};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::collections::IndexSet;
use crate::utils::js::{JsError, JsResult};
use graphql_core::{Directive, GraphQLError, NodeRef, Value, VisitAction, VisitInfo, Visitor};

use super::{finish, is_interface_kind, is_object_kind, node_name};

/// `KeyRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(directiveNode)`. The `visitFields` interceptors become a local struct implementing `VisitFieldsInterceptor`.
/// Fields: the locals of the TS factory.
pub struct KeyRules {}

impl<'a> SubgraphRule<'a> for KeyRules {
    const NAME: &'static str = "KeyRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        KeyRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for KeyRules {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_core::VisitKinds {
        use graphql_core::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut SubgraphValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "key", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `visitFields` interceptors of the `Directive` callback, with the locals they share.
struct KeyFieldsInterceptor<'r> {
    /// `typeDef.name.value` (also `typeCoordinate`).
    type_name: &'r str,
    printed_fields_value: &'r str,
    is_valid: bool,
    fields_used_in_key: IndexSet<String>,
}

impl<'a> VisitFieldsInterceptor<'a> for KeyFieldsInterceptor<'_> {
    fn intercept_field(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        // Mark the field as used in the key when it's a field of the type annotated with @key
        if info.type_definition.name.value == self.type_name {
            self.fields_used_in_key.insert(info.field_name.clone());
        }
        context.mark_as_key_field(&format!("{}.{}", info.type_definition.name.value, info.field_name));
        Ok(())
    }

    fn intercept_unknown_field(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        self.is_valid = false;
        context.report_error(GraphQLError::with_code(
            format!(
                "On type \"{}\", for @key(fields: {}): Cannot query field \"{}\" on type \"{}\" (the field should either be added to this subgraph or, if it should not be resolved by this subgraph, you need to add it to this subgraph with @external).",
                self.type_name, self.printed_fields_value, info.field_name, info.type_definition.name.value
            ),
            "KEY_INVALID_FIELDS",
        ));
        Ok(())
    }

    fn intercept_directive(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptDirectiveInfo,
    ) -> JsResult<()> {
        self.is_valid = false;
        if info.is_known {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On type \"{}\", for @key(fields: {}): cannot have directive applications in the @key(fields:) argument but found @{}.",
                    self.type_name, self.printed_fields_value, info.directive_name
                ),
                "KEY_DIRECTIVE_IN_FIELDS_ARG",
            ));
        } else {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On type \"{}\", for @key(fields: {}): Unknown directive \"@{}\"",
                    self.type_name, self.printed_fields_value, info.directive_name
                ),
                "KEY_INVALID_FIELDS",
            ));
        }
        Ok(())
    }

    fn intercept_arguments(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        self.is_valid = false;
        context.report_error(GraphQLError::with_code(
            format!(
                "On type \"{}\", for @key(fields: {}): field {}.{} cannot be included because it has arguments (fields with argument are not allowed in @key)",
                self.type_name, self.printed_fields_value, info.type_definition.name.value, info.field_name
            ),
            "KEY_FIELDS_HAS_ARGS",
        ));
        Ok(())
    }

    fn intercept_interface_type(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        self.is_valid = false;
        context.report_error(GraphQLError::with_code(
            format!(
                "On type \"{}\", for @key(fields: {}): field \"{}.{}\" is a Interface type which is not allowed in @key",
                self.type_name, self.printed_fields_value, info.type_definition.name.value, info.field_name
            ),
            "KEY_FIELDS_SELECT_INVALID_TYPE",
        ));
        Ok(())
    }
}

/// The `Directive(directiveNode)` callback.
fn directive<'a>(directive_node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("key", &directive_node.name.value) {
        return Ok(());
    }

    let type_def = context.type_node_info.get_type_def();

    // `@key` needs to be used on an object type or an interface type
    let Some(type_def) = type_def else {
        // Let regular validation handle this
        return Ok(());
    };

    let type_name = node_name(type_def);
    let type_coordinate = type_name;

    let used_on_object = is_object_kind(type_def.kind());
    let used_on_interface =
        is_interface_kind(type_def.kind()) || (used_on_object && context.state_builder.is_interface_object(type_name));

    if !used_on_object && !used_on_interface {
        return Ok(()); // Let regular validation handle this
    }

    if used_on_interface && context.satisfies_version_range("> v1.0") && context.satisfies_version_range("< v2.3") {
        context.report_error(GraphQLError::with_code(
            format!("Cannot use @key on interface \"{type_coordinate}\": @key is not yet supported on interfaces"),
            "KEY_UNSUPPORTED_ON_INTERFACE",
        ));
        return Ok(());
    }

    let Some(fields_arg) = get_fields_argument(directive_node) else {
        return Ok(());
    };

    let printed_fields_value = print(&fields_arg.value);

    if !matches!(fields_arg.value, Value::String(_)) {
        // In Federation v1, a string ["id", "name"] is equal to "id name" (that is not true for Federation v2)
        let is_list_with_strings = match &fields_arg.value {
            Value::List(list) => list.values.iter().all(|value| matches!(value, Value::String(_))),
            _ => false,
        };
        if context.satisfies_version_range("> v1.0") || !is_list_with_strings {
            // V2
            context.report_error(GraphQLError::with_code(
                format!(
                    "On type \"{type_coordinate}\", for @key(fields: {printed_fields_value}): Invalid value for argument \"fields\": must be a string."
                ),
                "KEY_INVALID_FIELDS_TYPE",
            ));
            return Ok(());
        }
    }

    let normalized_fields_arg_value: String = match &fields_arg.value {
        Value::String(value) => value.value.clone(),
        Value::List(list) => {
            let mut values = Vec::with_capacity(list.values.len());
            for v in &list.values {
                let Value::String(v) = v else {
                    // We checked if it's a string before, it should be at this point
                    return Err(JsError::error("Expected fields argument value to be a string"));
                };
                values.push(v.value.as_str());
            }
            values.join(" ")
        }
        // NOTE(port): unreachable (a non-string, non-list value returned above); the TS would
        // throw a TypeError reading `.values.map` here.
        _ => return Err(JsError::error("Expected fields argument value to be a string")),
    };

    let selection_set = match parse_fields(&normalized_fields_arg_value) {
        Ok(selection_set) => selection_set,
        Err(JsError::GraphQL(error)) => {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On type \"{type_coordinate}\", for @key(fields: {printed_fields_value}): {}",
                    error.message
                ),
                "KEY_INVALID_FIELDS",
            ));
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    let Some(selection_set) = selection_set else {
        // error already reported
        return Ok(());
    };

    let Some(merged_type_def) = context.get_subgraph_object_or_interface_types().get(type_name).cloned() else {
        return Err(JsError::error(format!("Could not find type \"{type_name}\"")));
    };

    let mut interceptor = KeyFieldsInterceptor {
        type_name,
        printed_fields_value: &printed_fields_value,
        is_valid: true,
        fields_used_in_key: IndexSet::default(),
    };

    visit_fields(
        context,
        &selection_set,
        &merged_type_def,
        &mut interceptor,
        Intercepts {
            field: true,
            unknown_field: true,
            directive: true,
            arguments: true,
            interface_type: true,
            ..Intercepts::default()
        },
    )?;

    let KeyFieldsInterceptor {
        mut is_valid,
        fields_used_in_key,
        ..
    } = interceptor;

    // check if the key directive from the interface is implemented on all implementations

    if used_on_interface {
        // TODO: It's a poor's man implementation of the check, we should compare the selected fields, but just a string
        let expected_fields_value = &normalized_fields_arg_value;
        let known_objects_and_interfaces: Vec<Rc<ObjectOrInterface>> = context
            .get_subgraph_object_or_interface_types()
            .values()
            .cloned()
            .collect();
        for def in known_objects_and_interfaces {
            let def_is_object_type = is_object_kind(def.kind);

            if def_is_object_type && def.interfaces.iter().any(|i| i.name.value == type_name) {
                let mut should_error = true;
                let key_directives: Vec<&Directive> = def
                    .directives
                    .iter()
                    .filter(|d| context.is_available_federation_directive("key", &d.name.value))
                    .collect();

                for key_directive in key_directives {
                    let fields_arg = get_fields_argument(key_directive);

                    if let Some(fields_arg) = fields_arg {
                        if let Value::String(value) = &fields_arg.value {
                            if &value.value == expected_fields_value {
                                should_error = false;
                            }
                        }
                    }
                }

                if should_error && context.satisfies_version_range("> v1.0") {
                    is_valid = false;
                    context.report_error(GraphQLError::with_code(
                        format!(
                            "Key @key(fields: {printed_fields_value}) on interface type \"{type_name}\" is missing on implementation type \"{}\".",
                            def.name.value
                        ),
                        "INTERFACE_KEY_NOT_ON_IMPLEMENTATION",
                    ));
                }
            }
        }
    }

    if is_valid {
        let resolvable_arg_value = directive_node
            .arguments
            .iter()
            .find(|arg| arg.name.value == "resolvable" && matches!(arg.value, Value::Boolean(_)))
            .and_then(|arg| match &arg.value {
                Value::Boolean(value) => Some(value.value),
                _ => None,
            });

        let resolvable = resolvable_arg_value.unwrap_or(true);

        if used_on_interface {
            context.state_builder.interface_type().set_key(
                type_name,
                &normalized_fields_arg_value,
                &fields_used_in_key,
                resolvable,
            )?;
            return Ok(());
        }

        context.state_builder.object_type().set_key(
            type_name,
            &normalized_fields_arg_value,
            &fields_used_in_key,
            resolvable,
        )?;
    }

    Ok(())
}
