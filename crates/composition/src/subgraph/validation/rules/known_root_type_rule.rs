//! Port of `src/subgraph/validation/rules/known-root-type-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::IndexSet;

use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `KnownRootTypeRule(context)`.
/// Visitor callbacks: `SchemaDefinition`.
/// Fields: the locals of the TS factory.
pub struct KnownRootTypeRule {
    type_names: IndexSet<String>,
}

impl<'a> SubgraphRule<'a> for KnownRootTypeRule {
    const NAME: &'static str = "KnownRootTypeRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        let definitions = &context.get_document().definitions;
        let type_names: IndexSet<String> = definitions
            .iter()
            .filter(|def| is_type_definition_or_extension_node(def.as_node_ref()))
            .filter_map(|def| def.name().map(|name| name.value.clone()))
            .collect();

        KnownRootTypeRule { type_names }
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for KnownRootTypeRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::SchemaDefinition(node) = node {
            for operation_type in &node.operation_types {
                if !self.type_names.contains(&operation_type.ty.name.value) {
                    cx.report_error(GraphQLError::with_code(
                        format!(
                            "Cannot set schema {} root to unknown type {}",
                            operation_type.operation.as_str(),
                            operation_type.ty.name.value
                        ),
                        "INVALID_GRAPHQL",
                    ));
                }
            }
        }
        VisitAction::Continue
    }
}

fn is_type_definition_or_extension_node(node: NodeRef<'_>) -> bool {
    graphql_js::is_type_definition_node(node) || graphql_js::is_type_extension_node(node)
}
