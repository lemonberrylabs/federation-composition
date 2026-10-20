//! Port of `src/subgraph/validation/rules/reserved-subgraph-name-rule.ts`.

use graphql_rs::{GraphQLError, VisitKinds, Visitor};

use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `ReservedSubgraphNameRule(context)`.
/// Visitor callbacks: none (the check runs in `new`).
/// Fields: the locals of the TS factory.
pub struct ReservedSubgraphNameRule {}

impl<'a> SubgraphRule<'a> for ReservedSubgraphNameRule {
    const NAME: &'static str = "ReservedSubgraphNameRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        if context.get_subgraph_name() == "_" {
            context.report_error(GraphQLError::with_code(
                "Invalid name _ for a subgraph: this name is reserved",
                "INVALID_SUBGRAPH_NAME",
            ));
        }

        ReservedSubgraphNameRule {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ReservedSubgraphNameRule {
    /// PERF: no callbacks, so `visit_in_parallel` never calls this visitor.
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[])
    }
}
