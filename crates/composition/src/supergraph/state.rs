//! Port of `src/supergraph/state.ts`.
//!
//! `createSupergraphStateBuilder()` returns closures over `state`, the type builders,
//! `subgraphStates` and `graphNameToIdMap`; here they are the fields of
//! [`SupergraphStateBuilder`] and the closures are its methods.
//! Subgraph states are immutable once validated and are shared by the validation context, the
//! supergraph state and the builder, so they are `Rc<SubgraphState>`.

use std::rc::Rc;

use graphql_js::{Definition, SelectionSet};
use indexmap::{IndexMap, IndexSet};

use super::composition::directive::{DirectiveBuilder, DirectiveState};
use super::composition::enum_type::{EnumTypeBuilder, EnumTypeState};
use super::composition::input_object_type::{InputObjectTypeBuilder, InputObjectTypeState};
use super::composition::interface_type::{InterfaceTypeBuilder, InterfaceTypeState};
use super::composition::object_type::{ObjectTypeBuilder, ObjectTypeState};
use super::composition::scalar_type::{ScalarTypeBuilder, ScalarTypeState};
use super::composition::union_type::{UnionTypeBuilder, UnionTypeState};
use crate::specifications::link::Link;
use crate::subgraph::state::{CostSpec, SubgraphState};
use crate::utils::js::JsResult;

/// Subgraph states by graph id (`Map<string, SubgraphState>`).
pub type SubgraphStates = IndexMap<String, Rc<SubgraphState>>;

/// `Link & { graph: string }`. `linkWithGraph` mutates the subgraph's link object in TS; the
/// port copies it (nothing reads `graph` through the subgraph state).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkWithGraph {
    pub link: Link,
    pub graph: String,
}

/// `SupergraphState["specs"]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SupergraphSpecs {
    pub tag: bool,
    pub context: bool,
    pub cost: CostSpec,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub requires_scopes: bool,
    pub policy: bool,
}

/// `SupergraphState`.
#[derive(Clone, Debug, Default)]
pub struct SupergraphState {
    pub subgraphs: SubgraphStates,
    pub scalar_types: IndexMap<String, ScalarTypeState>,
    pub object_types: IndexMap<String, ObjectTypeState>,
    pub interface_types: IndexMap<String, InterfaceTypeState>,
    pub union_types: IndexMap<String, UnionTypeState>,
    pub enum_types: IndexMap<String, EnumTypeState>,
    pub input_object_types: IndexMap<String, InputObjectTypeState>,
    pub directives: IndexMap<String, DirectiveState>,
    pub links: Vec<LinkWithGraph>,
    pub specs: SupergraphSpecs,
}

/// `SupergraphStateBuilder = ReturnType<typeof createSupergraphStateBuilder>`.
#[derive(Debug)]
pub struct SupergraphStateBuilder {
    state: SupergraphState,
    directive: DirectiveBuilder,
    scalar_type: ScalarTypeBuilder,
    enum_type: EnumTypeBuilder,
    input_object_type: InputObjectTypeBuilder,
    interface_type: InterfaceTypeBuilder,
    object_type: ObjectTypeBuilder,
    union_type: UnionTypeBuilder,
    subgraph_states: SubgraphStates,
    /// `graphNameToIdMap` (a plain object in TS; only looked up).
    graph_name_to_id_map: IndexMap<String, String>,
}

/// `createSupergraphStateBuilder()`.
pub fn create_supergraph_state_builder() -> SupergraphStateBuilder {
    todo!()
}

impl SupergraphStateBuilder {
    /// `addSubgraph(subgraph)`. Throws `Graph with ID "X" already exists`.
    pub fn add_subgraph(&mut self, subgraph: Rc<SubgraphState>) -> JsResult<()> {
        todo!()
    }

    pub fn get_graph(&self, id: &str) -> Option<&Rc<SubgraphState>> {
        self.state.subgraphs.get(id)
    }

    /// `composeSupergraphState()`: runs the builders' `composeSupergraphState` over every
    /// type state (objects, then interfaces; the other builders have none).
    pub fn compose_supergraph_state(&mut self) -> JsResult<()> {
        todo!()
    }

    pub fn visit_subgraph_state(&mut self, subgraph_state: Rc<SubgraphState>) {
        todo!()
    }

    pub fn get_supergraph_state(&self) -> &SupergraphState {
        &self.state
    }

    pub fn get_subgraph_state(&self, graph_id: &str) -> Option<&Rc<SubgraphState>> {
        self.subgraph_states.get(graph_id)
    }

    /// `links()`: `mergeLinks(state.links)`.
    pub fn links(&self) -> Vec<Link> {
        crate::specifications::link::merge_links(self.state.links.iter().map(|l| &l.link))
    }

    /// `build()`: the supergraph type and directive definitions.
    /// NOTE(port): the TS transforms each object type (`transformFields(objectTypeState)`, which
    /// mutates its fields' `requires`/`provides`) right before composing it, and the object
    /// builder lazily indexes `@requires` usage across all object types on the first compose;
    /// keep that order. Also strips executable directives not identical in every subgraph
    /// (removes from `state.directives` while iterating it; use `shift_remove` after the loop
    /// or iterate over a snapshot of names, preserving the TS outcome).
    pub fn build(&mut self) -> JsResult<Vec<Definition>> {
        todo!()
    }
}

/// `linkWithGraph(link, graph)`.
fn link_with_graph(link: &Link, graph: &str) -> LinkWithGraph {
    LinkWithGraph {
        link: link.clone(),
        graph: graph.to_string(),
    }
}

fn visit_selection_set_node(selection_set: &SelectionSet, locations: &mut IndexSet<String>, current_path: &[String]) {
    todo!()
}

fn create_fields_set(selection_set: &SelectionSet) -> IndexSet<String> {
    let mut locations = IndexSet::new();
    visit_selection_set_node(selection_set, &mut locations, &[]);
    locations
}

/// `filterSelectionSetNode(selectionSet, removableFields, currentPath)`: removes removable
/// leaf fields in place.
fn filter_selection_set_node(selection_set: &mut SelectionSet, removable_fields: &IndexSet<String>, current_path: &str) {
    todo!()
}

/// `createFieldsTransformer(state)` returns `visitTypeState`; its inner functions
/// `transformFields` and `mergeKeyFieldsCollection` are the free functions below.
/// The returned function mutates the object type state at `object_type_name` in `state`.
/// NOTE(port): the TS throws `Referenced type not found: X` for a `@provides` on a field
/// whose output type is unknown.
fn visit_type_state(state: &mut SupergraphState, object_type_name: &str) -> JsResult<()> {
    todo!()
}

/// `transformFields(removableFields, fields)`. `parseFields` can throw.
fn transform_fields(removable_fields: &IndexSet<String>, fields: &str) -> JsResult<String> {
    todo!()
}

/// `mergeKeyFieldsCollection(fieldsCollection)`: the fields common to every key.
fn merge_key_fields_collection(fields_collection: &[String]) -> JsResult<IndexSet<String>> {
    todo!()
}
