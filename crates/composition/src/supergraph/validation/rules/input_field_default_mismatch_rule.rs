//! Port of `src/supergraph/validation/rules/input-field-default-mismatch-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `InputFieldDefaultMismatchRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct InputFieldDefaultMismatchRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InputFieldDefaultMismatchRule<'s> {
    const NAME: &'static str = "InputFieldDefaultMismatchRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for InputFieldDefaultMismatchRule<'s> {
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}
