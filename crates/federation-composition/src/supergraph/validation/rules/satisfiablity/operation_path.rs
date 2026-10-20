//! Port of `src/supergraph/validation/rules/satisfiablity/operation-path.ts`.
//!
//! A path holds node/edge ids of one graph (the supergraph graph for a walker's `superPath`,
//! the merged graph otherwise); methods that need the edges take that graph.

use super::contexts::ContextMask;
use super::edge::{is_field_edge, EdgeId};
use super::graph::Graph;
use super::moves::Move;
use super::node::NodeId;

/// `Step = FieldStep | AbstractStep`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// `FieldStep`: `{ fieldName, typeName }`.
    Field { field_name: String, type_name: String },
    /// `AbstractStep`: `{ typeName }`.
    Abstract { type_name: String },
}

/// `OperationPath`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationPath {
    root_node: NodeId,
    previous_edges: Vec<EdgeId>,
    /// Contexts set by the types we moved out of (the ancestors of the path's tail).
    contexts_in_scope: ContextMask,
}

impl OperationPath {
    pub fn new(root_node: NodeId) -> Self {
        OperationPath {
            root_node,
            previous_edges: Vec::new(),
            contexts_in_scope: ContextMask::zero(),
        }
    }

    /// The `contexts` getter.
    pub fn contexts(&self) -> &ContextMask {
        &self.contexts_in_scope
    }

    /// `move(edge)`: appends the edge (mutating, returns `self` like the TS).
    pub fn r#move(&mut self, graph: &Graph<'_>, edge: EdgeId) -> &mut Self {
        self.previous_edges.push(edge);

        // Only field moves turn the head into an ancestor.
        // Key and abstract moves stay at the same level and preserve the context scope.
        let e = graph.edge(edge);
        if is_field_edge(e) {
            let head = graph.node(e.head);
            if !head.contexts.is_zero() {
                self.contexts_in_scope.or_assign(&head.contexts);
            }
        }
        self
    }

    /// `path.clone().move(edge)`.
    pub fn moved(&self, graph: &Graph<'_>, edge: EdgeId) -> OperationPath {
        // PERF: the copy is sized for the new edge, so `move` does not reallocate it.
        let mut previous_edges = Vec::with_capacity(self.previous_edges.len() + 1);
        previous_edges.extend_from_slice(&self.previous_edges);
        let mut new_path = OperationPath {
            root_node: self.root_node,
            previous_edges,
            contexts_in_scope: self.contexts_in_scope.clone(),
        };
        new_path.r#move(graph, edge);
        new_path
    }

    /// `clone()`.
    pub fn clone_path(&self) -> OperationPath {
        self.clone()
    }

    /// A `@requires` selection is resolved as its own fetch starting from the requiring type,
    /// so it starts with no contexts in scope.
    pub fn without_contexts(&self) -> OperationPath {
        let mut new_path = self.clone();
        new_path.contexts_in_scope = ContextMask::zero();
        new_path
    }

    pub fn has_contexts(&self, required: &ContextMask) -> bool {
        self.contexts_in_scope.contains_all(required)
    }

    pub fn depth(&self) -> usize {
        self.previous_edges.len()
    }

    pub fn edge(&self) -> Option<EdgeId> {
        self.previous_edges.last().copied()
    }

    pub fn steps(&self, graph: &Graph<'_>) -> Vec<Step> {
        self.previous_edges
            .iter()
            .map(|&edge| {
                let e = graph.edge(edge);
                match &e.r#move {
                    Move::Field(m) => Step::Field {
                        type_name: m.type_name.clone(),
                        field_name: m.field_name.clone(),
                    },
                    _ => Step::Abstract {
                        type_name: graph.node(e.tail).type_name.clone(),
                    },
                }
            })
            .collect()
    }

    pub fn tail(&self, graph: &Graph<'_>) -> Option<NodeId> {
        self.edge().map(|edge| graph.edge(edge).tail)
    }

    /// `path.tail() ?? path.rootNode()`.
    pub fn tail_or_root(&self, graph: &Graph<'_>) -> NodeId {
        self.tail(graph).unwrap_or(self.root_node)
    }

    // TS: the `rootNode()` getter; the ported callers do not need it.
    #[allow(dead_code)]
    pub fn root_node(&self) -> NodeId {
        self.root_node
    }

    pub fn is_visited_edge(&self, edge: EdgeId) -> bool {
        self.previous_edges.contains(&edge)
    }

    /// `toString()` (lazy in TS; logging only).
    pub fn to_string(&self, graph: &Graph<'_>) -> String {
        let mut str = graph.node(self.root_node).to_string();
        for &edge in &self.previous_edges {
            let e = graph.edge(edge);
            str += &format!(" -({})-> {}", e.r#move, graph.node(e.tail));
        }
        str
    }
}
