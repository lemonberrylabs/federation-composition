//! Port of `src/supergraph/validation/rules/satisfiablity/walker.ts`.

use std::rc::Rc;

use graphql_js::OperationType;
use indexmap::IndexMap;

use super::contexts::ContextMask;
use super::edge::EdgeId;
use super::errors::{LazyErrors, SatisfiabilityError};
use super::finder::PathFinder;
use super::graph::Graph;
use super::helpers::OverrideLabels;
use super::move_validator::MoveValidator;
use super::node::NodeId;
use super::operation_path::{OperationPath, Step};
use crate::utils::js::JsResult;
use crate::utils::logger::Logger;

/// `WalkTracker`. `super_path` is a path of the supergraph graph, `paths` of the merged graph.
#[derive(Clone, Debug)]
pub struct WalkTracker<'s> {
    errors: LazyErrors<'s, SatisfiabilityError>,
    pub super_path: OperationPath,
    pub paths: Vec<OperationPath>,
    pub label_values: OverrideLabels,
}

impl<'s> WalkTracker<'s> {
    pub fn new(super_path: OperationPath, paths: Vec<OperationPath>, label_values: OverrideLabels) -> Self {
        WalkTracker {
            errors: LazyErrors::new(),
            super_path,
            paths,
            label_values,
        }
    }

    /// `move(edge)`: `edge` is a supergraph-graph edge. Throws for an entity edge.
    pub fn r#move(&self, supergraph: &Graph<'s>, edge: EdgeId) -> JsResult<WalkTracker<'s>> {
        todo!()
    }

    pub fn add_path(&mut self, path: OperationPath) {
        self.paths.push(path);
        self.errors = LazyErrors::new();
    }

    pub fn add_error(&mut self, errors: &LazyErrors<'s, SatisfiabilityError>) {
        self.errors.add_all(errors);
    }

    pub fn is_possible(&self) -> bool {
        !self.paths.is_empty()
    }

    pub fn gives_empty_result(&self, supergraph: &Graph<'s>) -> bool {
        todo!()
    }

    pub fn is_edge_visited(&self, edge: EdgeId) -> bool {
        self.super_path.is_visited_edge(edge)
    }

    /// `listErrors()`: deduplicated by message.
    pub fn list_errors(&self, supergraph: &Graph<'s>) -> Vec<SatisfiabilityError> {
        todo!()
    }
}

/// `IsEdgeIgnored = (edge: Edge) => boolean` (edges of the supergraph graph).
pub type IsEdgeIgnored<'f> = &'f dyn Fn(EdgeId) -> bool;

/// `defaultIsEdgeIgnored`.
pub fn default_is_edge_ignored(_edge: EdgeId) -> bool {
    false
}

/// `ShortestPaths`: shortest path to a node, per set of contexts available on the way to it.
pub type ShortestPaths = IndexMap<NodeId, IndexMap<ContextMask, OperationPath>>;

/// `walk(method)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkMethod {
    Bfs,
    Dfs,
}

/// `Walker`.
pub struct Walker<'s> {
    logger: Logger,
    path_finder: PathFinder<'s>,
    move_checker: MoveValidator<'s>,
    supergraph: Rc<Graph<'s>>,
    merged_graph: Rc<Graph<'s>>,
}

impl<'s> Walker<'s> {
    pub fn new(logger: &Logger, move_checker: MoveValidator<'s>, supergraph: Rc<Graph<'s>>, merged_graph: Rc<Graph<'s>>) -> Self {
        let logger = logger.create("Walker");
        let path_finder = PathFinder::new(logger.clone(), merged_graph.clone(), move_checker.clone());
        Walker {
            logger,
            path_finder,
            move_checker,
            supergraph,
            merged_graph,
        }
    }

    /// `walkTrail(operationType, steps)`: walks only the given steps.
    pub fn walk_trail(&self, operation_type: OperationType, steps: &[Step]) -> JsResult<WalkTracker<'s>> {
        todo!()
    }

    /// `walk(method = "bfs")`: the unreachable paths.
    pub fn walk(&self, method: WalkMethod) -> JsResult<Vec<WalkTracker<'s>>> {
        match method {
            WalkMethod::Dfs => self.dfs(),
            WalkMethod::Bfs => self.bfs(),
        }
    }

    /// `nextStep(state, superTail, next, isEdgeIgnored)`. `next` may throw.
    fn next_step(
        &self,
        state: &WalkTracker<'s>,
        super_tail: NodeId,
        next: &mut dyn FnMut(WalkTracker<'s>, EdgeId) -> JsResult<()>,
        is_edge_ignored: IsEdgeIgnored<'_>,
    ) -> JsResult<()> {
        todo!()
    }

    fn dfs(&self) -> JsResult<Vec<WalkTracker<'s>>> {
        todo!()
    }

    fn dfs_inner(&self, super_tail: NodeId, state: WalkTracker<'s>, unreachable: &mut Vec<WalkTracker<'s>>) -> JsResult<()> {
        todo!()
    }

    fn bfs(&self) -> JsResult<Vec<WalkTracker<'s>>> {
        todo!()
    }
}

/// `setShortestPath(shortestPathPerTail, paths)`. Paths reaching the same node are
/// interchangeable, so only the shortest is kept, per contexts set by their ancestors.
fn set_shortest_path(merged_graph: &Graph<'_>, shortest_path_per_tail: &mut ShortestPaths, paths: &[OperationPath]) {
    todo!()
}
