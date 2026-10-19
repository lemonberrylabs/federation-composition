//! Port of `src/supergraph/validation/rules/satisfiablity/node.ts`.
//!
//! Nodes live in their `Graph`'s arena; [`NodeId`] is the TS `node.index` (the position in
//! `nodesByTypeIndex`, reassigned when a subgraph graph is merged with `addSubgraph`).
//! Fields mutated while walking (after the graph is built) use interior mutability.

use std::cell::RefCell;

use crate::utils::collections::IndexMap;

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
    /// Contexts (`@context(name:)`) set by this type, including the ones inherited from
    /// implemented interfaces and from unions this type is a member of.
    pub contexts: ContextMask,
    pub debug_post_fix: String,
    pub is_leaf: bool,
    /// `childrenIndex`: `field__X` / `entity__X` / `abstract__X` / `cross-graph__X` to edge
    /// positions in the head's edge list.
    children_index: IndexMap<String, Vec<usize>>,
    /// When a Node is visited, the names of the graphs that lead to it with @provides.
    visited_graph_combos: RefCell<Vec<Vec<String>>>,
}

impl<'s> Node<'s> {
    /// `new Node(index, typeName, typeState, graphId, graphName, contexts)`.
    pub fn new(
        index: NodeId,
        type_name: String,
        type_state: Option<NodeTypeState<'s>>,
        graph_id: String,
        graph_name: String,
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
            contexts,
            debug_post_fix: String::new(),
            is_leaf,
            children_index: IndexMap::default(),
            visited_graph_combos: RefCell::new(Vec::new()),
        }
    }

    /// `withoutState()`: clears the children index and visited combos (used by `addSubgraph`).
    pub fn without_state(mut self) -> Self {
        self.children_index = IndexMap::default();
        self.visited_graph_combos = RefCell::new(Vec::new());
        self
    }

    pub fn add_field_edge(&mut self, field_name: &str, edge_at: usize) {
        self.push_to_children_index(format!("field__{field_name}"), edge_at);
    }

    pub fn get_field_edge_indexes(&self, field_name: &str) -> Option<&Vec<usize>> {
        self.children_index.get(&format!("field__{field_name}"))
    }

    pub fn add_entity_edge(&mut self, type_name: &str, edge_at: usize) {
        self.push_to_children_index(format!("entity__{type_name}"), edge_at);
    }

    pub fn get_entity_edge_indexes(&self, type_name: &str) -> Option<&Vec<usize>> {
        self.children_index.get(&format!("entity__{type_name}"))
    }

    pub fn add_abstract_edge(&mut self, type_name: &str, edge_at: usize) {
        self.push_to_children_index(format!("abstract__{type_name}"), edge_at);
    }

    pub fn get_abstract_edge_indexes(&self, type_name: &str) -> Option<&Vec<usize>> {
        self.children_index.get(&format!("abstract__{type_name}"))
    }

    pub fn add_cross_graph_edge(&mut self, type_name: &str, edge_at: usize) {
        self.push_to_children_index(format!("cross-graph__{type_name}"), edge_at);
    }

    pub fn get_cross_graph_edge_indexes(&self, type_name: &str) -> Option<&Vec<usize>> {
        self.children_index.get(&format!("cross-graph__{type_name}"))
    }

    fn push_to_children_index(&mut self, id: String, edge_at: usize) {
        match self.children_index.get_mut(&id) {
            Some(indexes) => {
                if !indexes.contains(&edge_at) {
                    indexes.push(edge_at);
                }
            }
            None => {
                self.children_index.insert(id, vec![edge_at]);
            }
        }
    }

    /// Whether a combination of graph names and @provides that lead to the Node was already
    /// checked. (`labelValues` is unused, as in the TS.)
    pub fn is_graph_combo_visited(&self, graph_name_provides_combos: &[String], _label_values: &OverrideLabels) -> bool {
        self.visited_graph_combos
            .borrow()
            .iter()
            .any(|visited_graphs| visited_graphs.iter().all(|g| graph_name_provides_combos.contains(g)))
    }

    pub fn set_graph_combo_as_visited(&self, graph_names: Vec<String>) {
        self.visited_graph_combos.borrow_mut().push(graph_names);
    }

    /// `toString()`: `${typeName}/${graphName}${debugPostFix}` (lazy in TS; logging only).
    pub fn to_string(&self) -> String {
        format!("{}/{}{}", self.type_name, self.graph_name, self.debug_post_fix)
    }
}
