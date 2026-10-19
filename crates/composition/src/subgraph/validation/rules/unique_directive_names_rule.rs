//! Port of `src/subgraph/validation/rules/unique-directive-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueDirectiveNamesRule(context)`.
/// Visitor callbacks: `DirectiveDefinition`.
/// Fields: the locals of the TS factory.
pub struct UniqueDirectiveNamesRule {}

impl<'a> SimpleRule<'a> for UniqueDirectiveNamesRule {
    const NAME: &'static str = "UniqueDirectiveNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueDirectiveNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
