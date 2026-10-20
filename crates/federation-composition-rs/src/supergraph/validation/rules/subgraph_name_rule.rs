//! Port of `src/supergraph/validation/rules/subgraph-name-rule.ts`.

use graphql_rs::GraphQLError;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `SubgraphNameRule(context, supergraph)`. Its body runs in `new` (the TS returns `{}`).
/// Visitor callbacks: none.
pub struct SubgraphNameRule;

impl<'s> SupergraphRule<'s> for SubgraphNameRule {
    const NAME: &'static str = "SubgraphNameRule";

    fn new(context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        let mut errors = Vec::new();
        for subgraph in context.subgraph_states.values() {
            let id = &subgraph.graph.id;
            if id.starts_with("__") {
                errors.push(GraphQLError::with_code(
                    format!("Name \"{id}\" must not begin with \"__\", which is reserved by GraphQL introspection."),
                    "INVALID_GRAPHQL",
                ));
            }
        }
        for error in errors {
            context.report_error(error);
        }

        Ok(SubgraphNameRule)
    }
}

impl<'s> SupergraphVisitor<'s> for SubgraphNameRule {}
