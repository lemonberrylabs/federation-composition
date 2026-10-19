//! Port of `src/supergraph/validation/rules/default-value-uses-inaccessible-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use indexmap::IndexMap;
use crate::supergraph::composition::enum_type::EnumTypeState;

/// `DefaultValueUsesInaccessibleRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectTypeField`, `ObjectTypeFieldArg`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct DefaultValueUsesInaccessibleRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for DefaultValueUsesInaccessibleRule<'s> {
    const NAME: &'static str = "DefaultValueUsesInaccessibleRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for DefaultValueUsesInaccessibleRule<'s> {
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }

    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        todo!()
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
    todo!()
}
