//! Port of `src/supergraph/validation/rules/satisfiablity/edge.ts`.
//!
//! Edges live in their `Graph`'s arena and are referred to by [`EdgeId`]; identity checks
//! (`edge === other`, `previousEdges.includes(edge)`) compare ids. `head`/`tail` are node ids
//! of the same graph. The resolvability cache is filled while walking, so it is a `RefCell`.
//!
//! NOTE(port): in TS several edges can share one `Move` object (`new Edge(head, edge.move, tail)`
//! in `duplicateNode`, `connectUnionOrInterface` and the `@provides` rewiring), and
//! `updateOverride` mutates the shared `FieldMove`. Here every edge owns a copy of its move. This
//! is not observable: the edges sharing a move always have the same head type, head graph, field
//! name and `provided` flag, so `addOverriddenFields` reaches and updates all of them with the same
//! value in the same iteration (no edge is ignored before that loop ends).

use std::cell::RefCell;
use std::rc::Rc;

use super::errors::{OneOrMany, SatisfiabilityError};
use super::graph::Graph;
use super::helpers::{Lazy, OverrideLabels};
use super::moves::{FieldMoveOverride, Move};
use super::node::{GraphNameId, NodeId};
use crate::utils::js::{JsError, JsResult};

/// Index of an edge in its graph's edge arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub usize);

/// A shared lazy satisfiability error (`Lazy<SatisfiabilityError>`).
pub type LazySatisfiabilityError<'s> = Rc<Lazy<'s, JsResult<OneOrMany<SatisfiabilityError>>>>;

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
    resolvable: RefCell<Vec<(Vec<GraphNameId>, OverrideLabels, EdgeResolvabilityResult<'s>)>>,
    ignored: bool,
}

/// `isEntityEdge(edge)`.
pub fn is_entity_edge(edge: &Edge<'_>) -> bool {
    matches!(edge.r#move, Move::Entity(_))
}

/// `assertEntityEdge(edge)`. Takes the graph to print the edge (`${edge}`) in the error.
pub fn assert_entity_edge(graph: &Graph<'_>, edge: EdgeId) -> JsResult<()> {
    if !is_entity_edge(graph.edge(edge)) {
        return Err(JsError::error(format!(
            "Expected edge to be Edge<EntityMove>, but got {}",
            graph.edge_to_string(edge)
        )));
    }
    Ok(())
}

/// `isAbstractEdge(edge)`.
pub fn is_abstract_edge(edge: &Edge<'_>) -> bool {
    matches!(edge.r#move, Move::Abstract(_))
}

/// `assertAbstractEdge(edge)`. Takes the graph to print the edge in the error.
pub fn assert_abstract_edge(graph: &Graph<'_>, edge: EdgeId) -> JsResult<()> {
    if !is_abstract_edge(graph.edge(edge)) {
        return Err(JsError::error(format!(
            "Expected edge to be Edge<AbstractMove>, but got {}",
            graph.edge_to_string(edge)
        )));
    }
    Ok(())
}

/// `isFieldEdge(edge)`.
pub fn is_field_edge(edge: &Edge<'_>) -> bool {
    matches!(edge.r#move, Move::Field(_))
}

/// `assertFieldEdge(edge)`. Takes the graph to print the edge in the error.
pub fn assert_field_edge(graph: &Graph<'_>, edge: EdgeId) -> JsResult<()> {
    if !is_field_edge(graph.edge(edge)) {
        return Err(JsError::error(format!(
            "Expected edge to be Edge<FieldMove>, but got {}",
            graph.edge_to_string(edge)
        )));
    }
    Ok(())
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

    /// `isCrossGraphEdge()`: needs the nodes' graph ids, so the graph provides them
    /// (`Graph::is_cross_graph_edge`).
    pub fn is_cross_graph_edge(&self, head_graph_id: &str, tail_graph_id: &str) -> bool {
        head_graph_id != tail_graph_id
    }

    /// `updateOverride(override)`. Throws when the move is not a field move.
    pub fn update_override(&mut self, r#override: FieldMoveOverride) -> JsResult<()> {
        match &mut self.r#move {
            Move::Field(m) => {
                m.r#override = Some(r#override);
                Ok(())
            }
            other => Err(JsError::error(format!(
                "Expected move to be FieldMove, but got {}",
                other.to_string()
            ))),
        }
    }

    /// `getResolvability(graphNames, labelValues)`.
    /// PERF: the caller's `graphNames` is `concatIfNotExistsString(visitedGraphs, graphName)`;
    /// it is passed as its two parts so that the list is not built for every lookup.
    pub fn get_resolvability(
        &self,
        visited_graphs: &[GraphNameId],
        graph_name: GraphNameId,
        label_values: &OverrideLabels,
    ) -> Option<EdgeResolvabilityResult<'s>> {
        self.resolvable
            .borrow()
            .iter()
            .find(|(checked_graph_names, checked_label_values, _)| {
                checked_graph_names
                    .iter()
                    .all(|name| *name == graph_name || visited_graphs.contains(name))
                    && checked_label_values.matches(label_values)
            })
            .map(|(_, _, result)| result.clone())
    }

    /// `setResolvable(success, graphNames, labelValues, error?)`: `error` is `Some` exactly when
    /// `success` is false.
    pub fn set_resolvable(
        &self,
        success: bool,
        graph_names: Vec<GraphNameId>,
        label_values: OverrideLabels,
        error: Option<LazySatisfiabilityError<'s>>,
    ) -> EdgeResolvabilityResult<'s> {
        let result = if success {
            EdgeResolvabilityResult::Success
        } else {
            EdgeResolvabilityResult::Failure {
                error: error.expect("setResolvable(false, ...) is always called with an error"),
            }
        };
        self.resolvable.borrow_mut().push((graph_names, label_values, result.clone()));
        result
    }
}
