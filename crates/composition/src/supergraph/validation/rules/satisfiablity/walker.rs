//! Port of `src/supergraph/validation/rules/satisfiablity/walker.ts`.

use std::rc::Rc;

use graphql_js::OperationType;
use indexmap::{IndexMap, IndexSet};

use super::contexts::ContextMask;
use super::edge::{is_abstract_edge, is_field_edge, EdgeId};
use super::errors::{LazyErrors, SatisfiabilityError, SatisfiabilityErrorKind};
use super::finder::{PathFinder, PathFinderResult};
use super::graph::Graph;
use super::helpers::{is_truthy_str, OverrideLabels};
use super::move_validator::MoveValidator;
use super::moves::{FieldMove, Move};
use super::node::NodeId;
use super::operation_path::{OperationPath, Step};
use crate::utils::js::{JsError, JsResult};
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
        match &supergraph.edge(edge).r#move {
            Move::Field(field_move) => {
                if let Some(r#override) = &field_move.r#override {
                    if let Some(label) = r#override.label.as_deref().filter(|l| !l.is_empty()) {
                        return Ok(WalkTracker::new(
                            self.super_path.moved(supergraph, edge),
                            Vec::new(),
                            self.label_values.clone_labels().set(label, r#override.value),
                        ));
                    }
                }
                Ok(WalkTracker::new(
                    self.super_path.moved(supergraph, edge),
                    Vec::new(),
                    self.label_values.clone_labels(),
                ))
            }
            Move::Abstract(_) => Ok(WalkTracker::new(
                self.super_path.moved(supergraph, edge),
                Vec::new(),
                self.label_values.clone_labels(),
            )),
            Move::Entity(_) => Err(JsError::error("Expected edge to be FieldMove or AbstractMove")),
        }
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
        let last_edge = self.super_path.edge();
        self.paths.is_empty()
            && self.errors.is_empty()
            && last_edge.is_some_and(|edge| is_abstract_edge(supergraph.edge(edge)))
    }

    pub fn is_edge_visited(&self, edge: EdgeId) -> bool {
        self.super_path.is_visited_edge(edge)
    }

    /// `listErrors()`: deduplicated by message. Throws if evaluating a lazy error throws.
    pub fn list_errors(&self, supergraph: &Graph<'s>) -> JsResult<Vec<SatisfiabilityError>> {
        let all = self.errors.to_array()?;

        let unique: Vec<&SatisfiabilityError> = all
            .iter()
            .enumerate()
            .filter(|(i, error)| all.iter().position(|e| e.to_string() == error.to_string()) == Some(*i))
            .map(|(_, error)| error)
            .collect();

        Ok(unique
            .into_iter()
            .filter(|error| {
                if error.kind != SatisfiabilityErrorKind::Key {
                    return true;
                }

                // remove key resolution errors that does not lead to the field we are looking for
                let steps = self.super_path.steps(supergraph);
                let last_step = steps.last();

                let last_step_type_name = match last_step {
                    Some(Step::Field { type_name, .. }) | Some(Step::Abstract { type_name }) => Some(type_name),
                    None => None,
                };

                if last_step_type_name.is_none_or(|t| *t != error.type_name) {
                    return true;
                }

                true
            })
            .cloned()
            .collect())
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

    /// The root `WalkTracker` of a root node of the supergraph graph.
    fn root_tracker(&self, root_node: NodeId, override_labels: &OverrideLabels) -> JsResult<WalkTracker<'s>> {
        let root_type_name = &self.supergraph.node(root_node).type_name;
        Ok(WalkTracker::new(
            OperationPath::new(root_node),
            self.merged_graph
                .nodes_of(root_type_name, false)?
                .into_iter()
                .map(OperationPath::new)
                .collect(),
            override_labels.clone_labels(),
        ))
    }

    /// `walkTrail(operationType, steps)`: walks only the given steps.
    pub fn walk_trail(&self, operation_type: OperationType, steps: &[Step]) -> JsResult<WalkTracker<'s>> {
        if steps.is_empty() {
            return Err(JsError::error("Expected at least one step"));
        }

        let supergraph = &*self.supergraph;
        let root_type_name = match operation_type {
            OperationType::Query => "Query",
            OperationType::Mutation => "Mutation",
            _ => "Subscription",
        };
        let Some(root_node) = supergraph.node_of(root_type_name, false)? else {
            return Err(JsError::error(format!(
                "Expected root node for operation type {}",
                operation_type.as_str()
            )));
        };

        let mut state = self.root_tracker(root_node, &OverrideLabels::new(None))?;

        for step in steps {
            let is_edge_ignored = |edge: EdgeId| -> bool {
                match step {
                    Step::Field { field_name, .. } => match &supergraph.edge(edge).r#move {
                        Move::Field(field_move) => field_move.field_name != *field_name,
                        _ => true,
                    },
                    Step::Abstract { .. } => true,
                }
            };

            let mut called = 0;
            let mut unreachable = false;
            let mut empty_object_result = false;
            // NOTE(port): the TS callback assigns the outer `state`; `nextStep` keeps using the
            // state it was given, so the last assigned value is applied after the call.
            let mut next_state_slot: Option<WalkTracker<'s>> = None;

            let super_tail = state.super_path.tail_or_root(supergraph);
            self.next_step(
                &state,
                super_tail,
                &mut |next_state, super_edge| {
                    let was_called = called;
                    called += 1;
                    if was_called > 1 {
                        return Err(JsError::error("Expected nextStep to be called only once"));
                    }

                    if next_state.is_possible() {
                        self.logger.group_end(|| {
                            format!(
                                "Advanced to {} with {} paths",
                                supergraph.edge_to_string(super_edge),
                                next_state.paths.len()
                            )
                        });
                    } else if next_state.gives_empty_result(supergraph) {
                        empty_object_result = true;
                        self.logger.group_end(|| {
                            format!(
                                "Federation will resolve an empty object for {}",
                                supergraph.edge_to_string(super_edge)
                            )
                        });
                    } else {
                        unreachable = true;
                        self.logger.log("Dead end");
                        self.logger.group_end(|| {
                            format!("Unreachable path {}", next_state.super_path.to_string(supergraph))
                        });
                    }

                    next_state_slot = Some(next_state);
                    Ok(())
                },
                &is_edge_ignored,
            )?;

            if let Some(next_state) = next_state_slot {
                state = next_state;
            }

            if unreachable {
                break;
            }

            if empty_object_result {
                break;
            }
        }

        Ok(state)
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
        let supergraph = &*self.supergraph;
        let merged_graph = &*self.merged_graph;

        let mut graphs_leading_to_node: IndexSet<String> = IndexSet::new();
        for p in &state.paths {
            let edge = p.edge();
            let tail = p.tail_or_root(merged_graph);
            let tail_graph_name = &merged_graph.node(tail).graph_name;

            // To prevent a scenario where multiple paths converge on the same Node,
            // differing fundamentally – some with the "@provides" directive and others without – we must differentiate them
            // not solely by the graph names leading to the Node,
            // but also by the presence of provided fields.
            // Simply marking a Node as visited and successfully checked after traversing a path with a @provided field,
            // using only graph names, results in a flaw.
            // This is because other paths leading to the same Node, lacking a @provided field,
            // will also be incorrectly marked as resolvable.
            // This inaccuracy arises because the Node contains an @external field, making it non-resolvable (non-provided field).
            // The same reasoning applies to contexts set by the ancestors of the Node.
            // Two paths can lead to the same Node from the same graphs,
            // but only one of them may provide a context required by a `@fromContext` argument.
            // The context key is empty when the supergraph has no `@context`.
            let contexts_key = if p.contexts().is_zero() {
                String::new()
            } else {
                format!("#contexts:{}", p.contexts())
            };

            let has_provides = edge.is_some_and(|edge| {
                matches!(
                    &merged_graph.edge(edge).r#move,
                    Move::Field(FieldMove { provides: Some(_), .. })
                )
            });

            if has_provides {
                graphs_leading_to_node.insert(format!("{tail_graph_name}#provides{contexts_key}"));
            } else {
                graphs_leading_to_node.insert(format!("{tail_graph_name}{contexts_key}"));
            }
        }
        let graphs_leading_to_node: Vec<String> = graphs_leading_to_node.into_iter().collect();

        let super_tail_node = supergraph.node(super_tail);
        if super_tail_node.is_graph_combo_visited(&graphs_leading_to_node, &state.label_values) {
            self.logger
                .log(|| format!("Node already visited: {}", super_tail_node.to_string()));
            return Ok(());
        }

        super_tail_node.set_graph_combo_as_visited(graphs_leading_to_node);

        let super_edges = supergraph.edges_of_head(super_tail);

        for super_edge in super_edges {
            if is_edge_ignored(super_edge) {
                continue;
            }
            self.logger.group(|| {
                format!(
                    "Attempt to advance to {} ({} paths)",
                    supergraph.edge_to_string(super_edge),
                    state.paths.len()
                )
            });

            if state.is_edge_visited(super_edge) {
                self.logger
                    .group_end(|| format!("Edge already visited: {}", supergraph.edge_to_string(super_edge)));
                continue;
            }

            let se = supergraph.edge(super_edge);

            if !(is_field_edge(se) || is_abstract_edge(se)) {
                return Err(JsError::error("Expected edge to have a FieldMove or AbstractMove"));
            }

            if let Move::Field(field_move) = &se.r#move {
                if let Some(r#override) = &field_move.r#override {
                    if is_truthy_str(&r#override.label) {
                        let label = r#override.label.as_deref().unwrap_or_default();
                        let label_value = state.label_values.get(label);
                        if label_value.is_some_and(|v| v != r#override.value) {
                            self.logger.group_end(|| {
                                format!("Different label value. Skipping {}", supergraph.edge_to_string(super_edge))
                            });
                            continue;
                        }
                    }
                }
            }

            let mut next_state = state.r#move(supergraph, super_edge)?;
            let mut shortest_path_per_tail: ShortestPaths = IndexMap::new();
            let (target_type_name, target_field_name) = match &se.r#move {
                Move::Field(field_move) => (field_move.type_name.as_str(), Some(field_move.field_name.as_str())),
                _ => (supergraph.node(se.tail).type_name.as_str(), None),
            };
            let super_tail_is_leaf = supergraph.node(se.tail).is_leaf;

            for path in &state.paths {
                self.logger
                    .group(|| format!("Advance path: {}", path.to_string(merged_graph)));

                let direct_paths_result = self.path_finder.find_direct_paths(
                    path,
                    target_type_name,
                    target_field_name,
                    &[],
                    &next_state.label_values,
                )?;

                let direct_success = matches!(direct_paths_result, PathFinderResult::Success { .. });

                match &direct_paths_result {
                    // Special case when it's an abstract type and there are no direct paths
                    PathFinderResult::Success { paths } if paths.is_empty() => {
                        self.logger.group_end("Abstract type");
                        continue;
                    }
                    PathFinderResult::Success { paths } => {
                        set_shortest_path(merged_graph, &mut shortest_path_per_tail, paths);
                    }
                    PathFinderResult::Failure { errors } => {
                        next_state.add_error(errors);
                    }
                }

                if direct_success && super_tail_is_leaf {
                    self.logger
                        .group_end("Reached leaf node, no need to find indirect paths");
                    break;
                }

                let indirect_paths_result = self.path_finder.find_indirect_paths(
                    path,
                    target_type_name,
                    target_field_name,
                    &[],
                    &[],
                    &[],
                    &next_state.label_values,
                )?;

                let indirect_success = matches!(indirect_paths_result, PathFinderResult::Success { .. });

                match &indirect_paths_result {
                    PathFinderResult::Success { paths } => {
                        set_shortest_path(merged_graph, &mut shortest_path_per_tail, paths);
                    }
                    PathFinderResult::Failure { errors } => {
                        next_state.add_error(errors);
                    }
                }
                self.logger.group_end(|| {
                    if direct_success || indirect_success {
                        "Can advance".to_string()
                    } else {
                        "Cannot advance".to_string()
                    }
                });
            }

            for paths_by_contexts in shortest_path_per_tail.values() {
                for shortest_path in paths_by_contexts.values() {
                    next_state.add_path(shortest_path.clone());
                }
            }
            next(next_state, super_edge)?;
        }

        Ok(())
    }

    fn dfs(&self) -> JsResult<Vec<WalkTracker<'s>>> {
        let mut unreachable: Vec<WalkTracker<'s>> = Vec::new();

        let mut root_nodes = Vec::new();
        for name in ["Query", "Mutation", "Subscription"] {
            if let Some(node) = self.supergraph.node_of(name, false)? {
                root_nodes.push(node);
            }
        }

        let override_labels = OverrideLabels::new(None);

        for root_node in root_nodes {
            let state = self.root_tracker(root_node, &override_labels)?;
            self.dfs_inner(root_node, state, &mut unreachable)?;
        }

        Ok(unreachable)
    }

    fn dfs_inner(&self, super_tail: NodeId, state: WalkTracker<'s>, unreachable: &mut Vec<WalkTracker<'s>>) -> JsResult<()> {
        if self.supergraph.node(super_tail).is_leaf {
            return Ok(());
        }

        let supergraph = &*self.supergraph;
        self.next_step(
            &state,
            super_tail,
            &mut |next_state, super_edge| {
                if next_state.is_possible() {
                    self.logger.group_end(|| {
                        format!(
                            "Advanced to {} with {} paths",
                            supergraph.edge_to_string(super_edge),
                            next_state.paths.len()
                        )
                    });
                    self.dfs_inner(supergraph.edge(super_edge).tail, next_state, unreachable)?;
                } else if next_state.gives_empty_result(supergraph) {
                    self.logger.group_end(|| {
                        format!(
                            "Federation will resolve an empty object for {}",
                            supergraph.edge_to_string(super_edge)
                        )
                    });
                } else {
                    self.logger.log("Dead end");
                    self.logger
                        .group_end(|| format!("Unreachable path {}", next_state.super_path.to_string(supergraph)));
                    unreachable.push(next_state);
                }
                Ok(())
            },
            &default_is_edge_ignored,
        )
    }

    fn bfs(&self) -> JsResult<Vec<WalkTracker<'s>>> {
        let override_labels = OverrideLabels::new(None);
        let mut unreachable: Vec<WalkTracker<'s>> = Vec::new();
        let mut queue: Vec<WalkTracker<'s>> = Vec::new();

        for name in ["Query", "Mutation", "Subscription"] {
            let Some(root_node) = self.supergraph.node_of(name, false)? else {
                continue;
            };

            queue.push(self.root_tracker(root_node, &override_labels)?);
        }

        let supergraph = &*self.supergraph;
        while let Some(state) = queue.pop() {
            let super_tail = state.super_path.tail_or_root(supergraph);

            if supergraph.node(super_tail).is_leaf {
                continue;
            }

            self.next_step(
                &state,
                super_tail,
                &mut |next_state, super_edge| {
                    if next_state.is_possible() {
                        self.logger.group_end(|| {
                            format!(
                                "Advanced to {} with {} paths",
                                supergraph.edge_to_string(super_edge),
                                next_state.paths.len()
                            )
                        });
                        queue.push(next_state);
                    } else if next_state.gives_empty_result(supergraph) {
                        self.logger.group_end(|| {
                            format!(
                                "Federation will resolve an empty object for {}",
                                supergraph.edge_to_string(super_edge)
                            )
                        });
                    } else {
                        self.logger.log("Dead end");
                        self.logger
                            .group_end(|| format!("Unreachable path {}", next_state.super_path.to_string(supergraph)));
                        unreachable.push(next_state);
                    }
                    Ok(())
                },
                &default_is_edge_ignored,
            )?;
        }

        Ok(unreachable)
    }
}

/// `setShortestPath(shortestPathPerTail, paths)`. Paths reaching the same node are
/// interchangeable, so only the shortest is kept, per contexts set by their ancestors.
fn set_shortest_path(merged_graph: &Graph<'_>, shortest_path_per_tail: &mut ShortestPaths, paths: &[OperationPath]) {
    for path in paths {
        let tail = path.tail_or_root(merged_graph);
        let paths_by_contexts = shortest_path_per_tail.entry(tail).or_default();

        let replace = match paths_by_contexts.get(path.contexts()) {
            None => true,
            Some(shortest) => shortest.depth() > path.depth(),
        };

        if replace {
            paths_by_contexts.insert(path.contexts().clone(), path.clone());
        }
    }
}
