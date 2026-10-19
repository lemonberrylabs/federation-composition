//! Port of `src/supergraph/validation/rules/auth-on-requires-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use graphql_js::{Field, InlineFragment, SelectionSet};
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::union_type::UnionTypeState;

/// `AuthOnRequiresRule(context, supergraph)`. Validates that fields with `@requires` have the auth directives needed by the fields their selection set reaches.
/// Visitor callbacks: `ObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct AuthOnRequiresRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for AuthOnRequiresRule<'s> {
    const NAME: &'static str = "AuthOnRequiresRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for AuthOnRequiresRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `FieldCoordinate | TypeCoordinate` (`Type.field` or `Type`).
pub type Coordinate = String;

/// `ObjectTypeState | InterfaceTypeState | UnionTypeState`.
#[derive(Clone, Copy, Debug)]
pub enum RequiresCurrentType<'s> {
    Object(&'s ObjectTypeState),
    Interface(&'s InterfaceTypeState),
    Union(&'s UnionTypeState),
}

/// `ensureAccessToSelectionSet(supergraph, currentType, selectionSet, provisionedAccess)`:
/// the first coordinate the provisioned access cannot reach. Throws on unsupported selections.
pub fn ensure_access_to_selection_set(
    supergraph: &SupergraphState,
    current_type: RequiresCurrentType<'_>,
    selection_set: &SelectionSet,
    provisioned_access: &mut ProvisionedAccess,
) -> JsResult<Option<Coordinate>> {
    todo!()
}

/// `current_type` is an object or interface.
fn ensure_access_to_field(
    supergraph: &SupergraphState,
    current_type: RequiresCurrentType<'_>,
    field_node: &Field,
    provisioned_access: &mut ProvisionedAccess,
) -> JsResult<Option<Coordinate>> {
    todo!()
}

fn ensure_access_to_inline_fragment(
    supergraph: &SupergraphState,
    current_type: RequiresCurrentType<'_>,
    inline_fragment: &InlineFragment,
    provisioned_access: &mut ProvisionedAccess,
) -> JsResult<Option<Coordinate>> {
    todo!()
}

fn create_access_requirement_error(graph_name: &str, field_with_requires_coordinate: &str, auth_coordinate: &str) -> GraphQLError {
    todo!()
}

/// The `required` argument of `canAccess`.
#[derive(Clone, Copy, Debug)]
pub struct RequiredAccess<'r> {
    pub authenticated: bool,
    pub policies: &'r [Vec<String>],
    pub scopes: &'r [Vec<String>],
}

/// `ProvisionedAccess`: the auth a field with `@requires` (or `@fromContext`) is guaranteed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProvisionedAccess {
    pub scopes: Vec<Vec<String>>,
    pub policies: Vec<Vec<String>>,
    pub authenticated: bool,
}

impl ProvisionedAccess {
    pub fn new(object_type_state: &ObjectTypeState, field_state: &ObjectTypeFieldState) -> Self {
        todo!()
    }

    pub fn can_access(&self, required: RequiredAccess<'_>) -> bool {
        todo!()
    }
}
