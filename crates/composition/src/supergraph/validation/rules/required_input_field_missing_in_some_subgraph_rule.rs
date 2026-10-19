//! Port of `src/supergraph/validation/rules/required-input-field-missing-in-some-subgraph-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `RequiredInputFieldMissingInSomeSubgraphRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct RequiredInputFieldMissingInSomeSubgraphRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for RequiredInputFieldMissingInSomeSubgraphRule<'s> {
    const NAME: &'static str = "RequiredInputFieldMissingInSomeSubgraphRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for RequiredInputFieldMissingInSomeSubgraphRule<'s> {
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}
