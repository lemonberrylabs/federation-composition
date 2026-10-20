//! Port of `src/subgraph/validation/rules/elements/cost.ts`.

use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::CostSpecDirective;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{parse_int, JsError, JsResult};
use graphql_core::{Directive, GraphQLError, Kind, NodeRef, Value, VisitAction, VisitInfo, Visitor};

use super::{find_annotated_node, finish, is_interface_kind, is_object_kind, node_name};

/// `CostRule(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`. The weight is `parseInt(value, 10)` (`crate::utils::js::parse_int`).
/// Fields: the locals of the TS factory.
pub struct CostRule {}

impl<'a> SubgraphRule<'a> for CostRule {
    const NAME: &'static str = "CostRule";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        CostRule {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for CostRule {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_core::VisitKinds {
        use graphql_core::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut SubgraphValidationContext<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "cost", cx),
            NodeRef::Directive(node) => directive(node, cx, info),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node, _key, _parent, paths, ancestors)` callback.
fn directive<'a>(
    node: &'a Directive,
    context: &mut SubgraphValidationContext<'a>,
    info: &VisitInfo<'a, '_>,
) -> JsResult<()> {
    if !context.is_available_federation_directive("cost", &node.name.value) {
        return Ok(());
    }

    let Some(weight_arg) = node.arguments.iter().find(|arg| arg.name.value == "weight") else {
        return Err(JsError::error("Expected @cost to have a \"weight\" argument"));
    };

    let Value::Int(weight_value) = &weight_arg.value else {
        return Err(JsError::error("Expected \"@cost(weight:)\" to be of type Int"));
    };

    // NOTE(port): the TS wraps `parseInt` in a try/catch that reports
    // DIRECTIVE_COST_INVALID_WEIGHT, but `parseInt` never throws, so that branch is dead.
    let weight = parse_int(&weight_value.value);

    context
        .state_builder
        .mark_cost_spec_as_used(CostSpecDirective::Cost, &node.name.value);

    let parent = find_annotated_node(info, "Could not find the node annotated with @cost")?;

    match parent.kind() {
        Kind::FieldDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @cost",
                ));
            };

            if is_object_kind(type_def.kind()) {
                context
                    .state_builder
                    .object_type()
                    .field()
                    .set_cost(node_name(type_def), node_name(parent), weight)?;
            }

            if is_interface_kind(type_def.kind()) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "@cost cannot be applied to interface \"{}.{}\"",
                        node_name(type_def),
                        node_name(parent)
                    ),
                    "COST_APPLIED_TO_INTERFACE_FIELD",
                ));
                return Ok(());
            }
        }
        Kind::InputValueDefinition => {
            let type_def = context.type_node_info.get_type_def();
            let field_def = context.type_node_info.get_field_def();

            let Some(type_def) = type_def else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @cost",
                ));
            };

            let Some(field_def) = field_def else {
                return Err(JsError::error(
                    "Could not find the parent field of the argument annotated with @cost",
                ));
            };

            if matches!(
                type_def.kind(),
                Kind::InputObjectTypeDefinition | Kind::InputObjectTypeExtension
            ) {
                context.state_builder.input_object_type().field().set_cost(
                    node_name(type_def),
                    node_name(field_def),
                    weight,
                )?;
            }

            if is_object_kind(type_def.kind()) {
                let Some(arg_def) = context.type_node_info.get_argument_def() else {
                    return Err(JsError::error("Could not find the argument annotated with @cost"));
                };

                context.state_builder.object_type().field().arg().set_cost(
                    node_name(type_def),
                    node_name(field_def),
                    &arg_def.name.value,
                    weight,
                )?;
            }

            if is_interface_kind(type_def.kind()) {
                let Some(arg_def) = context.type_node_info.get_argument_def() else {
                    return Err(JsError::error("Could not find the argument annotated with @cost"));
                };

                context.report_error(GraphQLError::with_code(
                    format!(
                        "@cost cannot be applied to interface \"{}.{}\" argument \"{}\"",
                        node_name(type_def),
                        node_name(field_def),
                        arg_def.name.value
                    ),
                    "COST_APPLIED_TO_INTERFACE_FIELD",
                ));
                return Ok(());
            }
        }
        Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
            context
                .state_builder
                .object_type()
                .set_cost(node_name(parent), weight)?;
        }
        Kind::ScalarTypeDefinition | Kind::ScalarTypeExtension => {
            context
                .state_builder
                .scalar_type()
                .set_cost(node_name(parent), weight)?;
        }
        Kind::EnumTypeDefinition | Kind::EnumTypeExtension => {
            context.state_builder.enum_type().set_cost(node_name(parent), weight)?;
        }
        _ => {}
    }

    Ok(())
}
