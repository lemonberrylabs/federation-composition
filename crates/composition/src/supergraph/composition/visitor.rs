//! Port of `src/supergraph/composition/visitor.ts`: a visitor over the supergraph state used by
//! the supergraph validation rules, so the state is iterated once for all rules.

use super::directive::{DirectiveArgState, DirectiveState};
use super::enum_type::EnumTypeState;
use super::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use super::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use super::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::SupergraphValidationContext;
use crate::utils::js::JsResult;

/// `SupergraphVisitorMap`. A rule implements the callbacks it has; the others are no-ops.
/// The validation context is passed to every callback (the TS rules capture it). Callbacks
/// return `JsResult` because some rules throw; the error aborts the visit like a JS throw.
/// `'s` is the lifetime of the supergraph state, so rules may keep references into it.
pub trait SupergraphVisitor<'s> {
    // Object
    fn object_type(&mut self, cx: &mut SupergraphValidationContext, object_state: &'s ObjectTypeState) -> JsResult<()> {
        Ok(())
    }
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        Ok(())
    }
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        Ok(())
    }
    // Interface
    fn interface_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
        field_state: &'s InterfaceTypeFieldState,
    ) -> JsResult<()> {
        Ok(())
    }
    // Enum
    fn enum_type(&mut self, cx: &mut SupergraphValidationContext, enum_state: &'s EnumTypeState) -> JsResult<()> {
        Ok(())
    }
    // Input Object
    fn input_object_type(&mut self, cx: &mut SupergraphValidationContext, input_object_state: &'s InputObjectTypeState) -> JsResult<()> {
        Ok(())
    }
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        Ok(())
    }
    // Interface
    fn interface_type(&mut self, cx: &mut SupergraphValidationContext, interface_state: &'s InterfaceTypeState) -> JsResult<()> {
        Ok(())
    }
    // Directive
    fn directive(&mut self, cx: &mut SupergraphValidationContext, directive_state: &'s DirectiveState) -> JsResult<()> {
        Ok(())
    }
    fn directive_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        directive_state: &'s DirectiveState,
        directive_arg_state: &'s DirectiveArgState,
    ) -> JsResult<()> {
        Ok(())
    }

    /// Whether the TS visitor object has a `DirectiveFieldArg` property: `visitSupergraphState`
    /// only iterates a directive's arguments for visitors that have it.
    fn has_directive_field_arg(&self) -> bool {
        false
    }
}

/// `visitSupergraphState(supergraphState, visitors)`.
pub fn visit_supergraph_state<'s>(
    supergraph_state: &'s SupergraphState,
    cx: &mut SupergraphValidationContext,
    visitors: &mut [Box<dyn SupergraphVisitor<'s> + 's>],
) -> JsResult<()> {
    todo!()
}
