//! Port of `src/supergraph/validation/rules/satisfiablity/finder.ts`.

use std::rc::Rc;

use super::edge::{is_abstract_edge, EdgeId, EdgeResolvabilityResult};
use super::errors::{LazyErrors, OneOrMany, SatisfiabilityError};
use super::graph::Graph;
use super::helpers::{lazy, OverrideLabels};
use super::move_validator::MoveValidator;
use super::moves::Move;
use super::node::NodeTypeState;
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

/// `edge.move.keyFields` of an entity or abstract move (`None` for a field move).
fn key_fields_of(r#move: &Move) -> Option<Rc<Selection>> {
    match r#move {
        Move::Entity(m) => Some(m.key_fields.clone()),
        Move::Abstract(m) => m.key_fields.clone(),
        Move::Field(_) => None,
    }
}

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
        let graph = &*self.graph;
        let mut next_paths: Vec<OperationPath> = Vec::new();
        let mut errors = LazyErrors::new();
        let tail = path.tail_or_root(graph);
        let is_field_target = field_name.is_some();
        let id = match field_name {
            Some(field_name) => format!("{type_name}.{field_name}"),
            None => format!("... on {type_name}"),
        };

        self.logger
            .group(|| format!("Direct paths to {id} from: {}", graph.node(tail).to_string()));

        let edges = match field_name {
            Some(field_name) => graph.field_edges_of_head(tail, field_name)?,
            None => graph.abstract_edges_of_head(tail)?,
        };

        self.logger.log(|| format!("Checking {} edges", edges.len()));

        for (i, &edge) in edges.iter().enumerate() {
            self.logger
                .group(|| format!("Checking #{i} {}", graph.edge_to_string(edge)));
            let e = graph.edge(edge);

            if next_paths.iter().any(|p| p.tail(graph) == Some(e.tail)) {
                self.logger
                    .group_end(|| format!("Already resolvable tail: {}", graph.edge_to_string(edge)));
                continue;
            }

            if graph.is_cross_graph_edge(edge) {
                self.logger
                    .group_end(|| format!("Cross graph edge: {}", graph.edge_to_string(edge)));
                continue;
            }

            if visited_edges.contains(&edge) {
                self.logger
                    .group_end(|| format!("Already visited: {}", graph.edge_to_string(edge)));
                continue;
            }

            if !is_field_target && is_abstract_edge(e) && graph.node(e.tail).type_name == type_name {
                self.logger
                    .group_end(|| format!("Resolvable: {}", graph.edge_to_string(edge)));
                next_paths.push(path.moved(graph, edge));
                continue;
            }

            if let (Some(field_name), Move::Field(field_move)) = (field_name, &e.r#move) {
                if field_move.field_name == field_name {
                    let resolvable = self
                        .move_validator
                        .is_edge_resolvable(edge, path, &[], &[], &[], label_values)?;
                    match resolvable {
                        EdgeResolvabilityResult::Failure { error } => {
                            errors.add(error);
                            self.logger
                                .group_end(|| format!("Not resolvable: {}", graph.edge_to_string(edge)));
                            continue;
                        }
                        EdgeResolvabilityResult::Success => {
                            self.logger
                                .group_end(|| format!("Resolvable: {}", graph.edge_to_string(edge)));
                            next_paths.push(path.moved(graph, edge));
                            continue;
                        }
                    }
                }
            }

            self.logger.group_end("Not matching");
        }

        self.logger
            .group_end(|| format!("Found {} direct paths", next_paths.len()));

        if !next_paths.is_empty() {
            return Ok(PathFinderResult::Success { paths: next_paths });
        }

        if !errors.is_empty() {
            return Ok(PathFinderResult::Failure { errors });
        }

        let tail_node = graph.node(tail);

        let Some(field_name) = field_name else {
            if let Some(NodeTypeState::Interface(type_state)) = tail_node.type_state {
                if type_state.has_interface_object {
                    let type_state_in_graph = type_state.by_graph.get(&tail_node.graph_id);

                    if type_state_in_graph.is_some_and(|g| g.is_interface_object) {
                        // no subgraph can be reached to resolve the implementation type of @interfaceObject type
                        let (graph_name, tail_type_name) = (tail_node.graph_name.clone(), tail_node.type_name.clone());
                        let mut errors = LazyErrors::new();
                        errors.add(Rc::new(lazy(move || {
                            Ok(OneOrMany::One(SatisfiabilityError::for_no_implementation(
                                &graph_name,
                                &tail_type_name,
                            )))
                        })));
                        return Ok(PathFinderResult::Failure { errors });
                    }
                }
            }

            // This is a special case where we are looking for an abstract type, but there are no edges leading to it.
            // It's completely fine, as abstract types are not resolvable by themselves and Federation will handle it (return empty result).
            return Ok(PathFinderResult::Success { paths: Vec::new() });
        };

        // In case of no errors, we know that there were no edges matching the field name.
        {
            let (graph_name, type_name, field_name) =
                (tail_node.graph_name.clone(), type_name.to_string(), field_name.to_string());
            errors.add(Rc::new(lazy(move || {
                Ok(OneOrMany::One(SatisfiabilityError::for_missing_field(
                    &graph_name,
                    &type_name,
                    &field_name,
                )))
            })));
        }

        {
            let finder = self.clone();
            let (tail_graph_name, type_name, field_name) =
                (tail_node.graph_name.clone(), type_name.to_string(), field_name.to_string());
            errors.add(Rc::new(lazy(move || {
                let graph = &*finder.graph;
                let mut errors: Vec<SatisfiabilityError> = Vec::new();
                // find graphs with the same type and field name, but no @key defined
                let type_nodes = graph.nodes_of(&type_name, true)?;
                for type_node in type_nodes {
                    let edges = graph.field_edges_of_head(type_node, &field_name)?;
                    for edge in edges {
                        let e = graph.edge(edge);
                        let Move::Field(field_move) = &e.r#move else {
                            continue;
                        };
                        if
                        // edge.move.typeName === typeName &&
                        field_move.field_name == field_name && !finder.move_validator.is_external(edge) {
                            let head = graph.node(e.head);
                            let resolvable_keys = match head.type_state {
                                Some(NodeTypeState::Object(type_state)) => type_state
                                    .by_graph
                                    .get(&head.graph_id)
                                    .map(|g| g.keys.iter().filter(|key| key.resolvable).count())
                                    .unwrap_or(0),
                                _ => 0,
                            };

                            if resolvable_keys == 0 {
                                errors.push(SatisfiabilityError::for_no_key(
                                    &tail_graph_name,
                                    &graph.node(e.tail).graph_name,
                                    &type_name,
                                    &field_name,
                                ));
                            }
                        }
                    }
                }

                Ok(OneOrMany::Many(errors))
            })));
        }

        Ok(PathFinderResult::Failure { errors })
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
        let graph = &*self.graph;
        let e = graph.edge(edge);

        if matches!(e.r#move, Move::Field(_)) {
            self.logger.group_end("Ignored");
            return Ok(());
        }

        let key_fields = key_fields_of(&e.r#move);
        let tail = graph.node(e.tail);

        if resolved_graphs.contains(&tail.graph_name) {
            self.logger.group_end("Ignore: already resolved this graph");
            return Ok(());
        }

        // We can skip the edge in case the the graph interface implementation does not provide the field.
        if let Some(NodeTypeState::Interface(type_state)) = tail.type_state {
            let field_in_edge = type_state
                .fields
                .get(field_name)
                .and_then(|f| f.by_graph.get(&tail.graph_id));

            if field_in_edge.is_none() {
                self.logger
                    .group_end("Ignore edge: tail type does not have this field");
                return Ok(());
            }
        }

        // A huge win for performance, is when you do less work :D
        // We can ignore an edge that has already been visited with the same key fields / requirements.
        // The way entity-move edges are created, where every graph points to every other graph:
        //  Graph A: User @key(id) @key(name)
        //  Graph B: User @key(id)
        //  Edges in a merged graph:
        //    - User/A @key(id) -> User/B
        //    - User/B @key(id) -> User/A
        //    - User/B @key(name) -> User/A
        // Allows us to ignore an edge with the same key fields.
        // That's because in some other path, we will or already have checked the other edge.
        if let Some(key_fields) = &key_fields {
            if visited_fields.iter().any(|f| f.equals(key_fields)) {
                self.logger.group_end("Ignore: already visited fields");
                return Ok(());
            }
        }

        if is_abstract_edge(e) {
            // prevent a situation where we are doing a second abstract move
            if let Some(tail_edge) = path.edge() {
                if is_abstract_edge(graph.edge(tail_edge)) && key_fields.is_none() {
                    self.logger
                        .group_end("Ignore: cannot do two abstract moves in a row");
                    return Ok(());
                }
            }

            if !graph.is_cross_graph_edge(edge) {
                let new_path = path.moved(graph, edge);
                self.logger.log("Abstract move");
                self.logger
                    .group_end(|| format!("Adding to queue: {}", new_path.to_string(graph)));
                queue.push((visited_graphs.to_vec(), visited_fields.to_vec(), new_path));
                return Ok(());
            }
        }

        let mut new_visited_edges = visited_edges.to_vec();
        new_visited_edges.push(edge);
        let resolvable = self.move_validator.is_edge_resolvable(
            edge,
            path,
            &new_visited_edges,
            visited_graphs,
            visited_fields,
            label_values,
        )?;

        if let EdgeResolvabilityResult::Failure { error } = resolvable {
            errors.add(error);
            self.logger.group_end("Not resolvable");
            return Ok(());
        }

        let new_path = path.moved(graph, edge);

        self.logger.log(|| {
            format!(
                "From indirect path, look for direct paths to {type_name}.{field_name} from: {}",
                graph.edge_to_string(edge)
            )
        });
        let direct = self.find_direct_paths(&new_path, type_name, Some(field_name), &[edge], label_values)?;

        let new_path_tail_graph_name = graph.node(graph.edge(edge).tail).graph_name.clone();

        match direct {
            PathFinderResult::Success { paths } => {
                self.logger.group_end(|| {
                    format!(
                        "Resolvable: {} with {} paths",
                        graph.edge_to_string(edge),
                        paths.len()
                    )
                });

                final_paths.extend(paths);
                resolved_graphs.push(new_path_tail_graph_name);
                return Ok(());
            }
            PathFinderResult::Failure { errors: direct_errors } => {
                errors.add_all(&direct_errors);
            }
        }

        resolved_graphs.push(new_path_tail_graph_name);

        self.logger.log("Did not find direct paths");
        self.logger
            .group_end(|| format!("Adding to queue: {}", new_path.to_string(graph)));
        queue.push((
            concat_if_not_exists_string(visited_graphs, &tail.graph_name),
            match &key_fields {
                Some(key_fields) => concat_if_not_exists_fields(visited_fields, key_fields),
                None => visited_fields.to_vec(),
            },
            new_path,
        ));

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn find_type_indirectly(
        &self,
        path: &OperationPath,
        type_name: &str,
        visited_graphs: &[String],
        visited_fields: &[Rc<Selection>],
        _label_values: &OverrideLabels,
        final_paths: &mut Vec<OperationPath>,
        queue: &mut Vec<IndirectPathsQueueItem>,
        resolved_graphs: &mut Vec<String>,
        edge: EdgeId,
    ) {
        let graph = &*self.graph;
        let e = graph.edge(edge);

        if !is_abstract_edge(e) {
            self.logger.group_end("Ignored");
            return;
        }

        let tail = graph.node(e.tail);

        if resolved_graphs.contains(&tail.graph_name) {
            self.logger.group_end("Already resolved the graph");
            return;
        }

        let key_fields = key_fields_of(&e.r#move);

        if let Some(key_fields) = &key_fields {
            if visited_fields.iter().any(|f| f.equals(key_fields)) {
                self.logger.group_end("Ignore: already visited fields");
                return;
            }
        }

        let new_path = path.moved(graph, edge);

        // If the target is the tail of this edge, we have found a path
        if tail.type_name == type_name {
            resolved_graphs.push(tail.graph_name.clone());
            final_paths.push(new_path);
        } else {
            // Otherwise, we need to continue searching for the target
            queue.push((
                visited_graphs.to_vec(),
                match &key_fields {
                    Some(key_fields) => concat_if_not_exists_fields(visited_fields, key_fields),
                    None => visited_fields.to_vec(),
                },
                new_path,
            ));
        }
        self.logger.group_end("Resolvable");
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
        let graph = &*self.graph;
        let mut errors = LazyErrors::new();
        let tail = path.tail_or_root(graph);
        let source_graph_name = graph.node(tail).graph_name.clone();
        let id = match field_name {
            Some(field_name) => format!("{type_name}.{field_name}"),
            None => format!("... on {type_name}"),
        };

        self.logger
            .group(|| format!("Indirect paths to {id} from: {}", graph.node(tail).to_string()));

        let mut queue: Vec<IndirectPathsQueueItem> = vec![(visited_graphs.to_vec(), visited_fields.to_vec(), path.clone_path())];
        let mut final_paths: Vec<OperationPath> = Vec::new();
        let mut resolved_graphs: Vec<String> = Vec::new();

        while let Some((visited_graphs, visited_fields, path)) = queue.pop() {
            let tail = path.tail_or_root(graph);
            let edges = graph.indirect_edges_of_head(tail)?;

            self.logger.log(|| format!("At path: {}", path.to_string(graph)));
            self.logger.log(|| format!("Checking {} edges", edges.len()));
            for (i, &edge) in edges.iter().enumerate() {
                self.logger
                    .group(|| format!("Checking #{i} {}", graph.edge_to_string(edge)));
                self.logger
                    .log(|| format!("Visited graphs: {}", visited_graphs.join(",")));

                let e = graph.edge(edge);
                let edge_tail_graph_name = &graph.node(e.tail).graph_name;

                if visited_graphs.contains(edge_tail_graph_name) {
                    self.logger.group_end("Ignore: already visited graph");
                    continue;
                }

                if visited_edges.contains(&edge) {
                    self.logger.group_end("Ignore: already visited edge");
                    continue;
                }

                if *edge_tail_graph_name == source_graph_name && !is_abstract_edge(e) {
                    // Prevent a situation where we are going back to the same graph
                    // The only exception is when we are moving to an abstract type
                    self.logger.group_end("Ignore: we are back to the same graph");
                    continue;
                }

                if let Some(field_name) = field_name {
                    self.find_field_indirectly(
                        &path,
                        type_name,
                        field_name,
                        visited_edges,
                        &visited_graphs,
                        &visited_fields,
                        label_values,
                        &mut errors,
                        &mut final_paths,
                        &mut queue,
                        &mut resolved_graphs,
                        edge,
                    )?;
                } else {
                    self.find_type_indirectly(
                        &path,
                        type_name,
                        &visited_graphs,
                        &visited_fields,
                        label_values,
                        &mut final_paths,
                        &mut queue,
                        &mut resolved_graphs,
                        edge,
                    );
                }
            }
        }

        self.logger
            .group_end(|| format!("Found {} indirect paths", final_paths.len()));

        if final_paths.is_empty() {
            return Ok(PathFinderResult::Failure { errors });
        }

        Ok(PathFinderResult::Success { paths: final_paths })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_if_not_exists_string_keeps_order() {
        let list = vec!["a".to_string(), "b".to_string()];
        assert_eq!(concat_if_not_exists_string(&list, "b"), list);
        assert_eq!(concat_if_not_exists_string(&list, "c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn concat_if_not_exists_fields_uses_equals() {
        let a = Rc::new(Selection::new("User".into(), "id".into(), vec![]));
        let same_source = Rc::new(Selection::new("User".into(), "id".into(), vec![]));
        let other_type = Rc::new(Selection::new("Post".into(), "id".into(), vec![]));
        let list = vec![a];
        assert_eq!(concat_if_not_exists_fields(&list, &same_source).len(), 1);
        assert_eq!(concat_if_not_exists_fields(&list, &other_type).len(), 2);
    }
}
