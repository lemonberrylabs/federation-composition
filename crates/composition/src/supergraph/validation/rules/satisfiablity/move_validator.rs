//! Port of `src/supergraph/validation/rules/satisfiablity/move-validator.ts`.

use std::rc::Rc;

use super::edge::{EdgeId, EdgeResolvabilityResult};
use super::errors::{LazyErrors, SatisfiabilityError};
use super::finder::PathFinder;
use super::graph::Graph;
use super::helpers::OverrideLabels;
use super::operation_path::OperationPath;
use super::selection::{Field, Fragment, Selection, SelectionNode};
use crate::utils::js::JsResult;
use crate::utils::logger::Logger;

/// `MoveRequirement<T = SelectionNode>`.
#[derive(Clone, Debug)]
pub struct MoveRequirement {
    pub paths: Vec<OperationPath>,
    pub selection: SelectionNode,
}

fn is_fragment_requirement(requirement: &MoveRequirement) -> bool {
    matches!(requirement.selection, SelectionNode::Fragment(_))
}

fn is_field_requirement(requirement: &MoveRequirement) -> bool {
    matches!(requirement.selection, SelectionNode::Field(_))
}

/// `RequirementResult`.
#[derive(Clone, Debug)]
pub enum RequirementResult<'s> {
    Success { requirements: Vec<MoveRequirement> },
    Failure { errors: LazyErrors<'s, SatisfiabilityError> },
}

/// The result of `canResolveSelectionSet`: `{ success: true }` or the failing requirement's
/// result.
pub type CanResolveSelectionSetResult<'s> = RequirementResult<'s>;

/// `MoveValidator`. Holds the merged graph (named `supergraph` in TS). Cheap to clone; its
/// `PathFinder` is created on demand (`path_finder()`), since the TS one points back at it.
#[derive(Clone)]
pub struct MoveValidator<'s> {
    logger: Logger,
    supergraph: Rc<Graph<'s>>,
}

impl<'s> MoveValidator<'s> {
    pub fn new(logger: &Logger, supergraph: Rc<Graph<'s>>) -> Self {
        MoveValidator {
            logger: logger.create("MoveValidator"),
            supergraph,
        }
    }

    /// `this.pathFinder`.
    fn path_finder(&self) -> PathFinder<'s> {
        PathFinder::new(self.logger.clone(), self.supergraph.clone(), self.clone())
    }

    #[allow(clippy::too_many_arguments)]
    fn can_resolve_selection_set(
        &self,
        selection_set: &[SelectionNode],
        path: &OperationPath,
        visited_edges: &[EdgeId],
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<CanResolveSelectionSetResult<'s>> {
        todo!()
    }

    /// `requirement.selection` is a `Fragment`.
    fn validate_fragment_requirement(
        &self,
        requirement: &MoveRequirement,
        visited_edges: &[EdgeId],
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<RequirementResult<'s>> {
        todo!()
    }

    /// `requirement.selection` is a `Field`.
    fn validate_field_requirement(
        &self,
        requirement: &MoveRequirement,
        visited_edges: &[EdgeId],
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<RequirementResult<'s>> {
        todo!()
    }

    fn validate_requirement(
        &self,
        requirement: &MoveRequirement,
        visited_edges: &[EdgeId],
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<RequirementResult<'s>> {
        todo!()
    }

    pub fn is_external(&self, edge: EdgeId) -> bool {
        todo!()
    }

    /// `edge` is a field edge.
    fn can_access_field_with_override(&self, edge: EdgeId, override_labels: &OverrideLabels) -> bool {
        todo!()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn is_edge_resolvable(
        &self,
        edge: EdgeId,
        path: &OperationPath,
        visited_edges: &[EdgeId],
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<EdgeResolvabilityResult<'s>> {
        todo!()
    }
}
