//! Port of `src/supergraph/validation/rules/interface-subtype-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use indexmap::{IndexMap, IndexSet};
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::scalar_type::ScalarTypeState;
use crate::supergraph::composition::union_type::UnionTypeState;

/// `InterfaceSubtypeRule(context, supergraph)`. Fields: `implementations_map` (built in `new`).
/// Visitor callbacks: `ObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct InterfaceSubtypeRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InterfaceSubtypeRule<'s> {
    const NAME: &'static str = "InterfaceSubtypeRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceSubtypeRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}

fn is_type_sub_type_of(
    state: &SupergraphState,
    implementations_map: &IndexMap<String, IndexSet<String>>,
    maybe_sub_type_name: &str,
    super_type_name: &str,
) -> bool {
    todo!()
}

fn get_type_from_supergraph<'s>(state: &'s SupergraphState, name: &str) -> Option<SupergraphType<'s>> {
    todo!()
}

/// `abstract_type` is an interface or union, `maybe_sub_type` an object or interface.
fn is_sub_type(
    implementations_map: &IndexMap<String, IndexSet<String>>,
    abstract_type: SupergraphType<'_>,
    maybe_sub_type: SupergraphType<'_>,
) -> bool {
    todo!()
}

/// `SupergraphType`.
#[derive(Clone, Copy, Debug)]
enum SupergraphType<'s> {
    Object(&'s ObjectTypeState),
    Enum(&'s EnumTypeState),
    Scalar(&'s ScalarTypeState),
    Union(&'s UnionTypeState),
    Interface(&'s InterfaceTypeState),
}

fn is_abstract_type(ty: SupergraphType<'_>) -> bool {
    is_interface_type(ty) || is_union_type(ty)
}

fn is_object_type(ty: SupergraphType<'_>) -> bool {
    matches!(ty, SupergraphType::Object(_))
}

fn is_interface_type(ty: SupergraphType<'_>) -> bool {
    matches!(ty, SupergraphType::Interface(_))
}

fn is_union_type(ty: SupergraphType<'_>) -> bool {
    matches!(ty, SupergraphType::Union(_))
}
