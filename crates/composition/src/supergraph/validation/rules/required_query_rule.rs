//! Port of `src/supergraph/validation/rules/required-query-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::validation::validation_context::SupergraphValidationContext;

/// `RequiredQueryRule(context)`: a pre-supergraph rule (runs before the subgraphs are merged).
pub fn required_query_rule(context: &mut SupergraphValidationContext) {
    todo!()
}
