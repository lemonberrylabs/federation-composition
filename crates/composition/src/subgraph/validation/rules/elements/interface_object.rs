//! Port of `src/subgraph/validation/rules/elements/interface-object.ts`.

use graphql_js::{Directive, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::JsResult;

use super::finish;

/// `InterfaceObjectRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct InterfaceObjectRules {}

impl<'a> SubgraphRule<'a> for InterfaceObjectRules {
    const NAME: &'static str = "InterfaceObjectRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        InterfaceObjectRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for InterfaceObjectRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "interfaceObject", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("interfaceObject", &node.name.value) {
        return Ok(());
    }

    if context.satisfies_version_range("< v2.3") {
        context.report_error(GraphQLError::with_code(
            "@interfaceObject is not yet supported. See https://github.com/graphql-hive/federation-composition/issues/7",
            "UNSUPPORTED_FEATURE",
        ));
        return Ok(());
    }

    let Some(type_def) = context.type_node_info.get_type_def() else {
        return Ok(());
    };

    let (type_name, directives) = match type_def {
        NodeRef::ObjectTypeDefinition(type_def) => (&type_def.name.value, &type_def.directives),
        NodeRef::ObjectTypeExtension(type_def) => (&type_def.name.value, &type_def.directives),
        // handled by directive location validation
        _ => return Ok(()),
    };

    if !directives.iter().any(|d| d.name.value == "key") {
        context.report_error(GraphQLError::with_code(
            format!(
                "The @interfaceObject directive can only be applied to entity types but type \"{type_name}\" has no @key in this subgraph."
            ),
            "INTERFACE_OBJECT_USAGE_ERROR",
        ));
    }

    Ok(())
}
