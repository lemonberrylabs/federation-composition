//! Port of `src/subgraph/validation/rules/elements/list-size.ts`.

use graphql_js::{Argument, Directive, GraphQLError, NodeRef, Value, VisitAction, VisitInfo, Visitor};
use crate::subgraph::helpers::{named_type_from_type_node, validate_directive_against_original};
use crate::subgraph::state::{CostSpecDirective, ListSize};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{parse_int, JsError, JsResult};
use graphql_js::Type;

use super::{find_annotated_node, finish, is_interface_kind, is_object_kind, node_name};

/// `ListSizeRule(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct ListSizeRule {}

impl<'a> SubgraphRule<'a> for ListSizeRule {
    const NAME: &'static str = "ListSizeRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        ListSizeRule {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ListSizeRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "listSize", cx),
            NodeRef::Directive(node) => directive(node, cx, info),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The values of a list of `StringValue`s, or `None` when one of them is not a string.
fn string_values(values: &[Value]) -> Option<Vec<String>> {
    values
        .iter()
        .map(|val| match val {
            Value::String(val) => Some(val.value.clone()),
            _ => None,
        })
        .collect()
}

/// The `Directive(node, _key, _parent, paths, ancestors)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> JsResult<()> {
    if !context.is_available_federation_directive("listSize", &node.name.value) {
        return Ok(());
    }

    let mut assumed_size_arg: Option<&Argument> = None;
    let mut assumed_size: Option<f64> = None;
    let mut slicing_arguments_arg: Option<&Argument> = None;
    let mut slicing_arguments: Option<Vec<String>> = None;
    let mut sized_fields_arg: Option<&Argument> = None;
    let mut sized_fields: Option<Vec<String>> = None;
    let mut require_one_slicing_argument_arg: Option<&Argument> = None;
    let mut require_one_slicing_argument = true;

    for arg in &node.arguments {
        if arg.name.value == "assumedSize" {
            assumed_size_arg = Some(arg);
        } else if arg.name.value == "slicingArguments" {
            slicing_arguments_arg = Some(arg);
        } else if arg.name.value == "sizedFields" {
            sized_fields_arg = Some(arg);
        } else if arg.name.value == "requireOneSlicingArgument" {
            require_one_slicing_argument_arg = Some(arg);
        }
    }

    if let Some(assumed_size_arg) = assumed_size_arg {
        let Value::Int(value) = &assumed_size_arg.value else {
            context.report_error(GraphQLError::with_code(
                "The value of @listSize(assumedSize:) must be an integer",
                "LIST_SIZE_INVALID_ASSUMED_SIZE",
            ));
            return Ok(());
        };

        // NOTE(port): the TS catches a throw from `parseInt` and reports "The value of
        // @listSize(assumedSize:) is not a valid integer", but `parseInt` never throws.
        assumed_size = Some(parse_int(&value.value));
    }

    if let Some(slicing_arguments_arg) = slicing_arguments_arg {
        let Value::List(list) = &slicing_arguments_arg.value else {
            context.report_error(GraphQLError::with_code(
                "The value of @listSize(slicingArguments:) must be a list of strings",
                "LIST_SIZE_INVALID_SLICING_ARGUMENT",
            ));
            return Ok(());
        };

        let Some(values) = string_values(&list.values) else {
            context.report_error(GraphQLError::with_code(
                "The value of @listSize(slicingArguments:) must be a list of strings",
                "LIST_SIZE_INVALID_SLICING_ARGUMENT",
            ));
            return Ok(());
        };

        slicing_arguments = Some(values);
    }

    if let Some(sized_fields_arg) = sized_fields_arg {
        let Value::List(list) = &sized_fields_arg.value else {
            context.report_error(GraphQLError::with_code(
                "The value of @listSize(sizedFields:) must be a list of strings",
                "LIST_SIZE_INVALID_SIZED_FIELD",
            ));
            return Ok(());
        };

        let Some(values) = string_values(&list.values) else {
            context.report_error(GraphQLError::with_code(
                "The value of @listSize(sizedFields:) must be a list of strings",
                "LIST_SIZE_INVALID_SIZED_FIELD",
            ));
            return Ok(());
        };

        sized_fields = Some(values);
    }

    if let Some(require_one_slicing_argument_arg) = require_one_slicing_argument_arg {
        let Value::Boolean(value) = &require_one_slicing_argument_arg.value else {
            context.report_error(GraphQLError::with_code(
                "The value of @listSize(requireOneSlicingArgument:) must be a boolean when defined",
                "LIST_SIZE_INVALID_REQUIRE_ONE_SLICING_ARGUMENT",
            ));
            return Ok(());
        };

        require_one_slicing_argument = value.value;
    }

    let parent = find_annotated_node(info, "Could not find the node annotated with @listSize")?;

    context
        .state_builder
        .mark_cost_spec_as_used(CostSpecDirective::ListSize, &node.name.value);

    if let NodeRef::FieldDefinition(field_def) = parent {
        let Some(type_def) = context.type_node_info.get_type_def() else {
            return Err(JsError::error(
                "Could not find the parent type of the field annotated with @listSize",
            ));
        };

        let coordinate = format!("\"{}.{}\"", node_name(type_def), field_def.name.value);

        // NOTE(port): the TS repeats this check twice.
        if matches!(assumed_size, Some(size) if size < 0.0) {
            context.report_error(GraphQLError::with_code(
                format!("{coordinate} has negative @listSize(assumedSize:) value"),
                "LIST_SIZE_INVALID_ASSUMED_SIZE",
            ));
            return Ok(());
        }

        if sized_fields.as_ref().is_none_or(|fields| fields.is_empty()) && !is_list_type(&field_def.ty) {
            context.report_error(GraphQLError::with_code(
                format!("{coordinate} is not a list. Try to add @listSize(sizedFields:) argument."),
                "LIST_SIZE_INVALID_SIZED_FIELD",
            ));
            return Ok(());
        }

        if let Some(sized_fields) = sized_fields.as_ref().filter(|fields| !fields.is_empty()) {
            let output_type = named_type_from_type_node(&field_def.ty);
            let target_type = context
                .get_subgraph_object_or_interface_types()
                .get(&output_type.name.value)
                .cloned();

            let Some(target_type) = target_type else {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "{coordinate} has @listSize(sizedFields:) applied, but the output type is not a composite type"
                    ),
                    "LIST_SIZE_INVALID_SIZED_FIELD",
                ));
                return Ok(());
            };

            let mut non_list_fields: Vec<&str> = Vec::new();
            let mut non_existing_fields: Vec<&str> = Vec::new();

            for sized_field in sized_fields {
                let target_field = target_type.fields.iter().find(|f| &f.name.value == sized_field);

                match target_field {
                    None => non_existing_fields.push(sized_field),
                    Some(target_field) if !is_list_type(&target_field.ty) => non_list_fields.push(sized_field),
                    Some(_) => {}
                }
            }

            if !non_list_fields.is_empty() || !non_existing_fields.is_empty() {
                for field_name in non_list_fields {
                    context.report_error(GraphQLError::with_code(
                        format!(
                            "{coordinate} references \"{field_name}\" field in @listSize(sizedFields:) argument that is not a list."
                        ),
                        "LIST_SIZE_APPLIED_TO_NON_LIST",
                    ));
                }

                for field_name in non_existing_fields {
                    context.report_error(GraphQLError::with_code(
                        format!(
                            "{coordinate} references \"{field_name}\" field in @listSize(sizedFields:) argument that does not exist."
                        ),
                        "LIST_SIZE_INVALID_SIZED_FIELD",
                    ));
                }

                return Ok(());
            }
        }

        if let Some(slicing_arguments) = slicing_arguments.as_ref().filter(|args| !args.is_empty()) {
            let mut non_int_argument: Vec<&str> = Vec::new();
            let mut non_existing_argument: Vec<&str> = Vec::new();

            for slicing_argument in slicing_arguments {
                let target_argument = field_def.arguments.iter().find(|a| &a.name.value == slicing_argument);

                match target_argument {
                    None => non_existing_argument.push(slicing_argument),
                    Some(target_argument) if !is_int_type_or_nullable_int_type(&target_argument.ty) => {
                        non_int_argument.push(slicing_argument)
                    }
                    Some(_) => {}
                }
            }

            if !non_int_argument.is_empty() || !non_existing_argument.is_empty() {
                for argument_name in non_int_argument {
                    context.report_error(GraphQLError::with_code(
                        format!(
                            "{coordinate} references \"{argument_name}\" argument in @listSize(slicingArguments:) that is not an integer."
                        ),
                        "LIST_SIZE_INVALID_SLICING_ARGUMENT",
                    ));
                }

                for argument_name in non_existing_argument {
                    context.report_error(GraphQLError::with_code(
                        format!(
                            "{coordinate} references \"{argument_name}\" argument in @listSize(slicingArguments:) that does not exist."
                        ),
                        "LIST_SIZE_INVALID_SLICING_ARGUMENT",
                    ));
                }

                return Ok(());
            }
        }

        let list_size = || ListSize {
            print_require_one_slicing_argument: false,
            assumed_size,
            slicing_arguments: slicing_arguments.clone(),
            sized_fields: sized_fields.clone(),
            require_one_slicing_argument,
        };

        if is_object_kind(type_def.kind()) {
            context.state_builder.object_type().field().set_list_size(
                node_name(type_def),
                &field_def.name.value,
                list_size(),
            )?;
        }

        if is_interface_kind(type_def.kind()) {
            context.state_builder.interface_type().field().set_list_size(
                node_name(type_def),
                &field_def.name.value,
                list_size(),
            )?;
        }
    }

    Ok(())
}

fn is_list_type(type_node: &Type) -> bool {
    match type_node {
        Type::List(_) => true,
        Type::Named(_) => false,
        Type::NonNull(n) => is_list_type(&n.ty),
    }
}

fn is_int_type_or_nullable_int_type(type_node: &Type) -> bool {
    match type_node {
        Type::List(_) => false,
        Type::Named(n) => n.name.value == "Int",
        Type::NonNull(n) => is_int_type_or_nullable_int_type(&n.ty),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_type_detection_looks_through_non_null() {
        assert!(is_list_type(&Type::non_null(Type::list(Type::named("A")))));
        assert!(is_list_type(&Type::list(Type::non_null(Type::named("A")))));
        assert!(!is_list_type(&Type::non_null(Type::named("A"))));
    }

    #[test]
    fn int_type_detection() {
        assert!(is_int_type_or_nullable_int_type(&Type::named("Int")));
        assert!(is_int_type_or_nullable_int_type(&Type::non_null(Type::named("Int"))));
        assert!(!is_int_type_or_nullable_int_type(&Type::list(Type::named("Int"))));
        assert!(!is_int_type_or_nullable_int_type(&Type::named("Float")));
    }

    #[test]
    fn string_values_rejects_non_strings() {
        assert_eq!(
            string_values(&[Value::string("a"), Value::string("b")]),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(string_values(&[Value::string("a"), Value::Null(Default::default())]), None);
        assert_eq!(string_values(&[]), Some(vec![]));
    }
}
