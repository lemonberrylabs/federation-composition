//! Port of `src/supergraph/validation/rules/required-argument-missing-in-some-subgraph-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `RequiredArgumentMissingInSomeSubgraph(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct RequiredArgumentMissingInSomeSubgraph<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for RequiredArgumentMissingInSomeSubgraph<'s> {
    const NAME: &'static str = "RequiredArgumentMissingInSomeSubgraph";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for RequiredArgumentMissingInSomeSubgraph<'s> {
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        todo!()
    }
}
