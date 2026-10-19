//! Port of `src/supergraph/validation/rules/interface-object-usage-error.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `InterfaceObjectUsageErrorRule(context, supergraph)`.
/// Visitor callbacks: `InterfaceType`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct InterfaceObjectUsageErrorRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InterfaceObjectUsageErrorRule<'s> {
    const NAME: &'static str = "InterfaceObjectUsageErrorRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceObjectUsageErrorRule<'s> {
    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        todo!()
    }
}
