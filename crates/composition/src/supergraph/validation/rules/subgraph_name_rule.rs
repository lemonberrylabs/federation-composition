//! Port of `src/supergraph/validation/rules/subgraph-name-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `SubgraphNameRule(context, supergraph)`. Its body runs in `new` (the TS returns `{}`).
/// Visitor callbacks: none.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct SubgraphNameRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for SubgraphNameRule<'s> {
    const NAME: &'static str = "SubgraphNameRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for SubgraphNameRule<'s> {
}
