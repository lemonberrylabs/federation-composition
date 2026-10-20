//! Port of `src/subgraph/validation/rules/elements/external.ts`.

use crate::graphql::printer::print;
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::JsResult;
use graphql_core::{Directive, FieldDefinition, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};

use super::finish;

// Supergraph context:
// - mark a field or type as external (with the subgraph's name)

/// `ExternalRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct ExternalRules {}

impl<'a> SubgraphRule<'a> for ExternalRules {
    const NAME: &'static str = "ExternalRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        ExternalRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ExternalRules {
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
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "external", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("external", &node.name.value) {
        return Ok(());
    }

    // TODO: T12 check if interface types should be allowed in the @external directive
    let type_def = context.type_node_info.get_type_def();
    let field_def = context.type_node_info.get_field_def();

    let (type_name, type_fields) = match type_def {
        Some(NodeRef::ObjectTypeDefinition(type_def)) => (&type_def.name.value, &type_def.fields),
        Some(NodeRef::ObjectTypeExtension(type_def)) => (&type_def.name.value, &type_def.fields),
        // Let regular validation handle this, it should be applied on a field or an object
        _ => return Ok(()),
    };

    let mut field_definitions: Vec<&'a FieldDefinition> = Vec::new();

    if let Some(field_def) = field_def {
        let NodeRef::FieldDefinition(field_def) = field_def else {
            // Let regular validation handle this, it should be applied on a field or an object
            return Ok(());
        };
        field_definitions.push(field_def);
    } else if !type_fields.is_empty() {
        // In Federation v1, we do not mark all fields as @external
        if context.satisfies_version_range(">= v2.0") {
            field_definitions.extend(type_fields.iter());
        }
    }

    for field in field_definitions {
        context.mark_as_external(&format!("{}.{}", type_name, field.name.value));
        let conflicting_directives: Vec<&Directive> = field
            .directives
            .iter()
            .filter(|directive| {
                context.is_available_federation_directive("tag", &directive.name.value)
                    || context.is_available_federation_directive("inaccessible", &directive.name.value)
            })
            .collect();

        if !conflicting_directives.is_empty() && context.satisfies_version_range(">= v2.0") {
            for directive in conflicting_directives {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Cannot apply merged directive {} to external field \"{}.{}\"",
                        print(directive).trim(),
                        type_name,
                        field.name.value
                    ),
                    "MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL",
                ));
            }
        } else {
            context
                .state_builder
                .object_type()
                .field()
                .set_external(type_name, &field.name.value)?;
        }
    }

    Ok(())
}
