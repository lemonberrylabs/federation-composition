//! Port of `src/supergraph/validation/rules/only-inaccessible-children-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::input_object_type::InputObjectTypeState;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::object_type::ObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `OnlyInaccessibleChildrenRule(context, supergraph)`.
/// Visitor callbacks: `EnumType`, `ObjectType`, `InterfaceType`, `InputObjectType`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct OnlyInaccessibleChildrenRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for OnlyInaccessibleChildrenRule<'s> {
    const NAME: &'static str = "OnlyInaccessibleChildrenRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for OnlyInaccessibleChildrenRule<'s> {
    fn enum_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        enum_state: &'s EnumTypeState,
    ) -> JsResult<()> {
        todo!()
    }

    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        todo!()
    }

    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        todo!()
    }

    fn input_object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `areAllInaccessible(childrenMap)`: the `inaccessible` flags of the children.
fn are_all_inaccessible(children: impl IntoIterator<Item = bool>) -> bool {
    children.into_iter().all(|inaccessible| inaccessible)
}
