//! Port of `src/subgraph/validation/rules/unique-directive-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::utils::collections::IndexSet;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueDirectiveNamesRule(context)`.
/// Visitor callbacks: `DirectiveDefinition`.
/// Fields: the locals of the TS factory.
pub struct UniqueDirectiveNamesRule;

/// The visitor [`UniqueDirectiveNamesRule`] returns.
pub struct UniqueDirectiveNamesVisitor<'a> {
    known_directive_name_nodes: IndexSet<&'a str>,
}

impl<'a> SimpleRule<'a> for UniqueDirectiveNamesRule {
    const NAME: &'static str = "UniqueDirectiveNamesRule";
    type Visitor = UniqueDirectiveNamesVisitor<'a>;

    fn new(_context: &mut SimpleValidationContext<'a>) -> UniqueDirectiveNamesVisitor<'a> {
        UniqueDirectiveNamesVisitor {
            known_directive_name_nodes: IndexSet::default(),
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueDirectiveNamesVisitor<'a> {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_js::VisitKinds {
        use graphql_js::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::DirectiveDefinition(node) = node {
            let directive_name = &node.name.value;
            let existing_name_node = self.known_directive_name_nodes.contains(directive_name.as_str());

            if existing_name_node {
                cx.report_error(GraphQLError::with_code(
                    format!("There can be only one directive named \"@{directive_name}\"."),
                    "INVALID_GRAPHQL",
                ));
            } else {
                self.known_directive_name_nodes.insert(directive_name);
            }
        }
        VisitAction::Continue
    }
}
