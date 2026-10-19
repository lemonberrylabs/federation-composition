//! Port of `src/subgraph/validation/rules/elements/context.ts`.

use graphql_js::{Directive, GraphQLError, Kind, NodeRef, Value, VisitAction, VisitInfo, Visitor};
use crate::subgraph::context::is_valid_context_name;
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::SpecName;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::JsResult;

use super::{finish, node_name};

/// `ContextDirectiveRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct ContextDirectiveRules {}

impl<'a> SubgraphRule<'a> for ContextDirectiveRules {
    const NAME: &'static str = "ContextDirectiveRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        ContextDirectiveRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ContextDirectiveRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "context", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("context", &node.name.value) {
        return Ok(());
    }

    context.state_builder.mark_spec_as_used(SpecName::Context);

    let Some(type_def) = context.type_node_info.get_type_def() else {
        return Ok(());
    };

    if !matches!(
        type_def.kind(),
        Kind::ObjectTypeDefinition
            | Kind::ObjectTypeExtension
            | Kind::InterfaceTypeDefinition
            | Kind::InterfaceTypeExtension
            | Kind::UnionTypeDefinition
            | Kind::UnionTypeExtension
    ) {
        return Ok(());
    }

    let name = node.arguments.iter().find(|arg| arg.name.value == "name").map(|arg| &arg.value);

    let Some(Value::String(name)) = name else {
        return Ok(());
    };

    if !is_valid_context_name(&name.value) {
        context.report_error(GraphQLError::with_code(
            format!(
                "Context name \"{}\" is invalid. It should have only alphanumeric characters.",
                name.value
            ),
            "CONTEXT_NAME_INVALID",
        ));
    }

    match type_def.kind() {
        Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
            context.state_builder.object_type().add_context(node_name(type_def), &name.value)?;
        }
        Kind::InterfaceTypeDefinition | Kind::InterfaceTypeExtension => {
            context.state_builder.interface_type().add_context(node_name(type_def), &name.value)?;
        }
        Kind::UnionTypeDefinition | Kind::UnionTypeExtension => {
            context.state_builder.union_type().add_context(node_name(type_def), &name.value)?;
        }
        _ => {}
    }

    Ok(())
}
