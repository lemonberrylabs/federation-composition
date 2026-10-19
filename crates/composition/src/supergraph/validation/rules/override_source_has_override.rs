//! Port of `src/supergraph/validation/rules/override-source-has-override.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `OverrideSourceHasOverrideRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct OverrideSourceHasOverrideRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for OverrideSourceHasOverrideRule<'s> {
    const NAME: &'static str = "OverrideSourceHasOverrideRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for OverrideSourceHasOverrideRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `onlyWithOverride(entry)`: entries whose `override` is set.
fn only_with_override<T>(entry: &(String, T), r#override: impl Fn(&T) -> Option<&str>) -> bool {
    r#override(&entry.1).is_some()
}
