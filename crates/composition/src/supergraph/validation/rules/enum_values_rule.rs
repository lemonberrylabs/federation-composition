//! Port of `src/supergraph/validation/rules/enum-values-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `EnumValuesRule(context, supergraph)`.
/// Visitor callbacks: `EnumType`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct EnumValuesRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for EnumValuesRule<'s> {
    const NAME: &'static str = "EnumValuesRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for EnumValuesRule<'s> {
    fn enum_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        enum_state: &'s EnumTypeState,
    ) -> JsResult<()> {
        todo!()
    }
}
