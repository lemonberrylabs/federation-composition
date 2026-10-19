//! Port of `src/supergraph/validation/rules/field-arguments-of-the-same-type-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::directive::{DirectiveArgState, DirectiveState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `FieldArgumentsOfTheSameTypeRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`, `DirectiveFieldArg`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct FieldArgumentsOfTheSameTypeRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for FieldArgumentsOfTheSameTypeRule<'s> {
    const NAME: &'static str = "FieldArgumentsOfTheSameTypeRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for FieldArgumentsOfTheSameTypeRule<'s> {
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        todo!()
    }

    fn directive_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        directive_state: &'s DirectiveState,
        directive_arg_state: &'s DirectiveArgState,
    ) -> JsResult<()> {
        todo!()
    }

    fn has_directive_field_arg(&self) -> bool {
        true
    }
}

fn join_items(items: &[String]) -> String {
    todo!()
}
