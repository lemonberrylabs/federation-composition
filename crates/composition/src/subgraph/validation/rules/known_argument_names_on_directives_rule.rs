//! Port of `src/subgraph/validation/rules/known-argument-names-on-directives-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `KnownArgumentNamesOnDirectivesRule(context)`.
/// Visitor callbacks: `Directive`.
/// Fields: the locals of the TS factory.
pub struct KnownArgumentNamesOnDirectivesRule {}

impl<'a> SimpleRule<'a> for KnownArgumentNamesOnDirectivesRule {
    const NAME: &'static str = "KnownArgumentNamesOnDirectivesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for KnownArgumentNamesOnDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
