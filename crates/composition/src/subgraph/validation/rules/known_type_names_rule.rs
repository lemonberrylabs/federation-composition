//! Port of `src/subgraph/validation/rules/known-type-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use graphql_js::Ancestor;
use indexmap::IndexSet;
use std::sync::LazyLock;

/// `KnownTypeNamesRule(context)`.
/// Visitor callbacks: `NamedType(node, _1, parent, _2, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct KnownTypeNamesRule {
    type_names: IndexSet<String>,
}

impl<'a> SubgraphRule<'a> for KnownTypeNamesRule {
    const NAME: &'static str = "KnownTypeNamesRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        let definitions = &context.get_document().definitions;

        let type_names: IndexSet<String> = definitions
            .iter()
            .filter(|def| is_type_definition_or_extension_node(def.as_node_ref()))
            .filter_map(|def| def.name().map(|name| name.value.clone()))
            .collect();

        KnownTypeNamesRule { type_names }
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for KnownTypeNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::NamedType(node) = node else {
            return VisitAction::Continue;
        };

        let type_name = &node.name.value;
        if !self.type_names.contains(type_name) {
            let definition_node = info.ancestors.get(2).copied().or(info.parent);
            let is_sdl = definition_node.is_some_and(is_sdl_node);
            if is_sdl && STANDARD_TYPE_NAMES.contains(type_name) {
                return VisitAction::Continue;
            }

            cx.report_error(GraphQLError::with_code(format!("Unknown type {type_name}"), "INVALID_GRAPHQL"));
        }

        VisitAction::Continue
    }
}

fn is_type_definition_or_extension_node(node: NodeRef<'_>) -> bool {
    graphql_js::is_type_definition_node(node) || graphql_js::is_type_extension_node(node)
}

/// `standardTypeNames`: the specified scalar names.
static STANDARD_TYPE_NAMES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_js::specified_scalar_types()
        .iter()
        .map(|t| t.name.to_string())
        .collect()
});

/// `isSDLNode(value)`: a type system definition or extension node (never a list).
fn is_sdl_node(value: Ancestor<'_>) -> bool {
    match value {
        Ancestor::Node(n) => {
            graphql_js::is_type_system_definition_node(n) || graphql_js::is_type_system_extension_node(n)
        }
        Ancestor::List(_) => false,
    }
}
