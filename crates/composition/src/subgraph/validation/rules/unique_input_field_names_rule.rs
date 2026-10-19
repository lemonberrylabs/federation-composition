//! Port of `src/subgraph/validation/rules/unique-input-field-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueInputFieldNamesRule(context)`.
/// Visitor callbacks: `ObjectValue: { enter, leave }`, `ObjectField`.
/// Fields: the locals of the TS factory.
pub struct UniqueInputFieldNamesRule {}

impl<'a> SimpleRule<'a> for UniqueInputFieldNamesRule {
    const NAME: &'static str = "UniqueInputFieldNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueInputFieldNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }

    fn leave(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
