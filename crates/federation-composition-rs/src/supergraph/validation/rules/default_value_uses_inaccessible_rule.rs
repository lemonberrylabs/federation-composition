//! Port of `src/supergraph/validation/rules/default-value-uses-inaccessible-rule.ts`.

use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::collections::IndexMap;
use crate::utils::js::JsResult;
use crate::utils::state::strip_type_modifiers;
use graphql_rs::GraphQLError;

/// `DefaultValueUsesInaccessibleRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectTypeField`, `ObjectTypeFieldArg`.
pub struct DefaultValueUsesInaccessibleRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for DefaultValueUsesInaccessibleRule<'s> {
    const NAME: &'static str = "DefaultValueUsesInaccessibleRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(DefaultValueUsesInaccessibleRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for DefaultValueUsesInaccessibleRule<'s> {
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        let Some(default_value) = &field_state.default_value else {
            return Ok(());
        };
        // If the field or type is already inaccessible, we do not need to assert whether the default value would be accessible or not.
        if field_state.inaccessible {
            return Ok(());
        }

        detect_inaccessible_default_value(
            cx,
            &|| format!("{}.{}", input_object_state.name, field_state.name),
            &field_state.r#type,
            default_value,
            &self.supergraph.enum_types,
        );

        Ok(())
    }

    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        let Some(default_value) = &arg_state.default_value else {
            return Ok(());
        };

        if arg_state.inaccessible {
            return Ok(());
        }

        detect_inaccessible_default_value(
            cx,
            &|| format!("{}.{}({}:)", object_state.name, field_state.name, arg_state.name),
            &arg_state.r#type,
            default_value,
            &self.supergraph.enum_types,
        );

        Ok(())
    }
}

/// `detectInaccessibleDefaultValue(context, schemaCoordinate, outputType, defaultValue, enumTypes)`.
/// `schema_coordinate` is the lazily evaluated `() => string`.
fn detect_inaccessible_default_value(
    context: &mut SupergraphValidationContext,
    schema_coordinate: &dyn Fn() -> String,
    output_type: &str,
    default_value: &str,
    enum_types: &IndexMap<String, EnumTypeState>,
) {
    // TODO: another example why we should switch from strings to AST (but maybe enhanced with the string for ease of use)
    // `outputType.replace(/[\[\]\!]+/g, "")`
    let output_type_name = strip_type_modifiers(output_type);
    let Some(enum_type) = enum_types.get(&output_type_name) else {
        // Only enum value can be used as a default value and be marked as inaccessible
        return;
    };

    if enum_type.inaccessible || enum_type.values.get(default_value).is_some_and(|v| v.inaccessible) {
        context.report_error(GraphQLError::with_code(
            format!(
                "Enum value \"{output_type_name}.{default_value}\" is @inaccessible but is used in the default value of \"{}\", which is in the API schema.",
                schema_coordinate()
            ),
            "DEFAULT_VALUE_USES_INACCESSIBLE",
        ));
    }
}
