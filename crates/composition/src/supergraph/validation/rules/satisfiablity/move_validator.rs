//! Port of `src/supergraph/validation/rules/satisfiablity/move-validator.ts`.

use std::collections::VecDeque;
use std::rc::Rc;

use super::edge::{is_abstract_edge, is_entity_edge, EdgeId, EdgeResolvabilityResult};
use super::errors::{LazyErrors, OneOrMany, SatisfiabilityError};
use super::finder::{concat_edge, concat_if_not_exists_fields, concat_if_not_exists_string, PathFinder, PathFinderResult};
use super::graph::Graph;
use super::helpers::{lazy, OverrideLabels};
use super::moves::Move;
use super::node::{GraphNameId, NodeTypeState};
use super::operation_path::OperationPath;
use super::selection::{Field, Selection, SelectionNode};
use crate::specifications::federation::FederationVersion;
use crate::utils::js::{JsError, JsResult};
use crate::utils::logger::Logger;

/// `MoveRequirement<T = SelectionNode>`.
/// PERF: borrows the selection from the resolved `Selection` (which outlives the check) and
/// shares the paths between the requirements created together, instead of copying both into
/// every requirement. Neither is mutated.
#[derive(Clone, Debug)]
pub struct MoveRequirement<'a> {
    pub paths: Rc<Vec<OperationPath>>,
    pub selection: &'a SelectionNode,
}

fn is_fragment_requirement(requirement: &MoveRequirement<'_>) -> bool {
    matches!(requirement.selection, SelectionNode::Fragment(_))
}

fn is_field_requirement(requirement: &MoveRequirement<'_>) -> bool {
    matches!(requirement.selection, SelectionNode::Field(_))
}

/// `RequirementResult`.
#[derive(Clone, Debug)]
pub enum RequirementResult<'s, 'a> {
    Success { requirements: Vec<MoveRequirement<'a>> },
    Failure { errors: LazyErrors<'s, SatisfiabilityError> },
}

impl RequirementResult<'_, '_> {
    pub fn success(&self) -> bool {
        matches!(self, RequirementResult::Success { .. })
    }
}

/// The result of `canResolveSelectionSet`: `{ success: true }` (an empty `Success`) or the
/// failing requirement's result.
pub type CanResolveSelectionSetResult<'s> = RequirementResult<'s, 'static>;

/// `MoveValidator`. Holds the merged graph (named `supergraph` in TS). Cheap to clone; its
/// `PathFinder` is created on demand (`path_finder()`), since the TS one points back at it.
#[derive(Clone)]
pub struct MoveValidator<'s> {
    logger: Logger,
    supergraph: Rc<Graph<'s>>,
}

/// `Lazy` holding one `SatisfiabilityError`.
fn lazy_error<'s>(factory: impl Fn() -> SatisfiabilityError + 's) -> super::edge::LazySatisfiabilityError<'s> {
    Rc::new(lazy(move || Ok(OneOrMany::One(factory()))))
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
        visited_graphs: &[GraphNameId],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<CanResolveSelectionSetResult<'s>> {
        // `requirements.unshift(..)` is `push_front`, `requirements.pop()` is `pop_back`.
        let mut requirements: VecDeque<MoveRequirement<'_>> = VecDeque::new();

        let paths = Rc::new(vec![path.clone_path()]);
        for selection in selection_set {
            requirements.push_front(MoveRequirement {
                selection,
                paths: paths.clone(),
            });
        }

        // it's important to pop from the end as we want to process the last added requirement first
        while let Some(requirement) = requirements.pop_back() {
            let result = self.validate_requirement(&requirement, visited_edges, visited_graphs, visited_fields, label_values)?;

            match result {
                RequirementResult::Failure { errors } => return Ok(RequirementResult::Failure { errors }),
                RequirementResult::Success { requirements: inner } => {
                    for inner_requirement in inner {
                        requirements.push_front(inner_requirement);
                    }
                }
            }
        }

        Ok(RequirementResult::Success { requirements: Vec::new() })
    }

    /// `requirement.selection` is a `Fragment`.
    fn validate_fragment_requirement<'a>(
        &self,
        requirement: &MoveRequirement<'a>,
        visited_edges: &[EdgeId],
        visited_graphs: &[GraphNameId],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<RequirementResult<'s, 'a>> {
        let SelectionNode::Fragment(selection) = requirement.selection else {
            return Err(JsError::error("Expected a fragment requirement"));
        };
        let graph = &*self.supergraph;
        self.logger.log(|| format!("Validating: ... on {}", selection.type_name));

        let mut next_paths: Vec<OperationPath> = Vec::new();
        let mut errors = LazyErrors::new();

        // Looks like we hit a fragment spread that matches the current type.
        // It means that it's a fragment spread on an object type, not a union or interface.
        // We can ignore the fragment and continue with the selection set.
        let Some(first_path) = requirement.paths.first() else {
            return Err(JsError::type_error("Cannot read properties of undefined (reading 'tail')"));
        };
        if first_path.tail(graph).map(|t| graph.node(t).type_name.as_str()) == Some(selection.type_name.as_str()) {
            return Ok(RequirementResult::Success {
                requirements: selection
                    .selection_set
                    .iter()
                    .map(|s| MoveRequirement {
                        selection: s,
                        paths: requirement.paths.clone(),
                    })
                    .collect(),
            });
        }

        let path_finder = self.path_finder();

        for path in requirement.paths.iter() {
            let direct_paths_result =
                path_finder.find_direct_paths(path, &selection.type_name, None, visited_edges, label_values)?;
            match direct_paths_result {
                PathFinderResult::Success { paths } => {
                    if self.logger.is_enabled {
                        self.logger.log("Possible direct paths:");
                        for path in &paths {
                            self.logger.log(|| format!(" {}", path.to_string(graph)));
                        }
                    }
                    next_paths.extend(paths);
                }
                PathFinderResult::Failure { errors: e } => {
                    errors.add_all(e);
                }
            }
        }

        // // we could add these as lazy
        // try indirect paths
        for path in requirement.paths.iter() {
            let indirect_paths_result = path_finder.find_indirect_paths(
                path,
                &selection.type_name,
                None,
                visited_edges,
                visited_graphs,
                visited_fields,
                label_values,
            )?;

            match indirect_paths_result {
                PathFinderResult::Success { paths } => {
                    if self.logger.is_enabled {
                        self.logger.log("Possible indirect paths:");
                        for path in &paths {
                            self.logger.log(|| format!(" {}", path.to_string(graph)));
                        }
                    }
                    next_paths.extend(paths);
                }
                PathFinderResult::Failure { errors: e } => {
                    errors.add_all(e);
                }
            }
        }

        if next_paths.is_empty() {
            if self.logger.is_enabled {
                self.logger.log("Could not resolve from:");
                for path in requirement.paths.iter() {
                    self.logger.log(|| format!(" {}", path.to_string(graph)));
                }
            }

            // cannot advance
            return Ok(RequirementResult::Failure { errors });
        }

        if selection.selection_set.is_empty() {
            // we reached the end of the path
            return Ok(RequirementResult::Success { requirements: Vec::new() });
        }

        let next_paths = Rc::new(next_paths);
        Ok(RequirementResult::Success {
            requirements: selection
                .selection_set
                .iter()
                .map(|s| MoveRequirement {
                    selection: s,
                    paths: next_paths.clone(),
                })
                .collect(),
        })
    }

    /// `requirement.selection` is a `Field`.
    fn validate_field_requirement<'a>(
        &self,
        requirement: &MoveRequirement<'a>,
        visited_edges: &[EdgeId],
        visited_graphs: &[GraphNameId],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<RequirementResult<'s, 'a>> {
        let SelectionNode::Field(selection) = requirement.selection else {
            return Err(JsError::error("Expected a field requirement"));
        };
        let graph = &*self.supergraph;
        let Field {
            field_name, type_name, ..
        } = selection;
        self.logger.log(|| format!("Validating: {type_name}.{field_name}"));

        let mut next_paths: Vec<OperationPath> = Vec::new();
        let mut errors = LazyErrors::new();
        let path_finder = self.path_finder();

        for path in requirement.paths.iter() {
            let direct_paths_result =
                path_finder.find_direct_paths(path, type_name, Some(field_name), visited_edges, label_values)?;
            match direct_paths_result {
                PathFinderResult::Success { paths } => {
                    if self.logger.is_enabled {
                        self.logger.log("Possible direct paths:");
                        for path in &paths {
                            self.logger.log(|| format!(" {}", path.to_string(graph)));
                        }
                    }
                    next_paths.extend(paths);
                }
                PathFinderResult::Failure { errors: e } => {
                    errors.add_all(e);
                }
            }
        }

        // we could add make it lazy
        for path in requirement.paths.iter() {
            let indirect_paths_result = path_finder.find_indirect_paths(
                path,
                type_name,
                Some(field_name),
                visited_edges,
                visited_graphs,
                visited_fields,
                label_values,
            )?;

            match indirect_paths_result {
                PathFinderResult::Success { paths } => {
                    if self.logger.is_enabled {
                        self.logger.log("Possible indirect paths:");
                        for path in &paths {
                            self.logger.log(|| format!(" {}", path.to_string(graph)));
                        }
                    }
                    next_paths.extend(paths);
                }
                PathFinderResult::Failure { errors: e } => {
                    errors.add_all(e);
                }
            }
        }

        if next_paths.is_empty() {
            self.logger.log(|| format!("Failed to resolve field {type_name}.{field_name} from:"));
            if self.logger.is_enabled {
                for path in requirement.paths.iter() {
                    self.logger.log(|| format!(" {}", path.to_string(graph)));
                }
            }

            // cannot advance
            return Ok(RequirementResult::Failure { errors });
        }

        match &selection.selection_set {
            Some(selection_set) if !selection_set.is_empty() => {
                let next_paths = Rc::new(next_paths);
                Ok(RequirementResult::Success {
                    requirements: selection_set
                        .iter()
                        .map(|s| MoveRequirement {
                            selection: s,
                            paths: next_paths.clone(),
                        })
                        .collect(),
                })
            }
            // we reached the end of the path
            _ => Ok(RequirementResult::Success { requirements: Vec::new() }),
        }
    }

    fn validate_requirement<'a>(
        &self,
        requirement: &MoveRequirement<'a>,
        visited_edges: &[EdgeId],
        visited_graphs: &[GraphNameId],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<RequirementResult<'s, 'a>> {
        if is_fragment_requirement(requirement) {
            return self.validate_fragment_requirement(requirement, visited_edges, visited_graphs, visited_fields, label_values);
        }

        if is_field_requirement(requirement) {
            return self.validate_field_requirement(requirement, visited_edges, visited_graphs, visited_fields, label_values);
        }

        // NOTE(port): unreachable, `SelectionNode` has two kinds
        // (`Unsupported requirement: ${requirement.selection.kind}` in TS).
        Err(JsError::error("Unsupported requirement"))
    }

    pub fn is_external(&self, edge: EdgeId) -> bool {
        let graph = &*self.supergraph;
        let e = graph.edge(edge);
        let Move::Field(field_move) = &e.r#move else {
            return false;
        };

        let head = graph.node(e.head);
        let Some(NodeTypeState::Object(type_state)) = head.type_state else {
            return false;
        };
        if field_move.provided {
            return false;
        }

        let Some(field_state) = type_state.fields.get(&field_move.field_name) else {
            return false;
        };

        let object_type_state_in_graph = type_state.by_graph.get(&head.graph_id);
        let field_state_in_graph = field_state.by_graph.get(&head.graph_id);

        let (Some(field_state_in_graph), Some(object_type_state_in_graph)) = (field_state_in_graph, object_type_state_in_graph) else {
            return false;
        };

        if !field_state_in_graph.external {
            return false;
        }

        if field_state_in_graph.version == FederationVersion::V1_0
            && object_type_state_in_graph.extension
            && field_state.used_as_key
        {
            return false;
        }

        if !field_state_in_graph.used_as_key {
            return true;
        }

        if object_type_state_in_graph.extension {
            return false;
        }

        true
    }

    /// `edge` is a field edge.
    fn can_access_field_with_override(&self, edge: EdgeId, override_labels: &OverrideLabels) -> bool {
        let Some(field_move) = self.supergraph.edge(edge).r#move.as_field() else {
            return true;
        };
        let Some(r#override) = &field_move.r#override else {
            return true;
        };

        // the edge is overridden if the expected value is different from the actual value
        // If we have label: "percent(10)":
        //   true will be for the edge field with @override(label: "percent(10)", from: "a") in Subgraph B
        //   false will be for the edge field in Subgraph A
        // If we do not have a label:
        //  true will be for the edge field with @override(from: "a") in Subgraph B
        //  false will be for the edge field in Subgraph A

        let Some(label) = r#override.label.as_deref().filter(|l| !l.is_empty()) else {
            return r#override.value;
        };

        let Some(value) = override_labels.get(label) else {
            return false;
        };

        r#override.value == value
    }

    #[allow(clippy::too_many_arguments)]
    pub fn is_edge_resolvable(
        &self,
        edge: EdgeId,
        path: &OperationPath,
        visited_edges: &[EdgeId],
        visited_graphs: &[GraphNameId],
        visited_fields: &[Rc<Selection>],
        label_values: &OverrideLabels,
    ) -> JsResult<EdgeResolvabilityResult<'s>> {
        let graph = &*self.supergraph;
        let e = graph.edge(edge);
        let head = graph.node(e.head);
        let tail = graph.node(e.tail);
        self.logger.group(|| format!("Checking resolvability of {}", graph.edge_to_string(edge)));
        self.logger.log(|| format!("Visited graphs: {}", graph.graph_names.join(visited_graphs, ",")));

        // Access guards always run and are never cached.
        // Keep this order so override/external errors are reported before context errors.
        if let Move::Field(field_move) = &e.r#move {
            if !self.can_access_field_with_override(edge, label_values) {
                self.logger.group_end(|| {
                    format!(
                        "Cannot move to {} because it requirement of the override is not met",
                        graph.edge_to_string(edge)
                    )
                });
                let (graph_name, type_name, field_name) =
                    (tail.graph_name.clone(), field_move.type_name.clone(), field_move.field_name.clone());
                return Ok(EdgeResolvabilityResult::Failure {
                    error: lazy_error(move || SatisfiabilityError::for_missing_field(&graph_name, &type_name, &field_name)),
                });
            }

            if self.is_external(edge) {
                self.logger.group_end(|| {
                    format!("Cannot move to {} because it is external and cross-graph", graph.edge_to_string(edge))
                });
                let (graph_name, type_name, field_name) =
                    (head.graph_name.clone(), field_move.type_name.clone(), field_move.field_name.clone());
                return Ok(EdgeResolvabilityResult::Failure {
                    error: lazy_error(move || SatisfiabilityError::for_external(&graph_name, &type_name, &field_name)),
                });
            }

            if !path.has_contexts(&field_move.required_contexts) {
                self.logger.group_end(|| {
                    format!("Cannot move to {} because an ancestor context is missing", graph.edge_to_string(edge))
                });
                let (graph_name, type_name, field_name) =
                    (head.graph_name.clone(), field_move.type_name.clone(), field_move.field_name.clone());
                return Ok(EdgeResolvabilityResult::Failure {
                    error: lazy_error(move || SatisfiabilityError::for_required_context(&graph_name, &type_name, &field_name)),
                });
            }
        }

        let resolvability = e.get_resolvability(visited_graphs, tail.graph_name_id, label_values);

        if let Some(resolvability) = resolvability {
            self.logger.group_end(|| {
                if resolvability.success() {
                    format!("Can move to {}", graph.edge_to_string(edge))
                } else {
                    format!("Cannot move to {} (already visited)", graph.edge_to_string(edge))
                }
            });

            return Ok(resolvability);
        }

        if let Move::Field(field_move) = &e.r#move {
            if let Some(requires) = &field_move.requires {
                self.logger.log("Detected @requires");

                let new_visited_graphs = if graph.is_cross_graph_edge(edge) {
                    concat_if_not_exists_string(visited_graphs, &tail.graph_name_id)
                } else {
                    visited_graphs.to_vec()
                };
                let new_visited_fields = concat_if_not_exists_fields(visited_fields, requires);
                self.logger.log(|| format!("Visited graphs: {}", graph.graph_names.join(&new_visited_graphs, ",")));
                let new_visited_edges = concat_edge(visited_edges, edge);
                if self
                    .can_resolve_selection_set(
                        &requires.selection_set,
                        // A `@requires` selection starts with no ancestor contexts in scope.
                        &path.without_contexts(),
                        &new_visited_edges,
                        &new_visited_graphs,
                        &new_visited_fields,
                        label_values,
                    )?
                    .success()
                {
                    self.logger.group_end(|| format!("Can move to {}", graph.edge_to_string(edge)));
                    return Ok(e.set_resolvable(true, new_visited_graphs, label_values.clone_labels(), None));
                }

                self.logger.group_end(|| {
                    format!("Cannot move to {} because @require is not resolvable", graph.edge_to_string(edge))
                });

                let (graph_name, type_name, field_name) =
                    (head.graph_name.clone(), field_move.type_name.clone(), field_move.field_name.clone());
                return Ok(EdgeResolvabilityResult::Failure {
                    error: lazy_error(move || SatisfiabilityError::for_require(&graph_name, &type_name, &field_name)),
                });
            }

            self.logger.group_end(|| format!("Can move to {}", graph.edge_to_string(edge)));
            return Ok(e.set_resolvable(true, visited_graphs.to_vec(), label_values.clone_labels(), None));
        }

        if !is_entity_edge(e) && !is_abstract_edge(e) {
            return Err(JsError::error("Expected edge to be entity or abstract"));
        }

        let key_fields = match &e.r#move {
            Move::Entity(m) => Some(m.key_fields.clone()),
            Move::Abstract(m) => m.key_fields.clone(),
            Move::Field(_) => None,
        };

        let Some(key_fields) = key_fields else {
            self.logger.group_end(|| format!("Can move to {}", graph.edge_to_string(edge)));
            return Ok(e.set_resolvable(true, visited_graphs.to_vec(), label_values.clone_labels(), None));
        };

        let new_visited_graphs = concat_if_not_exists_string(visited_graphs, &tail.graph_name_id);
        let new_visited_fields = concat_if_not_exists_fields(visited_fields, &key_fields);

        self.logger.log("Detected @key");
        self.logger.log(|| format!("Visited graphs: {}", graph.graph_names.join(&new_visited_graphs, ",")));
        let new_visited_edges = concat_edge(visited_edges, edge);
        let resolvable = self
            .can_resolve_selection_set(
                &key_fields.selection_set,
                path,
                &new_visited_edges,
                &new_visited_graphs,
                &new_visited_fields,
                label_values,
            )?
            .success();

        if resolvable {
            self.logger.group_end(|| format!("Can move to {}", graph.edge_to_string(edge)));
            return Ok(e.set_resolvable(true, new_visited_graphs, label_values.clone_labels(), None));
        }

        self.logger.group_end(|| {
            format!("Cannot move to {} because key fields are not resolvable", graph.edge_to_string(edge))
        });

        // NOTE(port): this lazy error is cached in the edge, so it captures owned values rather
        // than the graph (an `Rc<Graph>` inside the graph would never be freed).
        let (source_graph_name, target_graph_name, type_name) =
            (head.graph_name.clone(), tail.graph_name.clone(), head.type_name.clone());
        Ok(e.set_resolvable(
            false,
            new_visited_graphs,
            label_values.clone_labels(),
            Some(lazy_error(move || {
                SatisfiabilityError::for_key(&source_graph_name, &target_graph_name, &type_name, &key_fields.to_string())
            })),
        ))
    }
}
