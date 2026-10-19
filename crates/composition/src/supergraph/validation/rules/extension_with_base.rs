//! Port of `src/supergraph/validation/rules/extension-with-base.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::ObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `ExtensionWithBaseRule(context, supergraph)`.
/// Visitor callbacks: `ObjectType`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct ExtensionWithBaseRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for ExtensionWithBaseRule<'s> {
    const NAME: &'static str = "ExtensionWithBaseRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for ExtensionWithBaseRule<'s> {
    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        todo!()
    }
}
