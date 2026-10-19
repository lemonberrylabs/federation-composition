//! Port of `src/supergraph/validation/rules/satisfiablity/finder.ts`.

use std::rc::Rc;

use super::edge::EdgeId;
use super::errors::{LazyErrors, SatisfiabilityError};
use super::graph::Graph;
use super::helpers::OverrideLabels;
use super::move_validator::MoveValidator;
use super::operation_path::OperationPath;
use super::selection::Selection;
use crate::utils::js::JsResult;
use crate::utils::logger::Logger;

/// `concatIfNotExistsString(list, item)`.
pub fn concat_if_not_exists_string(list: &[String], item: &str) -> Vec<String> {
    let mut out = list.to_vec();
    if !list.iter().any(|x| x == item) {
        out.push(item.to_string());
    }
    out
}

/// `concatIfNotExistsFields(list, item)`.
pub fn concat_if_not_exists_fields(list: &[Rc<Selection>], item: &Rc<Selection>) -> Vec<Rc<Selection>> {
    let mut out = list.to_vec();
    if !list.iter().any(|f| f.equals(item)) {
        out.push(item.clone());
    }
    out
}

/// `PathFinderResult`.
#[derive(Clone, Debug)]
pub enum PathFinderResult<'s> {
    Success { paths: Vec<OperationPath> },
    Failure { errors: LazyErrors<'s, SatisfiabilityError> },
}

/// An entry of the `queue` of `findIndirectPaths`: `[visitedGraphs, visitedFields, path]`.
pub type IndirectPathsQueueItem = (Vec<String>, Vec<Rc<Selection>>, OperationPath);

/// `PathFinder`. Stateless apart from its references, so `MoveValidator` and `Walker` create
/// one when needed (the TS `MoveValidator` owns one that points back at it).
/// The lazy errors it creates may capture `Rc` clones of the graph and a clone of the move
/// validator (see `findDirectPaths`).
#[derive(Clone)]
pub struct PathFinder<'s> {
    logger: Logger,
    graph: Rc<Graph<'s>>,
    move_validator: MoveValidator<'s>,
}

impl<'s> PathFinder<'s> {
    pub fn new(logger: Logger, graph: Rc<Graph<'s>>, move_validator: MoveValidator<'s>) -> Self {
        PathFinder {
            logger,
            graph,
            move_validator,
        }
    }

    pub fn find_direct_paths(
        &self,
        path: &OperationPath,
        type_name: &str,
        field_name: Option<&str>,
        visited_edges: &[EdgeId],
        label_values: &OverrideLabels,
    ) -> JsResult<PathFinderResult<'s>> {
        todo!()
    }

    #[allow(clippy::too_many_arguments)]
    fn find_field_indirectly(
        &self,
        path: &OperationPath,
        type_name: &str,
        field_name: &str,
        visited_edges: &[EdgeId],
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
        errors: &mut LazyErrors<'s, SatisfiabilityError>,
        final_paths: &mut Vec<OperationPath>,
        queue: &mut Vec<IndirectPathsQueueItem>,
        resolved_graphs: &mut Vec<String>,
        edge: EdgeId,
    ) -> JsResult<()> {
        todo!()
    }

    #[allow(clippy::too_many_arguments)]
    fn find_type_indirectly(
        &self,
        path: &OperationPath,
        type_name: &str,
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
        final_paths: &mut Vec<OperationPath>,
        queue: &mut Vec<IndirectPathsQueueItem>,
        resolved_graphs: &mut Vec<String>,
        edge: EdgeId,
    ) {
        todo!()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn find_indirect_paths(
        &self,
        path: &OperationPath,
        type_name: &str,
        field_name: Option<&str>,
        visited_edges: &[EdgeId],
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<PathFinderResult<'s>> {
        todo!()
    }
}
