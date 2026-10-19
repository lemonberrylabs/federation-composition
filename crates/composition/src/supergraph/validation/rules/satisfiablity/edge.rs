//! Port of `src/supergraph/validation/rules/satisfiablity/edge.ts`.
//!
//! Edges live in their `Graph`'s arena and are referred to by [`EdgeId`]; identity checks
//! (`edge === other`, `previousEdges.includes(edge)`) compare ids. `head`/`tail` are node ids
//! of the same graph. The resolvability cache is filled while walking, so it is a `RefCell`.

use std::cell::RefCell;
use std::rc::Rc;

use super::errors::{OneOrMany, SatisfiabilityError};
use super::helpers::{Lazy, OverrideLabels};
use super::moves::{FieldMoveOverride, Move};
use super::node::NodeId;
use crate::utils::js::{JsError, JsResult};

/// Index of an edge in its graph's edge arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub usize);

/// A shared lazy satisfiability error (`Lazy<SatisfiabilityError>`).
pub type LazySatisfiabilityError<'s> = Rc<Lazy<'s, OneOrMany<SatisfiabilityError>>>;

/// `EdgeResolvabilityResult`.
#[derive(Clone, Debug)]
pub enum EdgeResolvabilityResult<'s> {
    Success,
    Failure { error: LazySatisfiabilityError<'s> },
}

impl EdgeResolvabilityResult<'_> {
    pub fn success(&self) -> bool {
        matches!(self, EdgeResolvabilityResult::Success)
    }
}

/// `Edge<T = Move>`.
#[derive(Debug)]
pub struct Edge<'s> {
    pub head: NodeId,
    pub r#move: Move,
    pub tail: NodeId,
    resolvable: RefCell<Vec<(Vec<String>, OverrideLabels, EdgeResolvabilityResult<'s>)>>,
    ignored: bool,
}

/// `isEntityEdge(edge)`.
pub fn is_entity_edge(edge: &Edge<'_>) -> bool {
    matches!(edge.r#move, Move::Entity(_))
}

/// `assertEntityEdge(edge)`.
pub fn assert_entity_edge(edge: &Edge<'_>) -> JsResult<()> {
    todo!()
}

/// `isAbstractEdge(edge)`.
pub fn is_abstract_edge(edge: &Edge<'_>) -> bool {
    matches!(edge.r#move, Move::Abstract(_))
}

/// `assertAbstractEdge(edge)`.
pub fn assert_abstract_edge(edge: &Edge<'_>) -> JsResult<()> {
    todo!()
}

/// `isFieldEdge(edge)`.
pub fn is_field_edge(edge: &Edge<'_>) -> bool {
    matches!(edge.r#move, Move::Field(_))
}

/// `assertFieldEdge(edge)`.
pub fn assert_field_edge(edge: &Edge<'_>) -> JsResult<()> {
    todo!()
}

impl<'s> Edge<'s> {
    pub fn new(head: NodeId, r#move: Move, tail: NodeId) -> Self {
        Edge {
            head,
            r#move,
            tail,
            resolvable: RefCell::new(Vec::new()),
            ignored: false,
        }
    }

    pub fn set_ignored(&mut self, ignored: bool) {
        self.ignored = ignored;
    }

    pub fn is_ignored(&self) -> bool {
        self.ignored
    }

    /// `isCrossGraphEdge()`: needs the nodes' graph ids, so the graph provides them.
    pub fn is_cross_graph_edge(&self, head_graph_id: &str, tail_graph_id: &str) -> bool {
        head_graph_id != tail_graph_id
    }

    /// `updateOverride(override)`. Throws when the move is not a field move.
    pub fn update_override(&mut self, r#override: FieldMoveOverride) -> JsResult<()> {
        todo!()
    }

    /// `getResolvability(graphNames, labelValues)`.
    pub fn get_resolvability(&self, graph_names: &[String], label_values: &OverrideLabels) -> Option<EdgeResolvabilityResult<'s>> {
        todo!()
    }

    /// `setResolvable(success, graphNames, labelValues, error?)`: `error` is `Some` exactly when
    /// `success` is false.
    pub fn set_resolvable(
        &self,
        success: bool,
        graph_names: Vec<String>,
        label_values: OverrideLabels,
        error: Option<LazySatisfiabilityError<'s>>,
    ) -> EdgeResolvabilityResult<'s> {
        todo!()
    }
}
