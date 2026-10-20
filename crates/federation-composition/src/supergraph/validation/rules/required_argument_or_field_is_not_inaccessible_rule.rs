//! Port of `src/supergraph/validation/rules/required-argument-or-field-is-not-inaccessible-rule.ts`.

use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use graphql_core::GraphQLError;

/// `RequiredArgumentOrFieldIsNotInaccessibleRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectTypeField`, `ObjectTypeFieldArg`.
pub struct RequiredArgumentOrFieldIsNotInaccessibleRule;

impl<'s> SupergraphRule<'s> for RequiredArgumentOrFieldIsNotInaccessibleRule {
    const NAME: &'static str = "RequiredArgumentOrFieldIsNotInaccessibleRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(RequiredArgumentOrFieldIsNotInaccessibleRule)
    }
}

impl<'s> SupergraphVisitor<'s> for RequiredArgumentOrFieldIsNotInaccessibleRule {
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        if !input_object_state.inaccessible
            && field_state.inaccessible
            && field_state.r#type.ends_with('!')
            && field_state.default_value.is_none()
        {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Input field \"{}.{}\" is @inaccessible but is a required input field of its type.",
                    input_object_state.name, field_state.name
                ),
                "REQUIRED_INACCESSIBLE",
            ));
        }

        Ok(())
    }

    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        if !field_state.inaccessible
            && arg_state.inaccessible
            && arg_state.r#type.ends_with('!')
            && arg_state.default_value.is_none()
        {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Argument \"{}.{}({}:)\" is @inaccessible but is a required argument of its field.",
                    object_state.name, field_state.name, arg_state.name
                ),
                "REQUIRED_INACCESSIBLE",
            ));
        }

        Ok(())
    }
}
