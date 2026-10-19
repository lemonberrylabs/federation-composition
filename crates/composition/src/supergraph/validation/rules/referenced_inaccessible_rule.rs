//! Port of `src/supergraph/validation/rules/referenced-inaccessible-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use super::fields_of_the_same_type_rule::strip_all_modifiers;

/// `ReferencedInaccessibleRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`, `ObjectTypeFieldArg`.
pub struct ReferencedInaccessibleRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for ReferencedInaccessibleRule<'s> {
    const NAME: &'static str = "ReferencedInaccessibleRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(ReferencedInaccessibleRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for ReferencedInaccessibleRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        // PERF: the lookup is pure, so it is skipped when the field is inaccessible anyway, and
        // the type name borrows from the field type (`strip_all_modifiers` gives the same string).
        let is_inaccessible = field_state.inaccessible || object_state.inaccessible;
        if is_inaccessible {
            return Ok(());
        }
        // `fieldState.type.replace(/[\[\]\!]+/g, "")`
        let output_type_name = strip_all_modifiers(&field_state.r#type);
        let references_inaccessible =
            find_output_type(self.supergraph, &output_type_name).is_some_and(|t| t.inaccessible);

        if references_inaccessible && !is_inaccessible {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type \"{output_type_name}\" is @inaccessible but is referenced by \"{}.{}\", which is in the API schema.",
                    object_state.name, field_state.name
                ),
                "REFERENCED_INACCESSIBLE",
            ));
        }

        Ok(())
    }

    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        // PERF: as in `object_type_field`.
        let is_inaccessible = arg_state.inaccessible || field_state.inaccessible || object_state.inaccessible;
        if is_inaccessible {
            return Ok(());
        }
        let output_type_name = strip_all_modifiers(&arg_state.r#type);
        let references_inaccessible =
            find_input_type(self.supergraph, &output_type_name).is_some_and(|t| t.inaccessible);

        if references_inaccessible && !is_inaccessible {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type \"{output_type_name}\" is @inaccessible but is referenced by \"{}.{}({}:)\", which is in the API schema.",
                    object_state.name, field_state.name, arg_state.name
                ),
                "REFERENCED_INACCESSIBLE",
            ));
        }

        Ok(())
    }
}

/// The type state `findOutputType`/`findInputType` returns; the callers read its `inaccessible`.
#[derive(Clone, Copy, Debug)]
struct FoundType {
    inaccessible: bool,
}

/// `enumTypes || objectTypes || interfaceTypes || unionTypes || scalarTypes`.
fn find_output_type(supergraph: &SupergraphState, type_name: &str) -> Option<FoundType> {
    if let Some(t) = supergraph.enum_types.get(type_name) {
        return Some(FoundType { inaccessible: t.inaccessible });
    }
    if let Some(t) = supergraph.object_types.get(type_name) {
        return Some(FoundType { inaccessible: t.inaccessible });
    }
    if let Some(t) = supergraph.interface_types.get(type_name) {
        return Some(FoundType { inaccessible: t.inaccessible });
    }
    if let Some(t) = supergraph.union_types.get(type_name) {
        return Some(FoundType { inaccessible: t.inaccessible });
    }
    if let Some(t) = supergraph.scalar_types.get(type_name) {
        return Some(FoundType { inaccessible: t.inaccessible });
    }
    None
}

/// `enumTypes || inputObjectTypes`.
fn find_input_type(supergraph: &SupergraphState, type_name: &str) -> Option<FoundType> {
    if let Some(t) = supergraph.enum_types.get(type_name) {
        return Some(FoundType { inaccessible: t.inaccessible });
    }
    if let Some(t) = supergraph.input_object_types.get(type_name) {
        return Some(FoundType { inaccessible: t.inaccessible });
    }
    None
}
