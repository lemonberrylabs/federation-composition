//! Port of `src/supergraph/validation/rules/satisfiablity/operation-path.ts`.
//!
//! A path holds node/edge ids of one graph (the supergraph graph for a walker's `superPath`,
//! the merged graph otherwise); methods that need the edges take that graph.

use super::contexts::ContextMask;
use super::edge::EdgeId;
use super::graph::Graph;
use super::node::NodeId;

/// `Step = FieldStep | AbstractStep`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// `FieldStep`: `{ fieldName, typeName }`.
    Field { field_name: String, type_name: String },
    /// `AbstractStep`: `{ typeName }`.
    Abstract { type_name: String },
}

/// `@override(label:)` cannot start with a space, so a space marks an empty label.
pub const EMPTY_OVERRIDE_LABEL: &str = " ";

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
        todo!()
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
        todo!()
    }

    pub fn tail(&self, graph: &Graph<'_>) -> Option<NodeId> {
        todo!()
    }

    pub fn root_node(&self) -> NodeId {
        self.root_node
    }

    pub fn is_visited_edge(&self, edge: EdgeId) -> bool {
        self.previous_edges.contains(&edge)
    }

    /// `toString()` (lazy in TS; logging only).
    pub fn to_string(&self, graph: &Graph<'_>) -> String {
        todo!()
    }
}
