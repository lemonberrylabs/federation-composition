//! Port of `src/supergraph/validation/rules/field-argument-default-mismatch-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `FieldArgumentDefaultMismatchRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct FieldArgumentDefaultMismatchRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for FieldArgumentDefaultMismatchRule<'s> {
    const NAME: &'static str = "FieldArgumentDefaultMismatchRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for FieldArgumentDefaultMismatchRule<'s> {
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
