//! Port of `src/subgraph/validation/rules/unique-directives-per-location-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueDirectivesPerLocationRule(context)`.
/// Visitor callbacks: generic `enter(node)`.
/// Fields: the locals of the TS factory.
pub struct UniqueDirectivesPerLocationRule {}

impl<'a> SimpleRule<'a> for UniqueDirectivesPerLocationRule {
    const NAME: &'static str = "UniqueDirectivesPerLocationRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueDirectivesPerLocationRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
