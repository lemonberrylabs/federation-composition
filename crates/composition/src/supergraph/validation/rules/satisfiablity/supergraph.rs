//! Port of `src/supergraph/validation/rules/satisfiablity/supergraph.ts`.

use std::rc::Rc;

use graphql_js::OperationType;

use super::graph::Graph;
use super::move_validator::MoveValidator;
use super::operation_path::Step;
use super::selection::SelectionResolver;
use super::walker::WalkTracker;
use crate::supergraph::state::SupergraphState;
use crate::utils::js::JsResult;
use crate::utils::logger::Logger;

/// `Supergraph`: the supergraph graph, the merged graph (all subgraph graphs joined) and the
/// move validator over the merged graph.
pub struct Supergraph<'s> {
    supergraph: Rc<Graph<'s>>,
    merged_graph: Rc<Graph<'s>>,
    selection_resolver: Rc<SelectionResolver<'s>>,
    move_requirement_checker: MoveValidator<'s>,
    logger: Logger,
}

impl<'s> Supergraph<'s> {
    /// Builds every graph (throws on inconsistent state).
    pub fn new(supergraph_state: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }

    /// The supergraph graph (resolves the edges of `WalkTracker::super_path`).
    pub fn supergraph_graph(&self) -> &Rc<Graph<'s>> {
        &self.supergraph
    }

    /// The merged graph (resolves the edges of `WalkTracker::paths`).
    pub fn merged_graph(&self) -> &Rc<Graph<'s>> {
        &self.merged_graph
    }

    /// `validate()`: a depth-first walk; the unreachable paths.
    pub fn validate(&self) -> JsResult<Vec<WalkTracker<'s>>> {
        todo!()
    }

    pub fn validate_operation(&self, operation: OperationType, steps: &[Step]) -> JsResult<WalkTracker<'s>> {
        todo!()
    }
}
