//! Port of `src/subgraph/validation/rules/elements/extends.ts`.

use graphql_rs::{Directive, Kind, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::ExtensionType;
use crate::subgraph::validation::validation_context::{MarkAsUsedReason, SubgraphRule, SubgraphValidationContext};
use crate::utils::js::JsResult;

use super::finish;

/// `ExtendsRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct ExtendsRules {}

impl<'a> SubgraphRule<'a> for ExtendsRules {
    const NAME: &'static str = "ExtendsRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        ExtendsRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ExtendsRules {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "extends", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("extends", &node.name.value) {
        return Ok(());
    }

    let type_def = context.type_node_info.get_type_def();

    let (kind, type_name, fields) = match type_def {
        Some(NodeRef::ObjectTypeDefinition(type_def)) => {
            context
                .state_builder
                .object_type()
                .set_extension(&type_def.name.value, ExtensionType::Extends)?;
            (Kind::ObjectTypeDefinition, &type_def.name.value, &type_def.fields)
        }
        Some(NodeRef::InterfaceTypeDefinition(type_def)) => {
            context.state_builder.interface_type().set_extension(&type_def.name.value)?;
            (Kind::InterfaceTypeDefinition, &type_def.name.value, &type_def.fields)
        }
        // Let regular validation handle this, it should be applied on a field
        _ => return Ok(()),
    };

    for field in fields {
        // TODO: T11 make sure it's actually correct to mark extended types as used
        context.mark_as_used(MarkAsUsedReason::Extends, kind, type_name, &field.name.value)?;
    }

    Ok(())
}
