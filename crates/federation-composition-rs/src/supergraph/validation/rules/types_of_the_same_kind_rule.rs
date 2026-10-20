//! Port of `src/supergraph/validation/rules/types-of-the-same-kind-rule.ts`.

use graphql_rs::GraphQLError;
use crate::supergraph::validation::validation_context::SupergraphValidationContext;
use crate::utils::collections::{IndexMap, IndexSet};
use crate::subgraph::state::{SubgraphType, TypeKind};

/// `TypesOfTheSameKindRule(context)`: a pre-supergraph rule (runs before the subgraphs are merged).
pub fn types_of_the_same_kind_rule(context: &mut SupergraphValidationContext) {
    // Map<typeName, Map<kind, Set<graphName>>>
    let mut type_to_kind_with_graphs: IndexMap<String, IndexMap<TypeKind, Vec<GraphTypeValidationContext>>> =
        IndexMap::default();
    let mut types_with_conflict: IndexSet<String> = IndexSet::default();

    for (_graph, state) in &context.subgraph_states {
        for ty in state.types.values() {
            let is_interface_object = match ty {
                SubgraphType::Interface(i) => i.is_interface_object,
                _ => false,
            };

            // `context.graphIdToName(graph)`: `graph` is a key of `subgraphStates`, so this is
            // the name of `state`'s graph and cannot throw.
            let graphs_value = GraphTypeValidationContext {
                graph_name: state.graph.name.clone(),
                is_interface_object,
            };

            if let Some(kind_to_graphs) = type_to_kind_with_graphs.get_mut(ty.name()) {
                // Seems like we've already seen this type
                if let Some(graphs) = kind_to_graphs.get_mut(&ty.kind()) {
                    // If we've already seen this kind
                    // Add the graph to the set.
                    graphs.push(graphs_value);
                } else {
                    // Add the kind to the map of kinds for that type
                    kind_to_graphs.insert(ty.kind(), vec![graphs_value]);
                }

                // If it has more than 1 kind
                if kind_to_graphs.len() > 1 {
                    // Add it to the conflict set
                    types_with_conflict.insert(ty.name().to_string());
                }
            } else {
                // We haven't seen this type yet
                let mut kinds = IndexMap::default();
                kinds.insert(ty.kind(), vec![graphs_value]);
                type_to_kind_with_graphs.insert(ty.name().to_string(), kinds);
            }
        }
    }

    for type_name in &types_with_conflict {
        let kind_to_graphs = &type_to_kind_with_graphs[type_name];

        if interface_object_conditions(kind_to_graphs) {
            continue;
        }

        let groups: Vec<String> = kind_to_graphs
            .iter()
            .map(|(kind, graphs)| {
                let plural = if graphs.len() > 1 { "s" } else { "" };
                format!(
                    "{} Type in subgraph{plural} \"{}\"",
                    map_ir_kind_to_string(*kind),
                    graphs
                        .iter()
                        .map(|c| c.graph_name.as_str())
                        .collect::<Vec<_>>()
                        .join("\", \"")
                )
            })
            .collect();
        let first = &groups[0];
        let second = &groups[1];
        let rest = &groups[2..];

        context.report_error(GraphQLError::with_code(
            format!(
                "Type \"{type_name}\" has mismatched kind: it is defined as {first} but {second}{}",
                if !rest.is_empty() {
                    format!(" and {}", rest.join(" and "))
                } else {
                    String::new()
                }
            ),
            "TYPE_KIND_MISMATCH",
        ));
    }
}

/// `mapIRKindToString[kind]`.
fn map_ir_kind_to_string(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Object => "Object",
        TypeKind::Interface => "Interface",
        TypeKind::Union => "Union",
        TypeKind::Enum => "Enum",
        TypeKind::InputObject => "InputObject",
        TypeKind::Scalar => "Scalar",
        TypeKind::Directive => "Directive",
    }
}

/// `GraphTypeValidationContext`.
/// NOTE(port): the TS keeps these objects in a `Set`, which dedupes by identity (never here,
/// as each is a fresh object); use a `Vec`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphTypeValidationContext {
    pub graph_name: String,
    pub is_interface_object: bool,
}

fn interface_object_conditions(kind_to_graphs: &IndexMap<TypeKind, Vec<GraphTypeValidationContext>>) -> bool {
    let interface_types = kind_to_graphs
        .get(&TypeKind::Interface)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for graph_type_validation_context in interface_types {
        if graph_type_validation_context.is_interface_object {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_object_conditions_needs_an_interface_object() {
        let mut kinds = IndexMap::default();
        kinds.insert(
            TypeKind::Object,
            vec![GraphTypeValidationContext {
                graph_name: "a".into(),
                is_interface_object: false,
            }],
        );
        assert!(!interface_object_conditions(&kinds));
        kinds.insert(
            TypeKind::Interface,
            vec![
                GraphTypeValidationContext {
                    graph_name: "b".into(),
                    is_interface_object: false,
                },
                GraphTypeValidationContext {
                    graph_name: "c".into(),
                    is_interface_object: true,
                },
            ],
        );
        assert!(interface_object_conditions(&kinds));
    }
}
