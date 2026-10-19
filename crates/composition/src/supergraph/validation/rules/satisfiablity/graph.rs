//! Port of `src/supergraph/validation/rules/satisfiablity/graph.ts`.
//!
//! Arena representation: `nodes[i]` is the single node of the TS `nodesByTypeIndex[i]`
//! (`NodeId(i)`), `edges` holds every edge (`EdgeId`), and the head/tail indexes hold edge ids.
//! A graph is built with `&mut self` (`add_from_roots`, `join_subgraphs`, ...); the walk only
//! needs `&self` (the caches it fills are `RefCell`/`Cell`), so a built graph is shared as
//! `Rc<Graph>`. `addSubgraph(graph)` moves the subgraph graph's nodes and edges into this one,
//! remapping their ids (in TS the same objects move and `node.index` is reassigned).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};

use super::constants::GraphId;
use super::contexts::{ContextMask, Contexts};
use super::edge::{Edge, EdgeId};
use super::moves::Move;
use super::node::{Node, NodeId, NodeTypeState};
use super::selection::{SelectionNode, SelectionResolver};
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::union_type::UnionTypeState;
use crate::supergraph::state::SupergraphState;
use crate::utils::js::JsResult;
use crate::utils::logger::Logger;

/// An entry of the `queue` used while adding `@provides` fields: `{ head, providedFields }`.
#[derive(Clone, Debug)]
pub struct ProvidedFieldsQueueItem {
    pub head: NodeId,
    pub provided_fields: Vec<SelectionNode>,
}

/// A field state passed to `requiredContextsOfField`.
#[derive(Clone, Copy, Debug)]
pub enum FieldStateRef<'s> {
    Object(&'s ObjectTypeFieldState),
    Interface(&'s InterfaceTypeFieldState),
}

/// `Graph`.
#[derive(Debug)]
pub struct Graph<'s> {
    warned_about_incorrect_edge: Cell<bool>,
    /// `nodesByTypeIndex` (one node per index).
    nodes: Vec<Node<'s>>,
    edges: Vec<Edge<'s>>,
    edges_by_head_type_index: Vec<Vec<EdgeId>>,
    edges_by_tail_type_index: Vec<Vec<EdgeId>>,
    type_name_to_node_indexes: IndexMap<String, Vec<usize>>,
    /// Type A -> points directly to types B, C, D.
    type_children: Vec<IndexSet<String>>,
    /// Can type A lead to type B?
    type_children_cache: RefCell<IndexMap<String, bool>>,
    is_subgraph: bool,
    logger: Logger,
    /// `id` (string) and `idSymbol`.
    id: GraphId,
    pub name: String,
    supergraph_state: &'s SupergraphState,
    selection_resolver: Rc<SelectionResolver<'s>>,
    ignore_inaccessible: bool,
    contexts: Option<Rc<Contexts>>,
    field_edges_with_progressive_override: Vec<EdgeId>,
    field_edges_with_override: Vec<EdgeId>,
    super_field_edges_to_apply_override: Vec<EdgeId>,
}

impl<'s> Graph<'s> {
    pub fn new(
        logger: &Logger,
        id: GraphId,
        name: String,
        supergraph_state: &'s SupergraphState,
        selection_resolver: Rc<SelectionResolver<'s>>,
        ignore_inaccessible: bool,
        contexts: Option<Rc<Contexts>>,
    ) -> Self {
        todo!()
    }

    /// The node with the given id.
    pub fn node(&self, id: NodeId) -> &Node<'s> {
        &self.nodes[id.0]
    }

    /// The edge with the given id.
    pub fn edge(&self, id: EdgeId) -> &Edge<'s> {
        &self.edges[id.0]
    }

    pub fn add_unreachable_types(&mut self) -> JsResult<&mut Self> {
        todo!()
    }

    pub fn add_from_roots(&mut self) -> JsResult<&mut Self> {
        todo!()
    }

    /// Adds fields from `@interfaceObject` types.
    pub fn add_interface_object_fields(&mut self) -> JsResult<&mut Self> {
        todo!()
    }

    pub fn add_from_entities(&mut self) -> JsResult<&mut Self> {
        todo!()
    }

    /// `addSubgraph(graph)`: moves the subgraph graph's nodes (`withoutState()`) and edges here.
    pub fn add_subgraph(&mut self, graph: Graph<'s>) {
        todo!()
    }

    fn connect_union_or_interface(&mut self, node_index: usize, same_type_name_node_indexes: &[usize], edges_to_add: &mut Vec<Edge<'s>>) {
        todo!()
    }

    fn connect_entities(&mut self, node_index: usize, same_type_name_node_indexes: &[usize], edges_to_add: &mut Vec<Edge<'s>>) -> JsResult<()> {
        todo!()
    }

    fn add_provided_interface_fields(
        &mut self,
        head: NodeId,
        provided_fields: &[SelectionNode],
        queue: &mut Vec<ProvidedFieldsQueueItem>,
    ) -> JsResult<()> {
        todo!()
    }

    fn add_provided_union_fields(
        &mut self,
        head: NodeId,
        provided_fields: &[SelectionNode],
        queue: &mut Vec<ProvidedFieldsQueueItem>,
    ) -> JsResult<()> {
        todo!()
    }

    fn add_provided_field(
        &mut self,
        head: NodeId,
        provided_field: &SelectionNode,
        queue: &mut Vec<ProvidedFieldsQueueItem>,
    ) -> JsResult<()> {
        todo!()
    }

    pub fn join_subgraphs(&mut self) -> JsResult<&mut Self> {
        todo!()
    }

    pub fn add_overridden_fields(&mut self) -> JsResult<&mut Self> {
        todo!()
    }

    fn duplicate_node(&mut self, original_node: NodeId, r#move: &Move) -> NodeId {
        todo!()
    }

    fn replace_edge_at(&mut self, head_index: usize, tail_index: usize, new_edge: EdgeId, edge_index: usize) {
        todo!()
    }

    /// `print(asLink = false)`: Graphviz dot output (debugging).
    pub fn print(&self, as_link: bool) -> String {
        todo!()
    }

    pub fn graph_name_to_id(&self, graph_name: &str) -> Option<String> {
        todo!()
    }

    /// `nodeOf(typeName, failIfMissing = true)`: throws when missing (if `fail_if_missing`) or
    /// when the type has more than one node.
    pub fn node_of(&self, type_name: &str, fail_if_missing: bool) -> JsResult<Option<NodeId>> {
        todo!()
    }

    /// `nodesOf(typeName, failIfMissing = true)`.
    pub fn nodes_of(&self, type_name: &str, fail_if_missing: bool) -> JsResult<Vec<NodeId>> {
        todo!()
    }

    fn get_same_graph_edges_of_index(&self, head: NodeId, indexes: Option<&Vec<usize>>, kind: &str) -> Vec<EdgeId> {
        todo!()
    }

    pub fn field_edges_of_head(&self, head: NodeId, field_name: &str) -> Vec<EdgeId> {
        todo!()
    }

    pub fn abstract_edges_of_head(&self, head: NodeId) -> Vec<EdgeId> {
        todo!()
    }

    fn filter_edges(&self, edges: &[EdgeId]) -> Vec<EdgeId> {
        todo!()
    }

    pub fn entity_edges_of_head(&self, head: NodeId) -> Vec<EdgeId> {
        todo!()
    }

    pub fn indirect_edges_of_head(&self, head: NodeId) -> Vec<EdgeId> {
        todo!()
    }

    pub fn edges_of_head(&self, head: NodeId) -> Vec<EdgeId> {
        todo!()
    }

    pub fn edges_of_tail(&self, tail: NodeId) -> Vec<EdgeId> {
        todo!()
    }

    pub fn possible_types_of(&self, type_name: &str) -> Vec<String> {
        todo!()
    }

    pub fn can_reach_type_from_type(&self, from_type_name: &str, to_type_name: &str) -> bool {
        todo!()
    }

    fn create_nodes_and_edges_for_type(&mut self, type_name: &str) -> JsResult<NodeId> {
        todo!()
    }

    fn ensure_non_or_single_node(&self, type_name: &str) -> JsResult<Option<NodeId>> {
        todo!()
    }

    fn create_nodes_and_edges_for_object_type(&mut self, type_state: &'s ObjectTypeState) -> JsResult<NodeId> {
        todo!()
    }

    fn create_node_for_scalar_type(&mut self, type_name: &str) -> JsResult<NodeId> {
        todo!()
    }

    fn create_node_for_enum_type(&mut self, type_state: &'s EnumTypeState) -> JsResult<NodeId> {
        todo!()
    }

    fn create_node_for_union_type(&mut self, type_state: &'s UnionTypeState) -> JsResult<NodeId> {
        todo!()
    }

    fn create_node_for_interface_type(&mut self, type_state: &'s InterfaceTypeState) -> JsResult<NodeId> {
        todo!()
    }

    fn create_edge_for_interface_type_field(&mut self, head: NodeId, field: &'s InterfaceTypeFieldState) -> JsResult<()> {
        todo!()
    }

    /// Mask of contexts (prefixed with the subgraph name) that the field's `@fromContext`
    /// arguments read from, in the subgraph owning the head node.
    fn required_contexts_of_field(&self, head: NodeId, field: FieldStateRef<'_>) -> ContextMask {
        todo!()
    }

    fn create_edge_for_object_type_field(&mut self, head: NodeId, field: &'s ObjectTypeFieldState) -> JsResult<()> {
        todo!()
    }

    /// Throws `Node for X already exists in subgraph Y`.
    fn create_type_node(&mut self, type_name: &str, type_state: Option<NodeTypeState<'s>>) -> JsResult<NodeId> {
        todo!()
    }

    fn create_node(
        &mut self,
        type_name: &str,
        type_state: Option<NodeTypeState<'s>>,
        graph_id: &str,
        graph_name: &str,
    ) -> NodeId {
        todo!()
    }

    /// `addNode(node)`: appends a node from another graph, giving it a new index.
    fn add_node(&mut self, node: Node<'s>) -> NodeId {
        todo!()
    }

    /// Soft delete: removing an edge from every index is not worth it.
    fn ignore_edge(&mut self, edge: EdgeId) {
        self.edges[edge.0].set_ignored(true);
    }

    fn add_edge(&mut self, edge: Edge<'s>) -> EdgeId {
        todo!()
    }

    fn get_indexes_of_type(&self, type_name: &str) -> Option<&Vec<usize>> {
        self.type_name_to_node_indexes.get(type_name)
    }

    fn true_or_if_subgraph_then(&self, condition_fn: impl FnOnce() -> bool) -> bool {
        if self.is_subgraph {
            return condition_fn();
        }
        true
    }

    fn is_supergraph(&self) -> bool {
        !self.is_subgraph
    }

    fn is_merged_graph(&self) -> bool {
        self.id == GraphId::MergedGraph
    }
}
