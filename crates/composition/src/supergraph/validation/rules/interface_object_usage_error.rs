//! Port of `src/supergraph/validation/rules/interface-object-usage-error.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `InterfaceObjectUsageErrorRule(context, supergraph)`.
/// Visitor callbacks: `InterfaceType`.
pub struct InterfaceObjectUsageErrorRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InterfaceObjectUsageErrorRule<'s> {
    const NAME: &'static str = "InterfaceObjectUsageErrorRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(InterfaceObjectUsageErrorRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceObjectUsageErrorRule<'s> {
    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        if !interface_state.has_interface_object {
            return Ok(());
        }

        for interface_state_in_graph in interface_state.by_graph.values() {
            if !interface_state_in_graph.is_interface_object {
                return Ok(());
            }
        }

        cx.report_error(GraphQLError::with_code(
            format!(
                "Type \"{}\" is declared with @interfaceObject in all the subgraphs in which is is defined",
                interface_state.name
            ),
            "INTERFACE_OBJECT_USAGE_ERROR",
        ));

        Ok(())
    }
}
