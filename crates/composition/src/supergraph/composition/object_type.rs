//! Port of `src/supergraph/composition/object-type.ts`.

use graphql_js::{Definition, SelectionSet};
use indexmap::{IndexMap, IndexSet};

use super::common::{ComposeHelpers, Graph, Key, MapByGraph, TypeBuilder};
use super::interface_type::InterfaceTypeState;
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{
    Ast, ArgumentKind, Deprecated, Description, ExtensionType, FromContext, ListSize, ObjectType,
};
use crate::supergraph::state::{SubgraphStates, SupergraphState};
use crate::utils::js::JsResult;

/// `RequiresUsageStateInGraph`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequiresUsageStateInGraph {
    pub is_actively_required: bool,
    pub is_only_overridden_required: bool,
}

/// `RequiresFieldUsageIndex`: field coordinate to graph id to usage.
pub type RequiresFieldUsageIndex = IndexMap<String, IndexMap<String, RequiresUsageStateInGraph>>;

/// `isFederationExtension(meta, version)`.
pub fn is_federation_extension(meta: &ObjectTypeStateInGraph, version: FederationVersion) -> bool {
    let has_extends_directive = meta.extension_type == Some(ExtensionType::Extends);

    if meta.extension {
        if version == FederationVersion::V1_0 && !has_extends_directive {
            return false;
        }

        if has_extends_directive {
            return true;
        }

        if meta.has_definition {
            return false;
        }

        return true;
    }

    false
}

/// `objectTypeBuilder()`. Holds the closure's `requiresUsageIndex`, built lazily from the
/// whole supergraph state on the first `composeSupergraphNode` call.
/// NOTE(port): the index is built when the first object type is composed, after
/// `SupergraphStateBuilder::build` has transformed (`createFieldsTransformer`) only the object
/// types visited so far; keep that interleaving.
#[derive(Clone, Debug, Default)]
pub struct ObjectTypeBuilder {
    requires_usage_index: Option<RequiresFieldUsageIndex>,
}

/// `objectTypeBuilder()`.
pub fn object_type_builder() -> ObjectTypeBuilder {
    ObjectTypeBuilder::default()
}

impl ObjectTypeBuilder {
    /// `getRequiresUsageIndexLazy(supergraphState, graphNameToId)`.
    fn get_requires_usage_index_lazy(&mut self, helpers: ComposeHelpers<'_>) -> JsResult<&RequiresFieldUsageIndex> {
        todo!()
    }

    /// `composeSupergraphState(objectType, _graphs, { supergraphState })`: merges auth
    /// directives from interface objects. Only `supergraphState.interfaceTypes` is read, so
    /// that is what it receives. Throws `Interface "X" not found in Supergraph state`.
    pub fn compose_supergraph_state(
        &mut self,
        object_type: &mut ObjectTypeState,
        graphs: &SubgraphStates,
        interface_types: &IndexMap<String, InterfaceTypeState>,
    ) -> JsResult<()> {
        todo!()
    }
}

impl TypeBuilder<ObjectType, ObjectTypeState> for ObjectTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, ObjectTypeState>,
        type_name: &str,
        ty: &ObjectType,
    ) {
        todo!()
    }

    fn compose_supergraph_node(
        &mut self,
        object_type: &ObjectTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        todo!()
    }
}

/// `ObjectTypeState | InterfaceTypeState` (`markRequiresUsageFromSelectionSet`'s `currentType`).
#[derive(Clone, Copy, Debug)]
pub enum ObjectOrInterfaceTypeState<'s> {
    Object(&'s ObjectTypeState),
    Interface(&'s InterfaceTypeState),
}

fn is_used_overridden_in_graph(
    field: &ObjectTypeFieldState,
    field_state_in_graph: &FieldStateInGraph,
    interface_field_graphs_by_field_name: &IndexMap<String, IndexSet<String>>,
    graph_id: &str,
    helpers: ComposeHelpers<'_>,
) -> bool {
    todo!()
}

fn to_field_coordinate(type_name: &str, field_name: &str) -> String {
    format!("{type_name}.{field_name}")
}

fn get_requires_usage_state<'i>(
    requires_usage_index: &'i RequiresFieldUsageIndex,
    coordinate: &str,
    graph_id: &str,
) -> Option<&'i RequiresUsageStateInGraph> {
    requires_usage_index.get(coordinate).and_then(|m| m.get(graph_id))
}

fn is_field_coordinate_effectively_required(
    requires_usage_index: &RequiresFieldUsageIndex,
    coordinate: &str,
    graph_id: &str,
    fallback_required: bool,
) -> bool {
    let usage_in_graph = get_requires_usage_state(requires_usage_index, coordinate, graph_id);
    if usage_in_graph.is_some_and(|u| u.is_actively_required) {
        return true;
    }
    if usage_in_graph.is_some_and(|u| u.is_only_overridden_required) {
        return false;
    }
    fallback_required
}

fn has_only_overridden_requires_dependency(
    requires_usage_index: &RequiresFieldUsageIndex,
    coordinate: &str,
    graph_id: &str,
) -> bool {
    let usage_in_graph = get_requires_usage_state(requires_usage_index, coordinate, graph_id);
    usage_in_graph.is_some_and(|u| u.is_only_overridden_required && !u.is_actively_required)
}

/// The argument object of `shouldKeepObjectTypeJoinTypeInGraph`.
struct ShouldKeepObjectTypeJoinTypeInGraphInput<'i> {
    graph_id: &'i str,
    object_type: &'i ObjectTypeState,
    requires_usage_index: &'i RequiresFieldUsageIndex,
    interface_field_graphs_by_field_name: &'i IndexMap<String, IndexSet<String>>,
    helpers: ComposeHelpers<'i>,
}

fn should_keep_object_type_join_type_in_graph(input: ShouldKeepObjectTypeJoinTypeInGraphInput<'_>) -> bool {
    todo!()
}

fn collect_overridden_graphs_by_field(field: &ObjectTypeFieldState, helpers: ComposeHelpers<'_>) -> IndexSet<String> {
    todo!()
}

/// `buildRequiresUsageIndex(supergraphState, graphNameToId)`. `parseFields` can throw.
fn build_requires_usage_index(helpers: ComposeHelpers<'_>) -> JsResult<RequiresFieldUsageIndex> {
    todo!()
}

fn mark_requires_usage_from_selection_set(
    supergraph_state: &SupergraphState,
    current_type: ObjectOrInterfaceTypeState<'_>,
    selection_set: &SelectionSet,
    graph_id: &str,
    overridden_source_usage: bool,
    index: &mut RequiresFieldUsageIndex,
) {
    todo!()
}

fn mark_requires_usage_for_coordinate(
    coordinate: &str,
    graph_id: &str,
    overridden_source_usage: bool,
    index: &mut RequiresFieldUsageIndex,
) {
    todo!()
}

/// `ObjectTypeState` (`kind: "object"`).
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub contexts: IndexSet<String>,
    pub cost: Option<f64>,
    pub has_definition: bool,
    pub by_graph: MapByGraph<ObjectTypeStateInGraph>,
    pub interfaces: IndexSet<String>,
    pub fields: IndexMap<String, ObjectTypeFieldState>,
    pub is_entity: bool,
    pub description: Option<Description>,
    pub ast: Ast,
}

/// `ObjectTypeFieldState`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeFieldState {
    pub name: String,
    pub r#type: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub is_leaf: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub list_size: Option<ListSize>,
    pub used_as_key: bool,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub by_graph: MapByGraph<FieldStateInGraph>,
    pub args: IndexMap<String, ObjectTypeFieldArgState>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
    /// `internal.seenNonExternal`.
    pub internal_seen_non_external: bool,
}

/// `ObjectTypeFieldArgState`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeFieldArgState {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub cost: Option<f64>,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub by_graph: MapByGraph<ArgStateInGraph>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
}

/// `ObjectTypeStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeStateInGraph {
    pub has_definition: bool,
    pub extension: bool,
    pub extension_type: Option<ExtensionType>,
    pub external: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub keys: Vec<Key>,
    pub interfaces: IndexSet<String>,
    pub inaccessible: bool,
    pub shareable: bool,
    pub version: FederationVersion,
}

/// `FieldStateInGraph` (object fields).
#[derive(Clone, Debug, PartialEq)]
pub struct FieldStateInGraph {
    pub r#type: String,
    pub external: bool,
    pub description: Option<Description>,
    pub inaccessible: bool,
    pub used: bool,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub provides: Option<String>,
    pub requires: Option<String>,
    pub provided: bool,
    pub required: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub shareable: bool,
    pub used_as_key: bool,
    pub extension: bool,
    pub version: FederationVersion,
}

/// `ArgStateInGraph` (object field arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct ArgStateInGraph {
    pub r#type: String,
    pub kind: ArgumentKind,
    pub inaccessible: bool,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub version: FederationVersion,
}

fn get_or_create_object_type<'s>(state: &'s mut IndexMap<String, ObjectTypeState>, type_name: &str) -> &'s mut ObjectTypeState {
    todo!()
}

fn get_or_create_field<'s>(
    object_type_state: &'s mut ObjectTypeState,
    field_name: &str,
    field_type: &str,
) -> &'s mut ObjectTypeFieldState {
    todo!()
}

fn get_or_create_arg<'s>(
    field_state: &'s mut ObjectTypeFieldState,
    arg_name: &str,
    arg_type: &str,
    arg_kind: ArgumentKind,
) -> &'s mut ObjectTypeFieldArgState {
    todo!()
}
