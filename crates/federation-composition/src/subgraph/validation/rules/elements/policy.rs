//! Port of `src/subgraph/validation/rules/elements/policy.ts`.

use crate::graphql::printer::print;
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::SpecName;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};
use graphql_core::{Directive, GraphQLError, Kind, NodeRef, Value, VisitAction, VisitInfo, Visitor};

use super::{find_annotated_node, finish, is_interface_kind, is_object_kind, node_name};

/// `PolicyRule(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct PolicyRule {}

impl<'a> SubgraphRule<'a> for PolicyRule {
    const NAME: &'static str = "PolicyRule";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        PolicyRule {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for PolicyRule {
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
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "policy", cx),
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
    if !context.is_available_federation_directive("policy", &node.name.value) {
        return Ok(());
    }

    let Some(policies_arg) = node.arguments.iter().find(|arg| arg.name.value == "policies") else {
        return Err(JsError::error("Expected @policy to have a \"policies\" argument"));
    };

    let Value::List(policies_list) = &policies_arg.value else {
        return Err(JsError::error("Expected \"@policy(policies:)\" to be a list"));
    };

    let mut policies: Vec<Vec<String>> = Vec::new();
    for policy_values in &policies_list.values {
        let Value::List(policy_values) = policy_values else {
            return Err(JsError::error(
                "Expected \"@policy(policies:)\" to be in [[policy__Policy!]!]! format",
            ));
        };

        let mut policy_or: Vec<String> = Vec::new();
        for policy in &policy_values.values {
            let Value::String(policy) = policy else {
                return Err(JsError::error(format!(
                    "Expected \"@policy(policies:)\" to be in [[policy__Policy!]!]! format, received [[{}]!]!",
                    print(&policies_arg.value)
                )));
            };

            policy_or.push(policy.value.clone());
        }
        policies.push(policy_or);
    }

    context.state_builder.mark_spec_as_used(SpecName::Policy);

    let parent = find_annotated_node(info, "Could not find the node annotated with @policy")?;

    match parent.kind() {
        Kind::FieldDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @policy",
                ));
            };

            if is_interface_kind(type_def.kind()) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Invalid use of @policy on field \"{}.{}\": @policy cannot be applied on interfaces, interface fields and interface objects",
                        node_name(type_def),
                        node_name(parent)
                    ),
                    "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
                ));
                return Ok(());
            }

            if is_object_kind(type_def.kind()) {
                context.state_builder.object_type().field().set_policies(
                    node_name(type_def),
                    node_name(parent),
                    &policies,
                )?;
            }
        }
        Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
            if context.state_builder.is_interface_object(node_name(parent)) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Invalid use of @policy on interface object \"{}\": @policy cannot be applied on interfaces, interface fields and interface objects",
                        node_name(parent)
                    ),
                    "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
                ));
                return Ok(());
            }
            context
                .state_builder
                .object_type()
                .set_policies(node_name(parent), &policies)?;
        }
        // NOTE(port): the TS lists `Kind.INTERFACE_TYPE_DEFINITION` twice and never
        // `INTERFACE_TYPE_EXTENSION`, so an interface extension matches no case.
        Kind::InterfaceTypeDefinition => {
            context.report_error(GraphQLError::with_code(
                format!(
                    "Invalid use of @policy on interface \"{}\": @policy cannot be applied on interfaces, interface fields and interface objects",
                    node_name(parent)
                ),
                "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
            ));
            return Ok(());
        }
        Kind::ScalarTypeDefinition | Kind::ScalarTypeExtension => {
            context
                .state_builder
                .scalar_type()
                .set_policies(node_name(parent), &policies)?;
        }
        Kind::EnumTypeDefinition | Kind::EnumTypeExtension => {
            context
                .state_builder
                .enum_type()
                .set_policies(node_name(parent), &policies)?;
        }
        _ => {}
    }

    Ok(())
}
