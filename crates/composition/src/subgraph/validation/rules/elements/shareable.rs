//! Port of `src/subgraph/validation/rules/elements/shareable.ts`.

use graphql_js::{Directive, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::JsResult;

use super::{finish, is_interface_kind, node_name};

/// `ShareableRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct ShareableRules {}

impl<'a> SubgraphRule<'a> for ShareableRules {
    const NAME: &'static str = "ShareableRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        ShareableRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ShareableRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "shareable", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("shareable", &node.name.value) {
        return Ok(());
    }

    let type_def = context.type_node_info.get_type_def();
    let field_def = context.type_node_info.get_field_def();

    let Some(type_def) = type_def else {
        return Ok(());
    };

    let type_name = node_name(type_def);

    let object_fields = match type_def {
        NodeRef::ObjectTypeDefinition(type_def) => Some(&type_def.fields),
        NodeRef::ObjectTypeExtension(type_def) => Some(&type_def.fields),
        _ => None,
    };

    if let Some(object_fields) = object_fields {
        if !context.state_builder.is_interface_object(type_name) {
            if let Some(field_def) = field_def {
                context
                    .state_builder
                    .object_type()
                    .field()
                    .set_shareable(type_name, node_name(field_def))?;
            } else {
                context.state_builder.object_type().set_shareable(type_name)?;
                // mark all fields as shareable
                for field_def in object_fields {
                    context
                        .state_builder
                        .object_type()
                        .field()
                        .set_shareable(type_name, &field_def.name.value)?;
                }
            }
        }
    }

    let Some(field_def) = field_def else {
        return Ok(()); // Let regular validation handle this, it should be applied on a field
    };

    if is_interface_kind(type_def.kind()) {
        context.report_error(GraphQLError::with_code(
            format!(
                "Invalid use of @shareable on field \"{type_name}.{}\": only object type fields can be marked with @shareable",
                node_name(field_def)
            ),
            "INVALID_SHAREABLE_USAGE",
        ));
        return Ok(());
    }

    Ok(())
}
