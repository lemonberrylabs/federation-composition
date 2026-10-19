//! Port of `src/supergraph/validation/rules/interface-key-missing-implementation-type.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `InterfaceKeyMissingImplementationTypeRule(context, supergraph)`.
/// Visitor callbacks: `InterfaceType`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct InterfaceKeyMissingImplementationTypeRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InterfaceKeyMissingImplementationTypeRule<'s> {
    const NAME: &'static str = "InterfaceKeyMissingImplementationTypeRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceKeyMissingImplementationTypeRule<'s> {
    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `joinWithAnd(list)`. NOTE(port): the TS `list.pop()` mutates the caller's array.
fn join_with_and(list: &mut Vec<String>) -> String {
    todo!()
}
