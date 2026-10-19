//! Port of `src/supergraph/validation/rules/fields-of-the-same-type-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `FieldsOfTheSameTypeRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`, `InputObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct FieldsOfTheSameTypeRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for FieldsOfTheSameTypeRule<'s> {
    const NAME: &'static str = "FieldsOfTheSameTypeRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for FieldsOfTheSameTypeRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }

    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `stripNonNull(type)`: removes one trailing `!`.
fn strip_non_null(ty: &str) -> String {
    ty.strip_suffix('!').unwrap_or(ty).to_string()
}

/// `stripList(type)`: `type.slice(indexOf("[") + 1, lastIndexOf("]"))`.
/// NOTE(port): JS `slice` with `-1` end (no `]`) counts from the end; port exactly.
fn strip_list(ty: &str) -> String {
    todo!()
}

/// `{ superType, localType }`, the argument and result of `normalizeOutputTypeStrings`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OutputTypeStrings {
    super_type: String,
    local_type: String,
}

/// `normalizeOutputTypeStrings({ superType, localType })`: removes the modifiers that make two
/// output types compatible (see the table in the TS).
fn normalize_output_type_strings(input: OutputTypeStrings) -> OutputTypeStrings {
    todo!()
}
