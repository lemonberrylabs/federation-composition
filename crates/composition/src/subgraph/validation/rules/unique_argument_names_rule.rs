//! Port of `src/subgraph/validation/rules/unique-argument-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::Argument;

/// `UniqueArgumentNamesRule(context)`.
/// Visitor callbacks: `Field: checkArgUniqueness`, `Directive: checkArgUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueArgumentNamesRule {}

impl<'a> SimpleRule<'a> for UniqueArgumentNamesRule {
    const NAME: &'static str = "UniqueArgumentNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl UniqueArgumentNamesRule {
    /// `checkArgUniqueness(parentNode)` (its `arguments`).
    fn check_arg_uniqueness(&mut self, context: &mut SimpleValidationContext<'_>, argument_nodes: &[Argument]) {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueArgumentNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
