//! Port of `src/supergraph/validation/rules/only-inaccessible-children-rule.ts`.

use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::input_object_type::InputObjectTypeState;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::object_type::ObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use graphql_rs::GraphQLError;

/// `OnlyInaccessibleChildrenRule(context, supergraph)`.
/// Visitor callbacks: `EnumType`, `ObjectType`, `InterfaceType`, `InputObjectType`.
/// If a type is not annotated with @inaccessible, but all of its fields are. We should error.
/// If a type is annotated with @inaccessible and all of its fields are as well, we should NOT error.
pub struct OnlyInaccessibleChildrenRule;

impl<'s> SupergraphRule<'s> for OnlyInaccessibleChildrenRule {
    const NAME: &'static str = "OnlyInaccessibleChildrenRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(OnlyInaccessibleChildrenRule)
    }
}

impl<'s> SupergraphVisitor<'s> for OnlyInaccessibleChildrenRule {
    fn enum_type(&mut self, cx: &mut SupergraphValidationContext, enum_state: &'s EnumTypeState) -> JsResult<()> {
        if !enum_state.inaccessible && are_all_inaccessible(enum_state.values.values().map(|v| v.inaccessible)) {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type \"{}\" is in the API schema but all of its values are @inaccessible.",
                    enum_state.name
                ),
                "ONLY_INACCESSIBLE_CHILDREN",
            ));
        }
        Ok(())
    }

    fn object_type(&mut self, cx: &mut SupergraphValidationContext, object_state: &'s ObjectTypeState) -> JsResult<()> {
        if !object_state.inaccessible && are_all_inaccessible(object_state.fields.values().map(|f| f.inaccessible)) {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type \"{}\" is in the API schema but all of its fields are @inaccessible.",
                    object_state.name
                ),
                "ONLY_INACCESSIBLE_CHILDREN",
            ));
        }
        Ok(())
    }

    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        if !interface_state.inaccessible
            && are_all_inaccessible(interface_state.fields.values().map(|f| f.inaccessible))
        {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type \"{}\" is in the API schema but all of its fields are @inaccessible.",
                    interface_state.name
                ),
                "ONLY_INACCESSIBLE_CHILDREN",
            ));
        }
        Ok(())
    }

    fn input_object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
    ) -> JsResult<()> {
        if !input_object_state.inaccessible
            && are_all_inaccessible(input_object_state.fields.values().map(|f| f.inaccessible))
        {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type \"{}\" is in the API schema but all of its fields are @inaccessible.",
                    input_object_state.name
                ),
                "ONLY_INACCESSIBLE_CHILDREN",
            ));
        }
        Ok(())
    }
}

/// `areAllInaccessible(childrenMap)`: the `inaccessible` flags of the children.
/// NOTE(port): `[].every(..)` is `true`, so a type without children counts as all-inaccessible.
fn are_all_inaccessible(children: impl IntoIterator<Item = bool>) -> bool {
    children.into_iter().all(|inaccessible| inaccessible)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn are_all_inaccessible_is_true_for_no_children() {
        assert!(are_all_inaccessible([]));
        assert!(are_all_inaccessible([true, true]));
        assert!(!are_all_inaccessible([true, false]));
    }
}
