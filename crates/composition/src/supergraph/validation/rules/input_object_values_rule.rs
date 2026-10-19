//! Port of `src/supergraph/validation/rules/input-object-values-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::input_object_type::InputObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `InputObjectValuesRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectType`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct InputObjectValuesRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InputObjectValuesRule<'s> {
    const NAME: &'static str = "InputObjectValuesRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for InputObjectValuesRule<'s> {
    fn input_object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
    ) -> JsResult<()> {
        todo!()
    }
}
