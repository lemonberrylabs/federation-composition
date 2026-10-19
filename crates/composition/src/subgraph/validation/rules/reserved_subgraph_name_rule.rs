//! Port of `src/subgraph/validation/rules/reserved-subgraph-name-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `ReservedSubgraphNameRule(context)`.
/// Visitor callbacks: none (the check runs in `new`).
/// Fields: the locals of the TS factory.
pub struct ReservedSubgraphNameRule {}

impl<'a> SubgraphRule<'a> for ReservedSubgraphNameRule {
    const NAME: &'static str = "ReservedSubgraphNameRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ReservedSubgraphNameRule {
}
