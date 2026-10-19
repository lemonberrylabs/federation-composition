//! Port of `src/subgraph/validation/rules/unique-directive-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::IndexSet;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueDirectiveNamesRule(context)`.
/// Visitor callbacks: `DirectiveDefinition`.
/// Fields: the locals of the TS factory.
pub struct UniqueDirectiveNamesRule {
    known_directive_name_nodes: IndexSet<String>,
}

impl<'a> SimpleRule<'a> for UniqueDirectiveNamesRule {
    const NAME: &'static str = "UniqueDirectiveNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueDirectiveNamesRule {
            known_directive_name_nodes: IndexSet::new(),
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueDirectiveNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::DirectiveDefinition(node) = node {
            let directive_name = &node.name.value;
            let existing_name_node = self.known_directive_name_nodes.contains(directive_name);

            if existing_name_node {
                cx.report_error(GraphQLError::with_code(
                    format!("There can be only one directive named \"@{directive_name}\"."),
                    "INVALID_GRAPHQL",
                ));
            } else {
                self.known_directive_name_nodes.insert(directive_name.clone());
            }
        }
        VisitAction::Continue
    }
}
