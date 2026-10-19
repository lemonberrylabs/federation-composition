//! Port of `src/supergraph/validation/rules/list-size-slicing-arguments-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use crate::subgraph::state::ListSize;
use indexmap::IndexMap;

/// `ListSizeSlicingArgumentsRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`, `InterfaceTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct ListSizeSlicingArgumentsRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for ListSizeSlicingArgumentsRule<'s> {
    const NAME: &'static str = "ListSizeSlicingArgumentsRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for ListSizeSlicingArgumentsRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }

    fn interface_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
        field_state: &'s InterfaceTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `ensureSlicingArgumentsExist(typeName, fieldState)` for an object or interface field (its
/// `name`, `listSize` and argument names). Throws for a missing `listSize`.
fn ensure_slicing_arguments_exist(
    type_name: &str,
    field_name: &str,
    list_size: Option<&ListSize>,
    arg_names: &[&str],
) -> JsResult<Option<GraphQLError>> {
    todo!()
}
