//! Port of `src/supergraph/validation/rules/no-inaccessible-on-implemented-interface-fields-rule.ts`.

use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use graphql_core::GraphQLError;

/// `NoInaccessibleOnImplementedInterfaceFieldsRule(context, supergraphState)`: fail on
/// `@inaccessible` on object type fields if the corresponding interface field does not have
/// `@inaccessible` applied.
/// Visitor callbacks: `ObjectTypeField`.
pub struct NoInaccessibleOnImplementedInterfaceFieldsRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for NoInaccessibleOnImplementedInterfaceFieldsRule<'s> {
    const NAME: &'static str = "NoInaccessibleOnImplementedInterfaceFieldsRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(NoInaccessibleOnImplementedInterfaceFieldsRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for NoInaccessibleOnImplementedInterfaceFieldsRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        if field_state.inaccessible && !object_type_state.interfaces.is_empty() {
            for interface_name in &object_type_state.interfaces {
                let Some(interface_type) = self.supergraph.interface_types.get(interface_name) else {
                    continue;
                };
                let Some(interface_field) = interface_type.fields.get(&field_state.name) else {
                    continue;
                };

                if !interface_field.inaccessible && !object_type_state.inaccessible {
                    let object_type_field_schema_coordinate =
                        format!("{}.{}", object_type_state.name, field_state.name);
                    let interface_field_schema_coordinate = format!("{interface_name}.{}", field_state.name);
                    cx.report_error(GraphQLError::with_code(
                        format!(
                            "Field \"{object_type_field_schema_coordinate}\" is @inaccessible but implements the interface field \"{interface_field_schema_coordinate}\", which is in the API schema."
                        ),
                        "IMPLEMENTED_BY_INACCESSIBLE",
                    ));
                }
            }
        }

        Ok(())
    }
}
