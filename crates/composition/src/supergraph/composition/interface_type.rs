//! Port of `src/supergraph/composition/interface-type.ts`.

use graphql_js::Definition;
use indexmap::{IndexMap, IndexSet};

use super::common::{ComposeHelpers, Graph, Key, MapByGraph, TypeBuilder};
use super::object_type::ObjectTypeState;
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{Ast, ArgumentKind, Deprecated, Description, FromContext, InterfaceType, ListSize};
use crate::supergraph::state::SubgraphStates;
use crate::utils::js::JsResult;

/// `interfaceTypeBuilder()`.
#[derive(Clone, Debug, Default)]
pub struct InterfaceTypeBuilder;

/// `interfaceTypeBuilder()`.
pub fn interface_type_builder() -> InterfaceTypeBuilder {
    InterfaceTypeBuilder
}

impl InterfaceTypeBuilder {
    /// `composeSupergraphState(interfaceType, graphs, { supergraphState })`: collects
    /// `@requiresScopes`/`@policy`/`@authenticated` from the implementing object types. Only
    /// `supergraphState.objectTypes` is read, so that is what it receives.
    pub fn compose_supergraph_state(
        &mut self,
        interface_type: &mut InterfaceTypeState,
        graphs: &SubgraphStates,
        object_types: &IndexMap<String, ObjectTypeState>,
    ) -> JsResult<()> {
        todo!()
    }
}

impl TypeBuilder<InterfaceType, InterfaceTypeState> for InterfaceTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, InterfaceTypeState>,
        type_name: &str,
        ty: &InterfaceType,
    ) {
        todo!()
    }

    fn compose_supergraph_node(
        &mut self,
        interface_type: &InterfaceTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        todo!()
    }
}

/// `InterfaceTypeState` (`kind: "interface"`).
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub contexts: IndexSet<String>,
    pub has_definition: bool,
    pub description: Option<Description>,
    pub by_graph: MapByGraph<InterfaceTypeInGraph>,
    pub interfaces: IndexSet<String>,
    pub implemented_by: IndexSet<String>,
    pub fields: IndexMap<String, InterfaceTypeFieldState>,
    pub has_interface_object: bool,
    pub is_entity: bool,
    pub ast: Ast,
}

/// `InterfaceTypeFieldState`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeFieldState {
    pub name: String,
    pub r#type: String,
    pub is_leaf: bool,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub list_size: Option<ListSize>,
    pub deprecated: Option<Deprecated>,
    pub description: Option<Description>,
    pub used_as_key: bool,
    pub by_graph: MapByGraph<FieldStateInGraph>,
    pub args: IndexMap<String, InterfaceTypeFieldArgState>,
    pub ast: Ast,
}

/// `InterfaceTypeFieldArgState`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeFieldArgState {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub tags: IndexSet<String>,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub cost: Option<f64>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub by_graph: MapByGraph<ArgStateInGraph>,
    pub ast: Ast,
}

/// `InterfaceTypeInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeInGraph {
    pub extension: bool,
    pub keys: Vec<Key>,
    pub interfaces: IndexSet<String>,
    pub implemented_by: IndexSet<String>,
    pub is_interface_object: bool,
    pub version: FederationVersion,
}

/// `FieldStateInGraph` (interface fields).
#[derive(Clone, Debug, PartialEq)]
pub struct FieldStateInGraph {
    pub r#type: String,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub provides: Option<String>,
    pub shareable: bool,
    pub used_as_key: bool,
    pub external: bool,
    pub requires: Option<String>,
    pub version: FederationVersion,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
}

/// `ArgStateInGraph` (interface field arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct ArgStateInGraph {
    pub r#type: String,
    pub kind: ArgumentKind,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub version: FederationVersion,
}

fn get_or_create_interface_type<'s>(
    state: &'s mut IndexMap<String, InterfaceTypeState>,
    type_name: &str,
) -> &'s mut InterfaceTypeState {
    todo!()
}

fn get_or_create_interface_field<'s>(
    interface_type_state: &'s mut InterfaceTypeState,
    field_name: &str,
    field_type: &str,
) -> &'s mut InterfaceTypeFieldState {
    todo!()
}

fn get_or_create_arg<'s>(
    field_state: &'s mut InterfaceTypeFieldState,
    arg_name: &str,
    arg_type: &str,
    arg_kind: ArgumentKind,
) -> &'s mut InterfaceTypeFieldArgState {
    todo!()
}
