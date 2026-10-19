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
    // Object
    for object_type_state in supergraph_state.object_types.values() {
        for visitor in visitors.iter_mut() {
            visitor.object_type(cx, object_type_state)?;
        }

        for field_state in object_type_state.fields.values() {
            for visitor in visitors.iter_mut() {
                visitor.object_type_field(cx, object_type_state, field_state)?;
            }

            for arg_state in field_state.args.values() {
                for visitor in visitors.iter_mut() {
                    visitor.object_type_field_arg(cx, object_type_state, field_state, arg_state)?;
                }
            }
        }
    }

    // Enum
    for enum_type_state in supergraph_state.enum_types.values() {
        for visitor in visitors.iter_mut() {
            visitor.enum_type(cx, enum_type_state)?;
        }
    }

    // Input Object
    for input_object_type_state in supergraph_state.input_object_types.values() {
        for visitor in visitors.iter_mut() {
            visitor.input_object_type(cx, input_object_type_state)?;
        }

        for field_state in input_object_type_state.fields.values() {
            for visitor in visitors.iter_mut() {
                visitor.input_object_type_field(cx, input_object_type_state, field_state)?;
            }
        }
    }

    // Interface
    for interface_type_state in supergraph_state.interface_types.values() {
        for visitor in visitors.iter_mut() {
            visitor.interface_type(cx, interface_type_state)?;
        }

        for field_state in interface_type_state.fields.values() {
            for visitor in visitors.iter_mut() {
                visitor.interface_type_field(cx, interface_type_state, field_state)?;
            }
        }
    }

    // Directive
    for directive_state in supergraph_state.directives.values() {
        for visitor in visitors.iter_mut() {
            visitor.directive(cx, directive_state)?;

            if visitor.has_directive_field_arg() {
                for directive_arg_state in directive_state.args.values() {
                    visitor.directive_field_arg(cx, directive_state, directive_arg_state)?;
                }
            }
        }
    }

    Ok(())
}
