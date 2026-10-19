//! Port of `src/supergraph/validation/rules/no-inaccessible-on-implemented-interface-fields-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `NoInaccessibleOnImplementedInterfaceFieldsRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct NoInaccessibleOnImplementedInterfaceFieldsRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for NoInaccessibleOnImplementedInterfaceFieldsRule<'s> {
    const NAME: &'static str = "NoInaccessibleOnImplementedInterfaceFieldsRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for NoInaccessibleOnImplementedInterfaceFieldsRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}
