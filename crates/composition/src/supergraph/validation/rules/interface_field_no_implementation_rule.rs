//! Port of `src/supergraph/validation/rules/interface-field-no-implementation-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::ObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use crate::supergraph::composition::interface_type::InterfaceTypeState;

/// `InterfaceFieldNoImplementationRule(context, supergraph)`.
/// Visitor callbacks: `ObjectType`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct InterfaceFieldNoImplementationRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InterfaceFieldNoImplementationRule<'s> {
    const NAME: &'static str = "InterfaceFieldNoImplementationRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceFieldNoImplementationRule<'s> {
    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `getTypeFromSupergraph(state, name)`: only interfaces are looked up by the callers.
/// NOTE(port): check the TS body for the exact maps it searches.
fn get_type_from_supergraph<'s>(state: &'s SupergraphState, name: &str) -> Option<&'s InterfaceTypeState> {
    todo!()
}
