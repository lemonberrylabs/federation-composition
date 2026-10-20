//! Port of `src/supergraph/validation/rules/satisfiablity/supergraph.ts`.

use std::rc::Rc;

use graphql_rs::OperationType;

use super::constants::GraphId;
use super::contexts::Contexts;
use super::graph::{Graph, TypesBySubgraph};
use super::move_validator::MoveValidator;
use super::node::GraphNames;
use super::operation_path::Step;
use super::selection::SelectionResolver;
use super::walker::{WalkMethod, WalkTracker, Walker};
use crate::supergraph::state::SupergraphState;
use crate::utils::js::JsResult;
use crate::utils::logger::{Logger, LoggerContext};

/// `Supergraph`: the supergraph graph, the merged graph (all subgraph graphs joined) and the
/// move validator over the merged graph.
pub struct Supergraph<'s> {
    supergraph: Rc<Graph<'s>>,
    merged_graph: Rc<Graph<'s>>,
    move_requirement_checker: MoveValidator<'s>,
    logger: Logger,
}

impl<'s> Supergraph<'s> {
    /// Builds every graph (throws on inconsistent state).
    pub fn new(supergraph_state: &'s SupergraphState) -> JsResult<Self> {
        let logger = Logger::new("Supergraph", &mut LoggerContext::new());
        let selection_resolver = Rc::new(SelectionResolver::new(supergraph_state));
        // Contexts are only ever resolved on the merged graph,
        // the supergraph graph does not need to know about them.
        let contexts = if supergraph_state.specs.context {
            Some(Rc::new(Contexts::new(supergraph_state)))
        } else {
            None
        };
        let graph_names = Rc::new(GraphNames::default());
        let mut supergraph = Graph::new(
            &logger,
            GraphId::Supergraph,
            "supergraph".to_string(),
            supergraph_state,
            selection_resolver.clone(),
            true,
            None,
            graph_names.clone(),
        );
        let mut merged_graph = Graph::new(
            &logger,
            GraphId::MergedGraph,
            "merged".to_string(),
            supergraph_state,
            selection_resolver.clone(),
            false,
            contexts.clone(),
            graph_names.clone(),
        );
        let types_by_subgraph = TypesBySubgraph::new(supergraph_state);
        for (id, subgraph_state) in &supergraph_state.subgraphs {
            let mut graph = Graph::new(
                &logger,
                GraphId::Subgraph(id.clone()),
                subgraph_state.graph.name.clone(),
                supergraph_state,
                selection_resolver.clone(),
                false,
                contexts.clone(),
                graph_names.clone(),
            );
            let types = types_by_subgraph.of(id);
            graph
                .add_from_roots()?
                .add_from_entities(types)?
                .add_unreachable_types(types)?;
            merged_graph.add_subgraph(graph);
        }

        merged_graph.join_subgraphs()?;
        merged_graph.add_overridden_fields()?;
        // PERF: the merged graph is complete.
        merged_graph.enable_edge_caches();

        supergraph
            .add_from_roots()?
            .add_overridden_fields()?
            .add_interface_object_fields()?;

        let merged_graph = Rc::new(merged_graph);
        let move_requirement_checker = MoveValidator::new(&logger, merged_graph.clone());

        Ok(Supergraph {
            supergraph: Rc::new(supergraph),
            merged_graph,
            move_requirement_checker,
            logger,
        })
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
        Walker::new(
            &self.logger,
            self.move_requirement_checker.clone(),
            self.supergraph.clone(),
            self.merged_graph.clone(),
        )
        .walk(WalkMethod::Dfs)
    }

    pub fn validate_operation(&self, operation: OperationType, steps: &[Step]) -> JsResult<WalkTracker<'s>> {
        Walker::new(
            &self.logger,
            self.move_requirement_checker.clone(),
            self.supergraph.clone(),
            self.merged_graph.clone(),
        )
        .walk_trail(operation, steps)
    }
}
