//! Port of `src/supergraph/validation/rules/required-query-rule.ts`.

use graphql_rs::GraphQLError;
use crate::supergraph::validation::validation_context::SupergraphValidationContext;

/// `RequiredQueryRule(context)`: a pre-supergraph rule (runs before the subgraphs are merged).
pub fn required_query_rule(context: &mut SupergraphValidationContext) {
    if context
        .subgraph_states
        .values()
        .all(|subgraph| subgraph.schema.query_type().is_none())
    {
        context.report_error(GraphQLError::with_code(
            "No queries found in any subgraph: a supergraph must have a query root type.",
            "NO_QUERIES",
        ));
    }
}
