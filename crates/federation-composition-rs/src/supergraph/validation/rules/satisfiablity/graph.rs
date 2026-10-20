//! Port of `src/supergraph/validation/rules/satisfiablity/graph.ts`.
//!
//! Arena representation: `nodes[i]` is the single node of the TS `nodesByTypeIndex[i]`
//! (`NodeId(i)`), `edges` holds every edge (`EdgeId`), and the head/tail indexes hold edge ids.
//! A graph is built with `&mut self` (`add_from_roots`, `join_subgraphs`, ...); the walk only
//! needs `&self` (the caches it fills are `RefCell`/`Cell`), so a built graph is shared as
//! `Rc<Graph>`. `addSubgraph(graph)` moves the subgraph graph's nodes and edges into this one,
//! remapping their ids (in TS the same objects move and `node.index` is reassigned).
//!
//! The edges of `replaceEdgeAt` are pushed to the arena without `addEdge` (as in TS, where they
//! are only placed in the head/tail indexes), see [`Graph::push_edge`].

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::utils::collections::{IndexMap, IndexSet};
use graphql_rs::is_specified_scalar_type;

use super::constants::GraphId;
use super::contexts::{ContextMask, Contexts};
use super::edge::{assert_abstract_edge, assert_field_edge, Edge, EdgeId};
use super::helpers::{is_truthy_str, score_key_fields};
use super::moves::{AbstractMove, EntityMove, FieldMove, FieldMoveOverride, Move};
use super::node::{GraphComboIds, GraphNameId, GraphNames, Names, Node, NodeId, NodeTypeState};
use super::selection::{Field, Fragment, SelectionNode, SelectionResolver};
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::FromContext;
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::union_type::UnionTypeState;
use crate::supergraph::state::SupergraphState;
use crate::utils::js::{JsError, JsResult};
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

/// PERF: the types defined in each subgraph (`typeState.byGraph.has(graphId)`), each list in
/// `SupergraphState` order. `addFromEntities` and `addUnreachableTypes` loop over every type of
/// the supergraph for every subgraph; this index gives the same types in the same order.
#[derive(Debug, Default)]
pub struct TypesBySubgraph<'s> {
    by_graph_id: IndexMap<&'s str, SubgraphTypes<'s>>,
}

/// The types of one subgraph, see [`TypesBySubgraph`].
#[derive(Debug, Default)]
pub struct SubgraphTypes<'s> {
    object_types: Vec<(&'s str, &'s ObjectTypeState)>,
    scalar_types: Vec<&'s str>,
    enum_types: Vec<&'s EnumTypeState>,
    union_types: Vec<&'s UnionTypeState>,
    interface_types: Vec<&'s InterfaceTypeState>,
}

impl<'s> TypesBySubgraph<'s> {
    pub fn new(state: &'s SupergraphState) -> Self {
        let mut index = TypesBySubgraph::default();
        for (type_name, s) in &state.object_types {
            for graph_id in s.by_graph.keys() {
                index.of_mut(graph_id).object_types.push((type_name, s));
            }
        }
        for (type_name, s) in &state.scalar_types {
            for graph_id in s.by_graph.keys() {
                index.of_mut(graph_id).scalar_types.push(type_name);
            }
        }
        for s in state.enum_types.values() {
            for graph_id in s.by_graph.keys() {
                index.of_mut(graph_id).enum_types.push(s);
            }
        }
        for s in state.union_types.values() {
            for graph_id in s.by_graph.keys() {
                index.of_mut(graph_id).union_types.push(s);
            }
        }
        for s in state.interface_types.values() {
            for graph_id in s.by_graph.keys() {
                index.of_mut(graph_id).interface_types.push(s);
            }
        }
        index
    }

    fn of_mut(&mut self, graph_id: &'s str) -> &mut SubgraphTypes<'s> {
        self.by_graph_id.entry(graph_id).or_default()
    }

    /// The types of the subgraph with this id (none if it defines no type).
    pub fn of(&self, graph_id: &str) -> Option<&SubgraphTypes<'s>> {
        self.by_graph_id.get(graph_id)
    }
}

/// `Graph`.
#[derive(Debug)]
pub struct Graph<'s> {
    warned_about_incorrect_edge: Cell<bool>,
    /// PERF: `indirectEdgesOfHead` per node, once `enable_edge_caches` was called (see there).
    indirect_edges_cache: Option<RefCell<Vec<Option<Rc<[EdgeId]>>>>>,
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
    /// `idSymbol`.
    id: GraphId,
    /// `id`: the subgraph id, or the symbol's `toString()`.
    id_str: String,
    pub name: String,
    /// PERF: the interned graph names of the `Supergraph` this graph belongs to.
    pub graph_names: Rc<GraphNames>,
    name_id: GraphNameId,
    /// PERF: the interned keys of the combos recorded in this graph's nodes (`Node::is_graph_combo_visited`).
    pub graph_combo_ids: GraphComboIds,
    /// PERF: interned type and field names, captured by lazy errors (`findDirectPaths`).
    pub names: Names,
    supergraph_state: &'s SupergraphState,
    selection_resolver: Rc<SelectionResolver<'s>>,
    ignore_inaccessible: bool,
    contexts: Option<Rc<Contexts>>,
    // When I detect a Edge<FieldMove> with override(label:), I need to store it
    // so I can later have two edges:
    //   type User { name: String @override(label: "feature") }            in subgraph A
    //   type User { name: String @override(label: "feature", from: "A") } in subgraph B
    // User/A -> (name ["feature" === true]) -> String/A
    // User/B -> (name ["feature" === false]) -> String/B
    field_edges_with_progressive_override: Vec<EdgeId>,
    // When I detect a Edge<FieldMove> with override(from:), I need to store it
    field_edges_with_override: Vec<EdgeId>,
    super_field_edges_to_apply_override: Vec<EdgeId>,
}

/// JS `encodeURIComponent`.
fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let unreserved =
            b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')');
        if unreserved {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The `fromContext` of each argument of a field in a graph.
fn from_contexts_in_graph<'a>(field: FieldStateRef<'a>, graph_id: &str) -> Vec<Option<&'a FromContext>> {
    match field {
        FieldStateRef::Object(f) => f
            .args
            .values()
            .map(|arg| arg.by_graph.get(graph_id).and_then(|a| a.from_context.as_ref()))
            .collect(),
        FieldStateRef::Interface(f) => f
            .args
            .values()
            .map(|arg| arg.by_graph.get(graph_id).and_then(|a| a.from_context.as_ref()))
            .collect(),
    }
}

/// `stripTypeModifiers(type)`.
/// PERF: borrows the type name when it has no modifiers (most output types are named types
/// here), instead of always allocating like `utils::state::strip_type_modifiers`.
fn strip_type_modifiers(ty: &str) -> Cow<'_, str> {
    if ty.contains(['!', '[', ']']) {
        Cow::Owned(ty.replace(['!', '[', ']'], ""))
    } else {
        Cow::Borrowed(ty)
    }
}

/// `{ ...providedField, typeName }`.
fn with_type_name(selection: &SelectionNode, type_name: &str) -> SelectionNode {
    match selection {
        SelectionNode::Field(f) => SelectionNode::Field(Field {
            type_name: type_name.to_string(),
            field_name: f.field_name.clone(),
            selection_set: f.selection_set.clone(),
        }),
        SelectionNode::Fragment(f) => SelectionNode::Fragment(Fragment {
            type_name: type_name.to_string(),
            selection_set: f.selection_set.clone(),
        }),
    }
}

impl<'s> Graph<'s> {
    pub fn new(
        _logger: &Logger,
        id: GraphId,
        name: String,
        supergraph_state: &'s SupergraphState,
        selection_resolver: Rc<SelectionResolver<'s>>,
        ignore_inaccessible: bool,
        contexts: Option<Rc<Contexts>>,
        graph_names: Rc<GraphNames>,
    ) -> Self {
        let is_subgraph = match &id {
            GraphId::Subgraph(_) => true,
            other => *other != GraphId::Supergraph,
        };
        let id_str = id.to_id_string();
        let name_id = graph_names.intern(&name);

        Graph {
            warned_about_incorrect_edge: Cell::new(false),
            indirect_edges_cache: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            edges_by_head_type_index: Vec::new(),
            edges_by_tail_type_index: Vec::new(),
            type_name_to_node_indexes: IndexMap::default(),
            type_children: Vec::new(),
            type_children_cache: RefCell::new(IndexMap::default()),
            is_subgraph,
            id,
            id_str,
            name,
            graph_names,
            name_id,
            graph_combo_ids: GraphComboIds::default(),
            names: Names::default(),
            supergraph_state,
            selection_resolver,
            ignore_inaccessible,
            contexts,
            field_edges_with_progressive_override: Vec::new(),
            field_edges_with_override: Vec::new(),
            super_field_edges_to_apply_override: Vec::new(),
        }
    }

    /// The node with the given id.
    pub fn node(&self, id: NodeId) -> &Node<'s> {
        &self.nodes[id.0]
    }

    /// The edge with the given id.
    pub fn edge(&self, id: EdgeId) -> &Edge<'s> {
        &self.edges[id.0]
    }

    /// `edge.isCrossGraphEdge()`.
    pub fn is_cross_graph_edge(&self, edge: EdgeId) -> bool {
        let e = self.edge(edge);
        e.is_cross_graph_edge(&self.node(e.head).graph_id, &self.node(e.tail).graph_id)
    }

    /// `edge.toString()`: `${head} -(${move})-> ${tail}`.
    pub fn edge_to_string(&self, edge: EdgeId) -> String {
        let e = self.edge(edge);
        format!(
            "{} -({})-> {}",
            self.node(e.head).to_string(),
            e.r#move.to_string(),
            self.node(e.tail).to_string()
        )
    }

    /// The `FieldMove` of an edge known to be a field edge (the override lists and the
    /// field-edge index only hold field edges).
    fn field_move_of(&self, edge: EdgeId) -> &FieldMove {
        self.edge(edge)
            .r#move
            .as_field()
            .expect("only field edges are indexed as field edges")
    }

    /// PERF: `types` is this subgraph's entry of [`TypesBySubgraph`]: the types for which
    /// `byGraph.has(this.id)`, in the order the TS loops visit them.
    pub fn add_unreachable_types(&mut self, types: Option<&SubgraphTypes<'s>>) -> JsResult<&mut Self> {
        if !self.is_supergraph() {
            let Some(types) = types else {
                return Ok(self);
            };

            for &(type_name, _) in &types.object_types {
                self.create_nodes_and_edges_for_type(type_name)?;
            }

            for &type_name in &types.scalar_types {
                self.create_node_for_scalar_type(type_name)?;
            }

            for &s in &types.enum_types {
                self.create_node_for_enum_type(s)?;
            }

            for &s in &types.union_types {
                self.create_node_for_union_type(s)?;
            }

            for &s in &types.interface_types {
                self.create_node_for_interface_type(s)?;
            }
        }

        Ok(self)
    }

    pub fn add_from_roots(&mut self) -> JsResult<&mut Self> {
        let state = self.supergraph_state;
        for type_name in ["Query", "Mutation", "Subscription"] {
            if let Some(type_state) = state.object_types.get(type_name) {
                if self.true_or_if_subgraph_then(|| type_state.by_graph.contains_key(&self.id_str)) {
                    self.create_nodes_and_edges_for_type(&type_state.name)?;
                }
            }
        }

        Ok(self)
    }

    /// Adds fields from `@interfaceObject` types.
    /// NOTE(port): the TS returns `undefined`; returning `self` changes nothing for the callers.
    pub fn add_interface_object_fields(&mut self) -> JsResult<&mut Self> {
        if self.is_subgraph {
            return Err(JsError::error("Expected to be called only on supergraph"));
        }

        let state = self.supergraph_state;

        for interface_state in state.interface_types.values() {
            if !interface_state.has_interface_object {
                // This is a regular interface, we don't need to do anything
                continue;
            }

            for implemented_by in &interface_state.implemented_by {
                let Some(object_state) = state.object_types.get(implemented_by) else {
                    return Err(JsError::error(format!(
                        "Expected object type {implemented_by} to be defined as it implements {}",
                        interface_state.name
                    )));
                };

                // If a Node is not defined, it means it's not reachable from the root as we first start from the root and add all reachable nodes
                let Some(head) = self.node_of(&object_state.name, false)? else {
                    continue;
                };

                for (interface_field_name, interface_field) in &interface_state.fields {
                    if object_state.fields.contains_key(interface_field_name) {
                        // It already has a field with the same name, we don't need to do anything
                        continue;
                    }

                    // even though it's an object type...
                    self.create_edge_for_interface_type_field(head, interface_field)?;
                }
            }

            for (type_name, s) in &state.object_types {
                if s.by_graph.contains_key(&self.id_str) {
                    self.create_nodes_and_edges_for_type(type_name)?;
                }
            }
        }

        Ok(self)
    }

    /// PERF: `types` as in `add_unreachable_types`; a subgraph graph only visits its own types.
    pub fn add_from_entities(&mut self, types: Option<&SubgraphTypes<'s>>) -> JsResult<&mut Self> {
        if self.is_subgraph {
            for &(_, type_state) in types.map(|t| t.object_types.as_slice()).unwrap_or_default() {
                if type_state.is_entity {
                    self.create_nodes_and_edges_for_type(&type_state.name)?;
                }
            }
            return Ok(self);
        }

        let state = self.supergraph_state;
        for type_state in state.object_types.values() {
            if type_state.is_entity && self.true_or_if_subgraph_then(|| type_state.by_graph.contains_key(&self.id_str))
            {
                self.create_nodes_and_edges_for_type(&type_state.name)?;
            }
        }

        Ok(self)
    }

    /// `addSubgraph(graph)`: moves the subgraph graph's nodes (`withoutState()`) and edges here.
    pub fn add_subgraph(&mut self, graph: Graph<'s>) {
        let offset = self.nodes.len();
        let Graph {
            nodes,
            edges,
            edges_by_head_type_index,
            ..
        } = graph;

        // `graph.nodesByTypeIndex.flat()` is in index order, so node `i` becomes `offset + i`.
        for node in nodes {
            self.add_node(node.without_state());
        }

        let mut edges: Vec<Option<Edge<'s>>> = edges.into_iter().map(Some).collect();
        for head_edges in edges_by_head_type_index {
            for edge_id in head_edges {
                // A subgraph graph only adds edges with `addEdge` (no `replaceEdgeAt`), so every
                // edge is in exactly one head list.
                let mut edge = edges[edge_id.0]
                    .take()
                    .expect("a subgraph graph edge is listed once in the head index");
                edge.head = NodeId(offset + edge.head.0);
                edge.tail = NodeId(offset + edge.tail.0);
                self.add_edge(edge);
            }
        }
    }

    fn connect_union_or_interface(
        &mut self,
        node_index: usize,
        same_type_name_node_indexes: &[usize],
        edges_to_add: &mut Vec<Edge<'s>>,
    ) {
        // `nodesByTypeIndex[nodeIndex]` holds a single node.
        let head_node = NodeId(node_index);
        let edges = self.edges_of_tail(head_node);

        if edges.is_empty() {
            return;
        }

        for &other_node_index in same_type_name_node_indexes {
            if node_index == other_node_index {
                continue;
            }

            let tail_node = NodeId(other_node_index);
            if head_node == tail_node {
                continue;
            }

            for &edge in &edges {
                let e = self.edge(edge);
                edges_to_add.push(Edge::new(e.head, e.r#move.clone(), tail_node));
            }
        }
    }

    fn connect_entities(
        &mut self,
        node_index: usize,
        same_type_name_node_indexes: &[usize],
        edges_to_add: &mut Vec<Edge<'s>>,
    ) -> JsResult<()> {
        let head_node = NodeId(node_index);
        let head_type_name = self.node(head_node).type_name.clone();

        for &other_node_index in same_type_name_node_indexes {
            if node_index == other_node_index {
                continue;
            }

            let tail_node = NodeId(other_node_index);
            let tail = self.node(tail_node);

            // `"isEntity" in tailNode.typeState`: objects and interfaces.
            let (is_entity, keys, is_object) = match tail.type_state {
                Some(NodeTypeState::Object(ts)) => {
                    (ts.is_entity, ts.by_graph.get(&tail.graph_id).map(|g| &g.keys), true)
                }
                Some(NodeTypeState::Interface(ts)) => {
                    (ts.is_entity, ts.by_graph.get(&tail.graph_id).map(|g| &g.keys), false)
                }
                _ => continue,
            };

            if !is_entity {
                continue;
            }

            let mut keys: Vec<_> = keys.map(|k| k.iter().collect()).unwrap_or_default();
            keys.sort_by_key(|a| score_key_fields(&a.fields));

            for key in keys {
                if key.resolvable {
                    let key_fields = self.selection_resolver.resolve(&head_type_name, &key.fields)?;
                    let r#move = if is_object {
                        Move::Entity(EntityMove::new(key_fields))
                    } else {
                        Move::Abstract(AbstractMove::new(Some(key_fields)))
                    };
                    edges_to_add.push(Edge::new(head_node, r#move, tail_node));
                }
            }
        }

        Ok(())
    }

    /// `edgesByHeadTypeIndex[head.index][index]`, throwing `Expected edge to be defined` when
    /// it is missing.
    fn head_edge_at(&self, head: NodeId, index: usize) -> JsResult<EdgeId> {
        self.edges_by_head_type_index[head.0]
            .get(index)
            .copied()
            .ok_or_else(|| JsError::error("Expected edge to be defined"))
    }

    fn add_provided_interface_fields(
        &mut self,
        head: NodeId,
        provided_fields: &[SelectionNode],
        queue: &mut Vec<ProvidedFieldsQueueItem>,
    ) -> JsResult<()> {
        let head_type_name = self.node(head).type_name.clone();
        let abstract_indexes = match self.node(head).get_abstract_edge_indexes(&head_type_name) {
            Some(indexes) if !indexes.is_empty() => indexes.clone(),
            _ => return Err(JsError::error("Expected abstract indexes to be defined")),
        };

        let mut interface_fields: Vec<SelectionNode> = Vec::new();

        let mut fields_by_type: IndexMap<String, Vec<SelectionNode>> = IndexMap::default();

        for provided_field in provided_fields {
            if provided_field.type_name() == head_type_name {
                interface_fields.push(provided_field.clone());
                continue;
            }

            fields_by_type
                .entry(provided_field.type_name().to_string())
                .or_default()
                .push(provided_field.clone());
        }

        for (type_name, provided_fields) in fields_by_type {
            let mut found: Option<(usize, EdgeId)> = None;
            for &index in &abstract_indexes {
                let potential_edge = self.head_edge_at(head, index)?;

                if self.node(self.edge(potential_edge).tail).type_name == type_name {
                    found = Some((index, potential_edge));
                    break;
                }
            }

            let Some((edge_index, edge)) = found else {
                return Err(JsError::error(format!(
                    "Expected an abstract edge matching \"{type_name}\" to be defined"
                )));
            };

            let (edge_head, edge_tail, edge_move) = {
                let e = self.edge(edge);
                (e.head, e.tail, e.r#move.clone())
            };
            let new_tail = self.duplicate_node(edge_tail, &edge_move);
            let new_edge = self.push_edge(Edge::new(edge_head, edge_move, new_tail));
            self.replace_edge_at(edge_head.0, edge_tail.0, new_edge, edge_index);

            queue.push(ProvidedFieldsQueueItem {
                head: new_tail,
                provided_fields,
            });
        }

        if interface_fields.is_empty() {
            return Ok(());
        }

        for &index in &abstract_indexes {
            let edge = self.head_edge_at(head, index)?;

            assert_abstract_edge(self, edge)?;

            if self.is_cross_graph_edge(edge) {
                continue;
            }

            let (edge_head, edge_tail, edge_move) = {
                let e = self.edge(edge);
                (e.head, e.tail, e.r#move.clone())
            };
            let new_tail = self.duplicate_node(edge_tail, &edge_move);
            let new_edge = self.push_edge(Edge::new(edge_head, Move::Abstract(AbstractMove::new(None)), new_tail));
            self.replace_edge_at(edge_head.0, edge_tail.0, new_edge, index);

            let new_tail_type_name = self.node(new_tail).type_name.clone();
            queue.push(ProvidedFieldsQueueItem {
                head: new_tail,
                provided_fields: interface_fields
                    .iter()
                    .map(|f| with_type_name(f, &new_tail_type_name))
                    .collect(),
            });
        }

        Ok(())
    }

    fn add_provided_union_fields(
        &mut self,
        head: NodeId,
        provided_fields: &[SelectionNode],
        queue: &mut Vec<ProvidedFieldsQueueItem>,
    ) -> JsResult<()> {
        let head_type_name = self.node(head).type_name.clone();
        let abstract_indexes = match self.node(head).get_abstract_edge_indexes(&head_type_name) {
            Some(indexes) if !indexes.is_empty() => indexes.clone(),
            _ => return Err(JsError::error("Expected abstract indexes to be defined")),
        };

        let mut fields_by_type: IndexMap<String, Vec<SelectionNode>> = IndexMap::default();

        for provided_field in provided_fields {
            let SelectionNode::Fragment(fragment) = provided_field else {
                return Err(JsError::error(
                    "Selection on union must be fragment. Received \"field\".",
                ));
            };

            fields_by_type
                .entry(fragment.type_name.clone())
                .or_default()
                .extend(fragment.selection_set.iter().cloned());
        }

        for (type_name, provided_fields) in fields_by_type {
            let mut found: Option<(usize, EdgeId)> = None;
            for &index in &abstract_indexes {
                // NOTE(port): the TS reads `potentialEdge.tail` without a check; a missing edge
                // would be a TypeError.
                let Some(&potential_edge) = self.edges_by_head_type_index[head.0].get(index) else {
                    return Err(JsError::type_error(
                        "Cannot read properties of undefined (reading 'tail')",
                    ));
                };

                if self.node(self.edge(potential_edge).tail).type_name == type_name {
                    found = Some((index, potential_edge));
                    break;
                }
            }

            let Some((edge_index, edge)) = found else {
                return Err(JsError::error(format!(
                    "Expected an abstract edge matching \"{type_name}\" to be defined"
                )));
            };

            let (edge_head, edge_tail, edge_move) = {
                let e = self.edge(edge);
                (e.head, e.tail, e.r#move.clone())
            };
            let new_tail = self.duplicate_node(edge_tail, &edge_move);
            let new_edge = self.push_edge(Edge::new(edge_head, edge_move, new_tail));
            self.replace_edge_at(edge_head.0, edge_tail.0, new_edge, edge_index);

            queue.push(ProvidedFieldsQueueItem {
                head: new_tail,
                provided_fields,
            });
        }

        Ok(())
    }

    fn add_provided_field(
        &mut self,
        head: NodeId,
        provided_field: &SelectionNode,
        queue: &mut Vec<ProvidedFieldsQueueItem>,
    ) -> JsResult<()> {
        let provided_field = match provided_field {
            // As we only need to check if all fields are reachable from the head, we can ignore __typename
            SelectionNode::Field(f) if f.field_name == "__typename" => return Ok(()),
            SelectionNode::Fragment(fragment) => {
                queue.push(ProvidedFieldsQueueItem {
                    head,
                    provided_fields: fragment.selection_set.clone(),
                });
                return Ok(());
            }
            SelectionNode::Field(f) => f,
        };

        let indexes = match self.node(head).get_field_edge_indexes(&provided_field.field_name) {
            Some(indexes) if !indexes.is_empty() => indexes.clone(),
            _ => {
                let type_state = self.node(head).type_state;
                if matches!(type_state, Some(NodeTypeState::Object(_))) {
                    return Err(JsError::error(format!(
                        "Expected indexes to be defined: {}.{}",
                        provided_field.type_name, provided_field.field_name
                    )));
                } else {
                    return Err(JsError::error(format!(
                        "Expected {}.{} to be point to an object type, other kinds are not supported (received: {})",
                        provided_field.type_name,
                        provided_field.field_name,
                        type_state.map(|t| t.kind()).unwrap_or("undefined")
                    )));
                }
            }
        };

        for index in indexes {
            let edge = self.head_edge_at(head, index)?;

            assert_field_edge(self, edge)?;

            if self.is_cross_graph_edge(edge) {
                continue;
            }

            let (edge_head, edge_tail, edge_move) = {
                let e = self.edge(edge);
                (e.head, e.tail, e.r#move.clone())
            };
            let field_move = self.field_move_of(edge).clone();
            let new_tail = self.duplicate_node(edge_tail, &edge_move);
            let new_edge = self.push_edge(Edge::new(
                edge_head,
                Move::Field(FieldMove::new(
                    field_move.type_name,
                    field_move.field_name,
                    field_move.requires,
                    field_move.provides,
                    None,
                    true,
                    field_move.required_contexts,
                )),
                new_tail,
            ));
            self.replace_edge_at(edge_head.0, edge_tail.0, new_edge, index);

            if let Some(selection_set) = &provided_field.selection_set {
                queue.push(ProvidedFieldsQueueItem {
                    head: new_tail,
                    provided_fields: selection_set.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn join_subgraphs(&mut self) -> JsResult<&mut Self> {
        // for each entity type, we want to assign one entity node to a matching entity node in other subgraphs
        // for each interface type, we want to assign one interface node to a matching interface node in other subgraphs
        let mut edges_to_add: Vec<Edge<'s>> = Vec::new();

        for i in 0..self.nodes.len() {
            let type_node = NodeId(i);
            let Some(type_state) = self.node(type_node).type_state else {
                continue;
            };

            let Some(other_nodes_indexes) = self.get_indexes_of_type(&self.node(type_node).type_name).cloned() else {
                continue;
            };

            let entity_interfaces = match type_state {
                NodeTypeState::Object(ts) if ts.is_entity => Some(&ts.interfaces),
                NodeTypeState::Interface(ts) if ts.is_entity => Some(&ts.interfaces),
                _ => None,
            };

            if let Some(interfaces) = entity_interfaces {
                self.connect_entities(i, &other_nodes_indexes, &mut edges_to_add)?;

                for &h in &other_nodes_indexes {
                    let head = NodeId(h);

                    // For example, if we have
                    // Subgraph 1:
                    //  type A @interfaceObject @key(fields: "id") {}
                    // Subgraph 2:
                    //  interface A @key(fields: "id") {}
                    //  type B implements A @key(fields: "id") {}
                    // We need to add an edge from A to B
                    //
                    // The general idea here is to allow to move from a type that implements an interface to another type that implements the same interface

                    for interface_name in interfaces {
                        let interface_nodes = self.nodes_of(interface_name, false)?;

                        if interface_nodes.is_empty() {
                            continue;
                        }

                        let Some(NodeTypeState::Interface(interface_type_state)) =
                            self.node(interface_nodes[0]).type_state
                        else {
                            continue;
                        };

                        if !interface_type_state.has_interface_object {
                            continue;
                        }

                        for &interface_node in &interface_nodes {
                            let node = self.node(interface_node);
                            let Some(NodeTypeState::Interface(interface_state)) = node.type_state else {
                                return Err(JsError::error(format!(
                                    "Expected interfaceNode {} to be an interface",
                                    node.to_string()
                                )));
                            };

                            let keys = match interface_state.by_graph.get(&node.graph_id).map(|g| &g.keys) {
                                Some(keys) if !keys.is_empty() => keys,
                                _ => continue,
                            };

                            for key in keys {
                                if !key.resolvable {
                                    continue;
                                }

                                let key_fields = self.selection_resolver.resolve(interface_name, &key.fields)?;
                                edges_to_add.push(Edge::new(
                                    head,
                                    Move::Abstract(AbstractMove::new(Some(key_fields))),
                                    interface_node,
                                ));
                            }
                        }
                    }
                }
            } else if matches!(type_state, NodeTypeState::Union(_) | NodeTypeState::Interface(_)) {
                self.connect_union_or_interface(i, &other_nodes_indexes, &mut edges_to_add);
            }
        }

        while let Some(edge) = edges_to_add.pop() {
            self.add_edge(edge);
        }

        // iterate over all edges
        let head_edges_length = self.edges_by_head_type_index.len();
        for head_index in 0..head_edges_length {
            let edges_length = self.edges_by_head_type_index[head_index].len();
            for edge_index in 0..edges_length {
                let edge = self.edges_by_head_type_index[head_index][edge_index];

                // if it's a cross-graph edge, we don't want to resolve it
                if self.is_cross_graph_edge(edge) {
                    continue;
                }

                // ignore non-field edges
                let provides = match &self.edge(edge).r#move {
                    Move::Field(FieldMove {
                        provides: Some(provides),
                        ..
                    }) => provides.clone(),
                    _ => continue,
                };

                // find field edges that are provided and mark them as resolvable.
                // if they are not available, create them
                let (edge_head, edge_tail, edge_move) = {
                    let e = self.edge(edge);
                    (e.head, e.tail, e.r#move.clone())
                };
                let new_tail = self.duplicate_node(edge_tail, &edge_move);
                let new_edge = self.push_edge(Edge::new(edge_head, edge_move, new_tail));
                self.replace_edge_at(head_index, edge_tail.0, new_edge, edge_index);

                let mut queue: Vec<ProvidedFieldsQueueItem> = vec![ProvidedFieldsQueueItem {
                    head: new_tail,
                    provided_fields: provides.selection_set.clone(),
                }];

                while let Some(item) = queue.pop() {
                    let ProvidedFieldsQueueItem { head, provided_fields } = item;

                    match self.node(head).type_state {
                        Some(NodeTypeState::Interface(_)) => {
                            self.add_provided_interface_fields(head, &provided_fields, &mut queue)?;
                            continue;
                        }
                        Some(NodeTypeState::Union(_)) => {
                            self.add_provided_union_fields(head, &provided_fields, &mut queue)?;
                            continue;
                        }
                        _ => {}
                    }

                    for provided_field in &provided_fields {
                        self.add_provided_field(head, provided_field, &mut queue)?;
                    }
                }
            }
        }

        Ok(self)
    }

    pub fn add_overridden_fields(&mut self) -> JsResult<&mut Self> {
        if self.is_supergraph() {
            let state = self.supergraph_state;
            for super_field_edge in self.super_field_edges_to_apply_override.clone() {
                // We need to look for field states in graphs
                // and add two edges (override: on and off)
                // when a label is available
                let (type_name, field_name) = {
                    let m = self.field_move_of(super_field_edge);
                    (m.type_name.clone(), m.field_name.clone())
                };
                let (head, tail) = {
                    let e = self.edge(super_field_edge);
                    (e.head, e.tail)
                };
                let Some(object_type_state) = state.object_types.get(&type_name) else {
                    return Err(JsError::error("Expected to find object type state"));
                };

                let Some(field_state) = object_type_state.fields.get(&field_name) else {
                    return Err(JsError::error("Expected to find field state"));
                };

                if !is_truthy_str(&field_state.override_label) {
                    return Err(JsError::error("Expected a label on a field"));
                }

                for field_state_in_graph in field_state.by_graph.values() {
                    if is_truthy_str(&field_state_in_graph.override_label)
                        && is_truthy_str(&field_state_in_graph.r#override)
                    {
                        let from_graph_id = field_state_in_graph
                            .r#override
                            .as_deref()
                            .and_then(|graph_name| self.graph_name_to_id(graph_name));
                        let Some(from_graph_id) = from_graph_id.filter(|id| !id.is_empty()) else {
                            // Looks like there is no such graph, we ignore it
                            continue;
                        };

                        // When the label is on, use the field with `@override(from:)`
                        if let Move::Field(m) = &mut self.edges[super_field_edge.0].r#move {
                            m.r#override = Some(FieldMoveOverride {
                                label: field_state_in_graph.override_label.clone(),
                                value: true,
                                from_graph_id: Some(from_graph_id),
                            });
                        }

                        // When the label is off, use the other field
                        self.add_edge(Edge::new(
                            head,
                            Move::Field(FieldMove::new(
                                type_name.clone(),
                                field_name.clone(),
                                None,
                                None,
                                Some(FieldMoveOverride {
                                    label: field_state_in_graph.override_label.clone(),
                                    value: false,
                                    from_graph_id: None,
                                }),
                                false,
                                ContextMask::zero(),
                            )),
                            tail,
                        ));
                    }
                }
            }
            return Ok(self);
        }

        if !self.is_merged_graph() {
            return Err(JsError::error("Expected to be called only on merged graph"));
        }

        for field_with_override in self.field_edges_with_progressive_override.clone() {
            let (head_type_name, field_name, from_graph_id) = {
                let m = self.field_move_of(field_with_override);
                let Some(r#override) = &m.r#override else {
                    return Err(JsError::error("Expected edge.move.override to be defined"));
                };

                let from_graph_id = match &r#override.from_graph_id {
                    Some(id) if !id.is_empty() => id.clone(),
                    // This edge is the source of the override,
                    // it should contain the `@override(from:)` argument
                    _ => return Err(JsError::error("Expected fromGraphId to be defined")),
                };

                (
                    self.node(self.edge(field_with_override).head).type_name.clone(),
                    m.field_name.clone(),
                    from_graph_id,
                )
            };

            let nodes = self.nodes_of(&head_type_name, true)?;
            for node in nodes {
                if self.node(node).graph_id != from_graph_id {
                    // We're not in the right graph
                    continue;
                }

                let field_edges = self.field_edges_of_head(node, &field_name)?;
                for field_edge in field_edges {
                    if self.field_move_of(field_edge).provided {
                        // We don't want to override provided fields
                        continue;
                    }

                    // `fieldWithOverride.move.override` is read again for every edge, as in TS
                    // (the edge itself may have been updated by this loop).
                    let (label, value) = match &self.field_move_of(field_with_override).r#override {
                        Some(o) => (o.label.clone(), o.value),
                        None => {
                            return Err(JsError::type_error("Cannot read properties of null (reading 'label')"));
                        }
                    };

                    self.edges[field_edge.0].update_override(FieldMoveOverride {
                        label,
                        // we negate the value, because we want to have 2 possible values
                        // for a given progressive override
                        value: !value,
                        from_graph_id: None,
                    })?;
                }
            }
        }

        for field_with_override in self.field_edges_with_override.clone() {
            let (head_type_name, field_name, from_graph_id) = {
                let m = self.field_move_of(field_with_override);
                let Some(r#override) = &m.r#override else {
                    return Err(JsError::error("Expected edge.move.override to be defined"));
                };

                let from_graph_id = match &r#override.from_graph_id {
                    Some(id) if !id.is_empty() => id.clone(),
                    // This edge is the source of the override,
                    // it should contain the `@override(from:)` argument
                    _ => return Err(JsError::error("Expected fromGraphId to be defined")),
                };

                (
                    self.node(self.edge(field_with_override).head).type_name.clone(),
                    m.field_name.clone(),
                    from_graph_id,
                )
            };

            let nodes = self.nodes_of(&head_type_name, true)?;
            for node in nodes {
                if self.node(node).graph_id != from_graph_id {
                    // We're not in the right graph
                    continue;
                }

                let field_edges = self.field_edges_of_head(node, &field_name)?;
                for field_edge in field_edges {
                    if self.field_move_of(field_edge).provided {
                        // We don't want to override provided fields
                        continue;
                    }

                    self.ignore_edge(field_edge);
                }
            }
        }

        Ok(self)
    }

    fn duplicate_node(&mut self, original_node: NodeId, r#move: &Move) -> NodeId {
        let (type_name, type_state, graph_id, graph_name, graph_name_id) = {
            let n = self.node(original_node);
            (
                n.type_name.clone(),
                n.type_state,
                n.graph_id.clone(),
                n.graph_name.clone(),
                n.graph_name_id,
            )
        };
        let new_node = self.create_node(&type_name, type_state, &graph_id, &graph_name, graph_name_id);

        for edge in self.edges_of_head(original_node) {
            let e = self.edge(edge);
            let new_edge = Edge::new(new_node, e.r#move.clone(), e.tail);
            self.add_edge(new_edge);
        }

        if let Move::Field(field_move) = r#move {
            self.nodes[new_node.0].debug_post_fix = format!(" (for {})", field_move.to_string());
        }

        new_node
    }

    fn replace_edge_at(&mut self, head_index: usize, tail_index: usize, new_edge: EdgeId, edge_index: usize) {
        self.edges_by_head_type_index[head_index][edge_index] = new_edge;

        let mut new_edges_by_tail: Vec<EdgeId> = self.edges_by_tail_type_index[tail_index]
            .iter()
            .copied()
            .filter(|&edge| edge != new_edge)
            .collect();

        new_edges_by_tail.push(new_edge);
        self.edges_by_tail_type_index[tail_index] = new_edges_by_tail;
    }

    /// `print(asLink = false)`: Graphviz dot output (debugging).
    pub fn print(&self, as_link: bool) -> String {
        let mut str = String::from("digraph G {");

        if self.supergraph_state.object_types.contains_key("Query") {
            str += "\n root -> Query";
        }

        if self.supergraph_state.object_types.contains_key("Mutation") {
            str += "\n root -> Mutation";
        }

        if self.supergraph_state.object_types.contains_key("Subscription") {
            str += "\n root -> Subscription";
        }

        for &edge in self.edges_by_head_type_index.iter().flatten() {
            let e = self.edge(edge);
            if e.is_ignored() {
                continue;
            }

            let head = self.node(e.head);
            if head.type_name == "Query" {
                str += &format!("\n  \"Query\" -> \"{}\";", head.to_string());
            } else if head.type_name == "Mutation" {
                str += &format!("\n  \"Mutation\" -> \"{}\";", head.to_string());
            } else if head.type_name == "Subscription" {
                str += &format!("\n  \"Subscription\" -> \"{}\";", head.to_string());
            }

            str += &format!(
                "\n  \"{}\" -> \"{}\" [label=\"{}\"];",
                head.to_string(),
                self.node(e.tail).to_string(),
                e.r#move.to_string()
            );
        }

        str += "\n}";

        if as_link {
            return format!(
                "https://dreampuf.github.io/GraphvizOnline/#{}",
                encode_uri_component(&str)
            );
        }

        str
    }

    pub fn graph_name_to_id(&self, graph_name: &str) -> Option<String> {
        for (id, subgraph_state) in &self.supergraph_state.subgraphs {
            if subgraph_state.graph.name == graph_name {
                return Some(id.clone());
            }
        }
        None
    }

    /// `nodeOf(typeName, failIfMissing = true)`: throws when missing (if `fail_if_missing`) or
    /// when the type has more than one node.
    pub fn node_of(&self, type_name: &str, fail_if_missing: bool) -> JsResult<Option<NodeId>> {
        let Some(indexes) = self.get_indexes_of_type(type_name) else {
            if fail_if_missing {
                return Err(JsError::error(format!(
                    "Expected TypeNode({type_name}) to be inserted first in graph {}",
                    self.id_str
                )));
            }

            return Ok(None);
        };

        if indexes.len() > 1 {
            return Err(JsError::error(format!(
                "Expected only one node for {type_name} in graph {}",
                self.id_str
            )));
        }

        Ok(Some(NodeId(indexes[0])))
    }

    /// `nodesOf(typeName, failIfMissing = true)`.
    pub fn nodes_of(&self, type_name: &str, fail_if_missing: bool) -> JsResult<Vec<NodeId>> {
        let Some(indexes) = self.get_indexes_of_type(type_name) else {
            if fail_if_missing {
                return Err(JsError::error(format!(
                    "Expected TypeNode({type_name}) to be inserted first in graph {}",
                    self.id_str
                )));
            }

            return Ok(Vec::new());
        };

        Ok(indexes.iter().map(|&i| NodeId(i)).collect())
    }

    /// Throws `Expected edge to be defined at index X` for an index without an edge.
    fn get_same_graph_edges_of_index(
        &self,
        head: NodeId,
        indexes: Option<&Vec<usize>>,
        kind: &str,
    ) -> JsResult<Vec<EdgeId>> {
        let mut edges = Vec::new();
        self.push_same_graph_edges_of_index(&mut edges, head, indexes, kind)?;
        Ok(edges)
    }

    /// PERF: `getSameGraphEdgesOfIndex` appending to `edges`, so `indirectEdgesOfHead` fills one list.
    fn push_same_graph_edges_of_index(
        &self,
        edges: &mut Vec<EdgeId>,
        head: NodeId,
        indexes: Option<&Vec<usize>>,
        kind: &str,
    ) -> JsResult<()> {
        let Some(indexes) = indexes else {
            return Ok(());
        };
        edges.reserve(indexes.len());

        let head_node = self.node(head);
        for &i in indexes {
            let Some(&edge) = self.edges_by_head_type_index[head.0].get(i) else {
                return Err(JsError::error(format!("Expected edge to be defined at index {i}")));
            };

            let e = self.edge(edge);
            if e.is_ignored() {
                continue;
            }

            // PERF: interned names compare like the names.
            if self.node(e.head).graph_name_id == head_node.graph_name_id {
                edges.push(edge);
                continue;
            }

            if !self.warned_about_incorrect_edge.get() {
                // NOTE(port): `console.error` in TS.
                eprintln!(
                    "Expected edge to be in the same graph as head ({kind}){}",
                    self.edge_to_string(edge)
                );
                self.warned_about_incorrect_edge.set(true);
            }
        }

        Ok(())
    }

    pub fn field_edges_of_head(&self, head: NodeId, field_name: &str) -> JsResult<Vec<EdgeId>> {
        self.get_same_graph_edges_of_index(head, self.node(head).get_field_edge_indexes(field_name), "field")
    }

    pub fn abstract_edges_of_head(&self, head: NodeId) -> JsResult<Vec<EdgeId>> {
        let node = self.node(head);
        self.get_same_graph_edges_of_index(head, node.get_abstract_edge_indexes(&node.type_name), "abstract")
    }

    fn filter_edges(&self, edges: &[EdgeId]) -> Vec<EdgeId> {
        edges
            .iter()
            .copied()
            .filter(|&edge| !self.edge(edge).is_ignored())
            .collect()
    }

    pub fn entity_edges_of_head(&self, head: NodeId) -> JsResult<Vec<EdgeId>> {
        let node = self.node(head);
        self.get_same_graph_edges_of_index(head, node.get_entity_edge_indexes(&node.type_name), "entity")
    }

    /// PERF: called once the graph is complete (no node or edge is added, ignored or replaced
    /// afterwards). From then on `indirectEdgesOfHead` remembers its result per node: the result
    /// only depends on the finished graph. A throw is not remembered (the next call throws
    /// again), and the one-time warning is printed by the first computation either way.
    pub fn enable_edge_caches(&mut self) {
        self.indirect_edges_cache = Some(RefCell::new(vec![None; self.nodes.len()]));
    }

    pub fn indirect_edges_of_head(&self, head: NodeId) -> JsResult<Rc<[EdgeId]>> {
        if let Some(cache) = &self.indirect_edges_cache {
            if let Some(edges) = &cache.borrow()[head.0] {
                return Ok(edges.clone());
            }
        }

        let node = self.node(head);
        let mut all = Vec::new();
        self.push_same_graph_edges_of_index(
            &mut all,
            head,
            node.get_cross_graph_edge_indexes(&node.type_name),
            "cross-graph",
        )?;
        self.push_same_graph_edges_of_index(
            &mut all,
            head,
            node.get_abstract_edge_indexes(&node.type_name),
            "abstract",
        )?;

        // `.filter((edge, i, all) => all.indexOf(edge) === i)`
        // PERF: in place; the kept edges are the prefix `all[..unique]`. Long lists (an entity
        // type shared by many subgraphs has an edge to each of them) use a set instead of the
        // quadratic scan; both keep the first occurrence of each edge, in order.
        if all.len() > 32 {
            let mut seen: IndexSet<EdgeId> = IndexSet::with_capacity_and_hasher(all.len(), Default::default());
            all.retain(|&edge| seen.insert(edge));
        } else {
            let mut unique = 0;
            for i in 0..all.len() {
                let edge = all[i];
                if !all[..unique].contains(&edge) {
                    all[unique] = edge;
                    unique += 1;
                }
            }
            all.truncate(unique);
        }

        let edges: Rc<[EdgeId]> = all.into();
        if let Some(cache) = &self.indirect_edges_cache {
            cache.borrow_mut()[head.0] = Some(edges.clone());
        }
        Ok(edges)
    }

    pub fn edges_of_head(&self, head: NodeId) -> Vec<EdgeId> {
        let edges: Vec<EdgeId> = self.edges_by_head_type_index[head.0]
            .iter()
            .copied()
            .filter(|&edge| self.edge(edge).head == head)
            .collect();
        self.filter_edges(&edges)
    }

    pub fn edges_of_tail(&self, tail: NodeId) -> Vec<EdgeId> {
        let edges: Vec<EdgeId> = self.edges_by_tail_type_index[tail.0]
            .iter()
            .copied()
            .filter(|&edge| self.edge(edge).tail == tail)
            .collect();
        self.filter_edges(&edges)
    }

    pub fn possible_types_of(&self, type_name: &str) -> Vec<String> {
        if let Some(interface_state) = self.supergraph_state.interface_types.get(type_name) {
            return interface_state.implemented_by.iter().cloned().collect();
        }

        if let Some(union_state) = self.supergraph_state.union_types.get(type_name) {
            return union_state.members.iter().cloned().collect();
        }

        vec![type_name.to_string()]
    }

    /// Throws `Could not find an index of type: X` (cannot happen: every child type has a node).
    pub fn can_reach_type_from_type(&self, from_type_name: &str, to_type_name: &str) -> JsResult<bool> {
        if from_type_name == to_type_name {
            return Ok(true);
        }

        let Some(from_type_indexes) = self.get_indexes_of_type(from_type_name) else {
            return Ok(false);
        };

        // Mark all the nodes as not visited
        let mut visited = vec![false; self.type_children.len()];

        // Create a queue for BFS
        let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();

        for &i in from_type_indexes {
            // Mark the current node as visited and enqueue it
            visited[i] = true;
        }
        queue.push_back(from_type_name.to_string());

        let mut cache = self.type_children_cache.borrow_mut();

        while let Some(type_name) = queue.pop_front() {
            if type_name.is_empty() {
                return Err(JsError::error("Unexpected end of queue"));
            }

            let Some(type_indexes) = self.get_indexes_of_type(&type_name) else {
                return Err(JsError::error(format!("Could not find an index of type: {type_name}")));
            };

            // Mark type accessible by From type
            cache.insert(format!("{from_type_name} -> {type_name}"), true);

            if type_name == to_type_name {
                return Ok(true);
            }

            for &type_index in type_indexes {
                let children = &self.type_children[type_index];

                for child_type_name in children {
                    let Some(child_type_indexes) = self.get_indexes_of_type(child_type_name) else {
                        return Err(JsError::error(format!("Could not find an index of type: {type_name}")));
                    };

                    for &child_type_index in child_type_indexes {
                        if !visited[child_type_index] {
                            visited[child_type_index] = true;
                            cache.insert(format!("{from_type_name} -> {child_type_name}"), true);
                            cache.insert(format!("{type_name} -> {child_type_name}"), true);
                            queue.push_back(child_type_name.clone());
                        }
                    }
                }
            }
        }

        cache.insert(format!("{from_type_name} -> {to_type_name}"), false);
        Ok(false)
    }

    fn create_nodes_and_edges_for_type(&mut self, type_name: &str) -> JsResult<NodeId> {
        // PERF: every branch below starts with `ensureNonOrSingleNode(typeName)` and returns the
        // existing node (or throws) when there is one, and a node only exists for a type of one
        // of these kinds, so checking first skips the type-state lookups for known types.
        if let Some(existing) = self.ensure_non_or_single_node(type_name)? {
            return Ok(existing);
        }

        let state = self.supergraph_state;

        if let Some(type_state) = state.object_types.get(type_name) {
            return self.create_nodes_and_edges_for_object_type(type_state);
        }

        if is_specified_scalar_type(type_name) || state.scalar_types.contains_key(type_name) {
            return self.create_node_for_scalar_type(type_name);
        }

        if let Some(type_state) = state.enum_types.get(type_name) {
            return self.create_node_for_enum_type(type_state);
        }

        if let Some(type_state) = state.union_types.get(type_name) {
            return self.create_node_for_union_type(type_state);
        }

        if let Some(type_state) = state.interface_types.get(type_name) {
            return self.create_node_for_interface_type(type_state);
        }

        Err(JsError::error(format!(
            "Not implemented path: createNodesAndEdgesForType({type_name})"
        )))
    }

    fn ensure_non_or_single_node(&self, type_name: &str) -> JsResult<Option<NodeId>> {
        let Some(indexes) = self.type_name_to_node_indexes.get(type_name) else {
            return Ok(None);
        };

        if indexes.len() > 1 {
            return Err(JsError::error(format!(
                "Expected only one node for {type_name} in graph {}",
                self.id_str
            )));
        }

        Ok(Some(NodeId(indexes[0])))
    }

    fn create_nodes_and_edges_for_object_type(&mut self, type_state: &'s ObjectTypeState) -> JsResult<NodeId> {
        if let Some(existing) = self.ensure_non_or_single_node(&type_state.name)? {
            return Ok(existing);
        }

        let head = self.create_type_node(&type_state.name, Some(NodeTypeState::Object(type_state)))?;

        for field in type_state.fields.values() {
            if self.true_or_if_subgraph_then(|| field.by_graph.contains_key(&self.id_str)) {
                self.create_edge_for_object_type_field(head, field)?;
            }
        }

        Ok(head)
    }

    fn create_node_for_scalar_type(&mut self, type_name: &str) -> JsResult<NodeId> {
        if let Some(existing) = self.ensure_non_or_single_node(type_name)? {
            return Ok(existing);
        }

        let state = self.supergraph_state;
        self.create_type_node(type_name, state.scalar_types.get(type_name).map(NodeTypeState::Scalar))
    }

    fn create_node_for_enum_type(&mut self, type_state: &'s EnumTypeState) -> JsResult<NodeId> {
        if let Some(existing) = self.ensure_non_or_single_node(&type_state.name)? {
            return Ok(existing);
        }

        self.create_type_node(&type_state.name, Some(NodeTypeState::Enum(type_state)))
    }

    fn create_node_for_union_type(&mut self, type_state: &'s UnionTypeState) -> JsResult<NodeId> {
        if let Some(existing) = self.ensure_non_or_single_node(&type_state.name)? {
            return Ok(existing);
        }

        let head = self.create_type_node(&type_state.name, Some(NodeTypeState::Union(type_state)))?;

        let members = if self.is_supergraph() {
            Some(&type_state.members)
        } else {
            type_state.by_graph.get(&self.id_str).map(|g| &g.members)
        };

        if let Some(members) = members {
            for member_type_name in members {
                let tail = self.create_nodes_and_edges_for_type(member_type_name)?;
                // TODO: if it's inaccessible and this.ignoreInaccessible is true, we should not add the edge
                self.add_edge(Edge::new(head, Move::Abstract(AbstractMove::new(None)), tail));
            }
        }

        Ok(head)
    }

    fn create_node_for_interface_type(&mut self, type_state: &'s InterfaceTypeState) -> JsResult<NodeId> {
        if let Some(existing) = self.ensure_non_or_single_node(&type_state.name)? {
            return Ok(existing);
        }

        let head = self.create_type_node(&type_state.name, Some(NodeTypeState::Interface(type_state)))?;
        let implemented_by = if self.is_supergraph() {
            Some(&type_state.implemented_by)
        } else {
            type_state.by_graph.get(&self.id_str).map(|g| &g.implemented_by)
        };

        if let Some(implemented_by) = implemented_by {
            for member_type_name in implemented_by {
                let tail = self.create_nodes_and_edges_for_type(member_type_name)?;
                self.add_edge(Edge::new(head, Move::Abstract(AbstractMove::new(None)), tail));
            }
        }

        if type_state.has_interface_object && self.is_subgraph {
            // We need to add fields of interface types with @interfaceObject
            // but only add fields that are owned by the subgraph.
            for field in type_state.fields.values() {
                if field.by_graph.contains_key(&self.id_str) {
                    self.create_edge_for_interface_type_field(head, field)?;
                }
            }
        } else if self.is_subgraph {
            // We're adding just leafs
            for field in type_state.fields.values() {
                if field.is_leaf && field.by_graph.contains_key(&self.id_str) {
                    self.create_edge_for_interface_type_field(head, field)?;
                }
            }
        }

        Ok(head)
    }

    fn create_edge_for_interface_type_field(
        &mut self,
        head: NodeId,
        field: &'s InterfaceTypeFieldState,
    ) -> JsResult<()> {
        if self.ignore_inaccessible && field.inaccessible {
            return Ok(());
        }

        let output_type_name = strip_type_modifiers(&field.r#type);
        let tail = self.create_nodes_and_edges_for_type(&output_type_name)?;

        let head_type_name = self.node(head).type_name.clone();
        let field_state_in_graph = field.by_graph.get(&self.node(head).graph_id);
        let requires = field_state_in_graph.and_then(|f| f.requires.as_deref());
        let provides = field_state_in_graph.and_then(|f| f.provides.as_deref());

        let requires = match requires {
            Some(requires) if !requires.is_empty() => Some(self.selection_resolver.resolve(&head_type_name, requires)?),
            _ => None,
        };
        let provides = match provides {
            Some(provides) if !provides.is_empty() => {
                Some(self.selection_resolver.resolve(&output_type_name, provides)?)
            }
            _ => None,
        };
        let required_contexts = self.required_contexts_of_field(head, FieldStateRef::Interface(field));

        self.add_edge(Edge::new(
            head,
            Move::Field(FieldMove::new(
                head_type_name,
                field.name.clone(),
                requires,
                provides,
                None,
                false,
                required_contexts,
            )),
            tail,
        ));

        Ok(())
    }

    /// Mask of contexts (prefixed with the subgraph name) that the field's `@fromContext`
    /// arguments read from, in the subgraph owning the head node.
    fn required_contexts_of_field(&self, head: NodeId, field: FieldStateRef<'_>) -> ContextMask {
        let Some(contexts) = &self.contexts else {
            return ContextMask::zero();
        };

        let mut required_contexts = ContextMask::zero();
        let head_node = self.node(head);

        for from_context in from_contexts_in_graph(field, &head_node.graph_id) {
            let Some(from_context) = from_context else {
                continue;
            };

            let context_name = format!("{}__{}", head_node.graph_name, from_context.context);

            required_contexts.or_assign(&contexts.mask_of(&context_name));
        }

        required_contexts
    }

    fn create_edge_for_object_type_field(&mut self, head: NodeId, field: &'s ObjectTypeFieldState) -> JsResult<()> {
        if self.ignore_inaccessible && field.inaccessible {
            return Ok(());
        }

        let state = self.supergraph_state;

        if self.is_supergraph() && field.by_graph.len() == 1 {
            if let Some((graph_id, field_in_graph)) = field.by_graph.first() {
                let is_external = field_in_graph.external;
                let is_federation_v1 = state
                    .subgraphs
                    .get(graph_id)
                    .is_some_and(|s| s.federation.version == FederationVersion::V1_0);

                if is_external && is_federation_v1 {
                    return Ok(());
                }
            }
        }

        // We always want to connect it to the field type within our graph not the field type from another graph/supergraph
        let field_type = field
            .by_graph
            .get(&self.id_str)
            .map(|f| f.r#type.as_str())
            .unwrap_or(&field.r#type);
        let output_type_name = strip_type_modifiers(field_type);

        let tail = self.create_nodes_and_edges_for_type(&output_type_name)?;

        let head_type_name = self.node(head).type_name.clone();

        if self.is_supergraph() {
            // If the field has a @override(label:),
            // we should add two edges, for each label
            // - one for the case when the gateway should override the field
            // - one for the case when it should not
            // This is because we want to allow progressive overrides
            let edge = Edge::new(
                head,
                Move::Field(FieldMove::new(
                    head_type_name,
                    field.name.clone(),
                    None,
                    None,
                    None,
                    false,
                    ContextMask::zero(),
                )),
                tail,
            );
            let apply_override = is_truthy_str(&field.r#override) && is_truthy_str(&field.override_label);
            let edge = self.add_edge(edge);
            if apply_override {
                self.super_field_edges_to_apply_override.push(edge);
            }
            return Ok(());
        }

        let field_state_in_graph = field.by_graph.get(&self.node(head).graph_id);
        let requires = field_state_in_graph.and_then(|f| f.requires.as_deref());
        let provides = field_state_in_graph.and_then(|f| f.provides.as_deref());
        let r#override = field_state_in_graph.and_then(|f| f.r#override.as_deref());
        let override_label = field_state_in_graph.and_then(|f| f.override_label.clone());
        let override_from_graph_id = match r#override {
            Some(r#override) if !r#override.is_empty() => self.graph_name_to_id(r#override),
            _ => None,
        };

        let requires = match requires {
            Some(requires) if !requires.is_empty() => Some(self.selection_resolver.resolve(&head_type_name, requires)?),
            _ => None,
        };
        let provides = match provides {
            Some(provides) if !provides.is_empty() => {
                Some(self.selection_resolver.resolve(&output_type_name, provides)?)
            }
            _ => None,
        };
        let r#override = match override_from_graph_id {
            Some(from_graph_id) if !from_graph_id.is_empty() => Some(FieldMoveOverride {
                label: override_label,
                value: true,
                from_graph_id: Some(from_graph_id),
            }),
            _ => None,
        };
        let required_contexts = self.required_contexts_of_field(head, FieldStateRef::Object(field));

        self.add_edge(Edge::new(
            head,
            Move::Field(FieldMove::new(
                head_type_name,
                field.name.clone(),
                requires,
                provides,
                r#override,
                false,
                required_contexts,
            )),
            tail,
        ));

        Ok(())
    }

    /// Throws `Node for X already exists in subgraph Y`.
    fn create_type_node(&mut self, type_name: &str, type_state: Option<NodeTypeState<'s>>) -> JsResult<NodeId> {
        if self.type_name_to_node_indexes.contains_key(type_name) {
            return Err(JsError::error(format!(
                "Node for {type_name} already exists in subgraph {}",
                self.id_str
            )));
        }

        let graph_id = self.id_str.clone();
        let graph_name = self.name.clone();
        let graph_name_id = self.name_id;
        Ok(self.create_node(type_name, type_state, &graph_id, &graph_name, graph_name_id))
    }

    fn create_node(
        &mut self,
        type_name: &str,
        type_state: Option<NodeTypeState<'s>>,
        graph_id: &str,
        graph_name: &str,
        graph_name_id: GraphNameId,
    ) -> NodeId {
        let index = self.nodes.len();
        let contexts = self
            .contexts
            .as_ref()
            .and_then(|c| c.by_type_name.get(type_name).cloned())
            .unwrap_or_default();
        self.nodes.push(Node::new(
            NodeId(index),
            type_name.to_string(),
            type_state,
            graph_id.to_string(),
            graph_name.to_string(),
            graph_name_id,
            contexts,
        ));
        self.edges_by_head_type_index.push(Vec::new());
        self.edges_by_tail_type_index.push(Vec::new());
        self.type_children.push(IndexSet::default());

        self.type_name_to_node_indexes
            .entry(type_name.to_string())
            .or_default()
            .push(index);

        NodeId(index)
    }

    /// `addNode(node)`: appends a node from another graph, giving it a new index.
    fn add_node(&mut self, mut node: Node<'s>) -> NodeId {
        let new_index = self.nodes.len();
        node.index = NodeId(new_index);

        let type_name = node.type_name.clone();
        self.nodes.push(node);
        self.edges_by_head_type_index.push(Vec::new());
        self.edges_by_tail_type_index.push(Vec::new());
        self.type_children.push(IndexSet::default());

        self.type_name_to_node_indexes
            .entry(type_name)
            .or_default()
            .push(new_index);

        NodeId(new_index)
    }

    /// Soft delete: removing an edge from every index is not worth it.
    fn ignore_edge(&mut self, edge: EdgeId) {
        self.edges[edge.0].set_ignored(true);
    }

    /// Stores an edge in the arena without indexing it (the `new Edge(..)` passed to
    /// `replaceEdgeAt`).
    fn push_edge(&mut self, edge: Edge<'s>) -> EdgeId {
        let id = EdgeId(self.edges.len());
        self.edges.push(edge);
        id
    }

    fn add_edge(&mut self, edge: Edge<'s>) -> EdgeId {
        let head = edge.head;
        let tail = edge.tail;
        let edge_id = EdgeId(self.edges.len());

        let edge_index = self.edges_by_head_type_index[head.0].len();
        self.edges_by_head_type_index[head.0].push(edge_id);
        self.edges_by_tail_type_index[tail.0].push(edge_id);
        // PERF: inserting an existing name keeps its position, so copy the name only when new.
        if !self.type_children[head.0].contains(&self.nodes[tail.0].type_name) {
            let tail_type_name = self.nodes[tail.0].type_name.clone();
            self.type_children[head.0].insert(tail_type_name);
        }

        // PERF: the head's type name is copied (to index the edge under it) only for the edges
        // indexed by type name; field edges are indexed by field name.
        let is_cross_graph_edge = edge.is_cross_graph_edge(&self.nodes[head.0].graph_id, &self.nodes[tail.0].graph_id);

        match &edge.r#move {
            Move::Field(field_move) => {
                if let Some(r#override) = &field_move.r#override {
                    if is_truthy_str(&r#override.label) {
                        self.field_edges_with_progressive_override.push(edge_id);
                    } else {
                        self.field_edges_with_override.push(edge_id);
                    }
                }

                self.nodes[head.0].add_field_edge(&field_move.field_name, edge_index);
            }
            Move::Entity(_) => {
                let head_type_name = self.nodes[head.0].type_name.clone();
                self.nodes[head.0].add_entity_edge(&head_type_name, edge_index)
            }
            Move::Abstract(_) => {
                let head_type_name = self.nodes[head.0].type_name.clone();
                self.nodes[head.0].add_abstract_edge(&head_type_name, edge_index)
            }
        }

        if is_cross_graph_edge {
            let head_type_name = self.nodes[head.0].type_name.clone();
            self.nodes[head.0].add_cross_graph_edge(&head_type_name, edge_index);
        }

        self.edges.push(edge);
        edge_id
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_uri_component_matches_js() {
        assert_eq!(
            encode_uri_component("digraph G {\n \"a\" -> b;}"),
            "digraph%20G%20%7B%0A%20%22a%22%20-%3E%20b%3B%7D"
        );
        assert_eq!(encode_uri_component("A-z_0.9!~*'()"), "A-z_0.9!~*'()");
        assert_eq!(encode_uri_component("🔑"), "%F0%9F%94%91");
    }
}
