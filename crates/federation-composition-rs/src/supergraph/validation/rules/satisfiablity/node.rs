//! Port of `src/supergraph/validation/rules/satisfiablity/node.ts`.
//!
//! Nodes live in their `Graph`'s arena; [`NodeId`] is the TS `node.index` (the position in
//! `nodesByTypeIndex`, reassigned when a subgraph graph is merged with `addSubgraph`).
//! Fields mutated while walking (after the graph is built) use interior mutability.

use std::cell::RefCell;

use super::graph::Graph;

use crate::utils::collections::{IndexMap, IndexSet};

use super::contexts::ContextMask;
use super::helpers::OverrideLabels;
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::object_type::ObjectTypeState;
use crate::supergraph::composition::scalar_type::ScalarTypeState;
use crate::supergraph::composition::union_type::UnionTypeState;

/// Index of a node in its graph (`node.index`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub usize);

/// PERF: an interned graph name. The TS compares graph names as strings (`visitedGraphs`,
/// `resolvedGraphs`, the resolvability cache); comparing interned ids is equivalent because
/// [`GraphNames`] gives equal names the same id and different names different ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphNameId(u32);

/// PERF: a list of graph names that is only pushed to and searched (`resolvedGraphs` of
/// `findIndirectPaths`), as a bit set over the interned ids: it can hold every subgraph, and
/// `includes` on an array is linear.
#[derive(Debug, Default)]
pub struct GraphNameSet {
    bits: Vec<u64>,
}

impl GraphNameSet {
    pub fn contains(&self, id: &GraphNameId) -> bool {
        let i = id.0 as usize;
        self.bits.get(i / 64).is_some_and(|word| word & (1 << (i % 64)) != 0)
    }

    pub fn push(&mut self, id: GraphNameId) {
        let i = id.0 as usize;
        if self.bits.len() <= i / 64 {
            self.bits.resize(i / 64 + 1, 0);
        }
        self.bits[i / 64] |= 1 << (i % 64);
    }
}

/// PERF: a string interner. Equal strings get equal ids, different strings different ids.
#[derive(Debug, Default)]
pub struct Names {
    names: RefCell<IndexSet<String>>,
}

/// An id from [`Names`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NameId(u32);

impl Names {
    pub fn intern(&self, name: &str) -> NameId {
        if let Some(index) = self.names.borrow().get_index_of(name) {
            return NameId(index as u32);
        }
        let (index, _) = self.names.borrow_mut().insert_full(name.to_string());
        NameId(index as u32)
    }

    pub fn get(&self, id: NameId) -> String {
        self.names.borrow()[id.0 as usize].clone()
    }
}

/// The graph name interner shared by the graphs of one `Supergraph`.
#[derive(Debug, Default)]
pub struct GraphNames {
    names: Names,
}

impl GraphNames {
    pub fn intern(&self, name: &str) -> GraphNameId {
        GraphNameId(self.names.intern(name).0)
    }

    pub fn name(&self, id: GraphNameId) -> String {
        self.names.get(NameId(id.0))
    }

    /// `graphNames.join(separator)` (logging only).
    pub fn join(&self, ids: &[GraphNameId], separator: &str) -> String {
        ids.iter().map(|&id| self.name(id)).collect::<Vec<_>>().join(separator)
    }
}

/// PERF: an interned "graph name + @provides + contexts" key of `Walker::next_step` (the TS
/// builds a string per path). Equal strings get equal ids, so comparing ids is comparing the
/// strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphComboId(u32);

/// PERF: interns the strings `nextStep` builds for each path (`${graphName}#provides${contextsKey}`
/// or `${graphName}${contextsKey}`). The string of a `(graph name, @provides, contexts)` triple
/// is formatted once, and triples whose strings are equal share an id.
/// Held by the graph whose nodes record the visited combos (the supergraph graph), so ids stay
/// valid across walks.
#[derive(Debug, Default)]
pub struct GraphComboIds {
    by_triple: RefCell<IndexMap<(GraphNameId, bool, ContextMask), GraphComboId>>,
    by_string: RefCell<IndexSet<String>>,
}

impl GraphComboIds {
    pub fn id(
        &self,
        merged_graph: &Graph<'_>,
        tail: NodeId,
        has_provides: bool,
        contexts: &ContextMask,
    ) -> GraphComboId {
        let tail_node = merged_graph.node(tail);
        let triple = (tail_node.graph_name_id, has_provides, contexts.clone());
        if let Some(&id) = self.by_triple.borrow().get(&triple) {
            return id;
        }

        let tail_graph_name = &tail_node.graph_name;
        // The context key is empty when the supergraph has no `@context`.
        let contexts_key = if contexts.is_zero() {
            String::new()
        } else {
            format!("#contexts:{contexts}")
        };
        let key = if has_provides {
            format!("{tail_graph_name}#provides{contexts_key}")
        } else {
            format!("{tail_graph_name}{contexts_key}")
        };
        let (index, _) = self.by_string.borrow_mut().insert_full(key);
        let id = GraphComboId(index as u32);
        self.by_triple.borrow_mut().insert(triple, id);
        id
    }
}

/// `typeState` of a node (non-null): a reference into the supergraph state.
#[derive(Clone, Copy, Debug)]
pub enum NodeTypeState<'s> {
    Object(&'s ObjectTypeState),
    Interface(&'s InterfaceTypeState),
    Enum(&'s EnumTypeState),
    Scalar(&'s ScalarTypeState),
    Union(&'s UnionTypeState),
}

impl NodeTypeState<'_> {
    /// `typeState.kind`.
    pub fn kind(&self) -> &'static str {
        match self {
            NodeTypeState::Object(_) => "object",
            NodeTypeState::Interface(_) => "interface",
            NodeTypeState::Enum(_) => "enum",
            NodeTypeState::Scalar(_) => "scalar",
            NodeTypeState::Union(_) => "union",
        }
    }
}

/// `Node`.
#[derive(Debug)]
pub struct Node<'s> {
    pub index: NodeId,
    pub type_name: String,
    /// `null` for specified scalars.
    pub type_state: Option<NodeTypeState<'s>>,
    pub graph_id: String,
    pub graph_name: String,
    /// PERF: `graph_name` interned, for the comparisons made while walking.
    pub graph_name_id: GraphNameId,
    /// Contexts (`@context(name:)`) set by this type, including the ones inherited from
    /// implemented interfaces and from unions this type is a member of.
    pub contexts: ContextMask,
    pub debug_post_fix: String,
    pub is_leaf: bool,
    /// `childrenIndex`: `field__X` / `entity__X` / `abstract__X` / `cross-graph__X` to edge
    /// positions in the head's edge list.
    /// PERF: one map per key prefix, keyed by `X`, so lookups take a `&str` instead of
    /// formatting a key. Keys of different prefixes never collide (the prefixes differ in their
    /// first character), and the map is only read by key, never iterated, so splitting it is
    /// not observable.
    field_children_index: IndexMap<String, Vec<usize>>,
    entity_children_index: IndexMap<String, Vec<usize>>,
    abstract_children_index: IndexMap<String, Vec<usize>>,
    cross_graph_children_index: IndexMap<String, Vec<usize>>,
    /// When a Node is visited, the names of the graphs that lead to it with @provides.
    visited_graph_combos: RefCell<Vec<Vec<GraphComboId>>>,
}

impl<'s> Node<'s> {
    /// `new Node(index, typeName, typeState, graphId, graphName, contexts)`.
    pub fn new(
        index: NodeId,
        type_name: String,
        type_state: Option<NodeTypeState<'s>>,
        graph_id: String,
        graph_name: String,
        graph_name_id: GraphNameId,
        contexts: ContextMask,
    ) -> Self {
        // NOTE(port): the TS throws when `typeState` is `undefined`; `Option` cannot express that
        // state (callers pass a state or `null`).
        let is_leaf = matches!(
            type_state,
            None | Some(NodeTypeState::Scalar(_)) | Some(NodeTypeState::Enum(_))
        );

        Node {
            index,
            type_name,
            type_state,
            graph_id,
            graph_name,
            graph_name_id,
            contexts,
            debug_post_fix: String::new(),
            is_leaf,
            field_children_index: IndexMap::default(),
            entity_children_index: IndexMap::default(),
            abstract_children_index: IndexMap::default(),
            cross_graph_children_index: IndexMap::default(),
            visited_graph_combos: RefCell::new(Vec::new()),
        }
    }

    /// `withoutState()`: clears the children index and visited combos (used by `addSubgraph`).
    pub fn without_state(mut self) -> Self {
        self.field_children_index = IndexMap::default();
        self.entity_children_index = IndexMap::default();
        self.abstract_children_index = IndexMap::default();
        self.cross_graph_children_index = IndexMap::default();
        self.visited_graph_combos = RefCell::new(Vec::new());
        self
    }

    pub fn add_field_edge(&mut self, field_name: &str, edge_at: usize) {
        Self::push_to_children_index(&mut self.field_children_index, field_name, edge_at);
    }

    pub fn get_field_edge_indexes(&self, field_name: &str) -> Option<&Vec<usize>> {
        self.field_children_index.get(field_name)
    }

    pub fn add_entity_edge(&mut self, type_name: &str, edge_at: usize) {
        Self::push_to_children_index(&mut self.entity_children_index, type_name, edge_at);
    }

    pub fn get_entity_edge_indexes(&self, type_name: &str) -> Option<&Vec<usize>> {
        self.entity_children_index.get(type_name)
    }

    pub fn add_abstract_edge(&mut self, type_name: &str, edge_at: usize) {
        Self::push_to_children_index(&mut self.abstract_children_index, type_name, edge_at);
    }

    pub fn get_abstract_edge_indexes(&self, type_name: &str) -> Option<&Vec<usize>> {
        self.abstract_children_index.get(type_name)
    }

    pub fn add_cross_graph_edge(&mut self, type_name: &str, edge_at: usize) {
        Self::push_to_children_index(&mut self.cross_graph_children_index, type_name, edge_at);
    }

    pub fn get_cross_graph_edge_indexes(&self, type_name: &str) -> Option<&Vec<usize>> {
        self.cross_graph_children_index.get(type_name)
    }

    fn push_to_children_index(children_index: &mut IndexMap<String, Vec<usize>>, id: &str, edge_at: usize) {
        match children_index.get_mut(id) {
            Some(indexes) => {
                if !indexes.contains(&edge_at) {
                    indexes.push(edge_at);
                }
            }
            None => {
                children_index.insert(id.to_string(), vec![edge_at]);
            }
        }
    }

    /// Whether a combination of graph names and @provides that lead to the Node was already
    /// checked. (`labelValues` is unused, as in the TS.)
    pub fn is_graph_combo_visited(
        &self,
        graph_name_provides_combos: &[GraphComboId],
        _label_values: &OverrideLabels,
    ) -> bool {
        self.visited_graph_combos
            .borrow()
            .iter()
            .any(|visited_graphs| visited_graphs.iter().all(|g| graph_name_provides_combos.contains(g)))
    }

    pub fn set_graph_combo_as_visited(&self, graph_names: Vec<GraphComboId>) {
        self.visited_graph_combos.borrow_mut().push(graph_names);
    }

    /// `toString()`: `${typeName}/${graphName}${debugPostFix}` (lazy in TS; logging only).
    pub fn to_string(&self) -> String {
        format!("{}/{}{}", self.type_name, self.graph_name, self.debug_post_fix)
    }
}
