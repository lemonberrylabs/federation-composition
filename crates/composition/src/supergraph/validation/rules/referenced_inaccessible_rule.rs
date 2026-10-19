//! Port of `src/supergraph/validation/rules/referenced-inaccessible-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `ReferencedInaccessibleRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`, `ObjectTypeFieldArg`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct ReferencedInaccessibleRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for ReferencedInaccessibleRule<'s> {
    const NAME: &'static str = "ReferencedInaccessibleRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for ReferencedInaccessibleRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }

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

/// The type state `findOutputType`/`findInputType` returns; the callers read its `inaccessible`.
/// NOTE(port): check the TS bodies for the maps each one searches and in which order.
#[derive(Clone, Copy, Debug)]
struct FoundType {
    inaccessible: bool,
}

fn find_output_type(supergraph: &SupergraphState, type_name: &str) -> Option<FoundType> {
    todo!()
}

fn find_input_type(supergraph: &SupergraphState, type_name: &str) -> Option<FoundType> {
    todo!()
}
