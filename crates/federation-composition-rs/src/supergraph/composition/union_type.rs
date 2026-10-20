//! Port of `src/supergraph/composition/union-type.ts`.

use graphql_rs::Definition;
use crate::utils::collections::{IndexMap, IndexSet};

use super::ast::{create_union_type_node, JoinAst, JoinTypeAst, JoinUnionMemberAst, UnionTypeAst};
use super::common::{convert_to_const, get_or_insert_with, ComposeHelpers, Graph, MapByGraph, TypeBuilder};
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{Ast, Description, UnionType};
use crate::supergraph::state::SubgraphStates;
use crate::utils::js::JsResult;

/// `unionTypeBuilder()`.
#[derive(Clone, Debug, Default)]
pub struct UnionTypeBuilder;

/// `unionTypeBuilder()`.
pub fn union_type_builder() -> UnionTypeBuilder {
    UnionTypeBuilder
}

impl TypeBuilder<UnionType, UnionTypeState> for UnionTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, UnionTypeState>,
        type_name: &str,
        ty: &UnionType,
    ) {
        let union_type_state = get_or_create_union_type(state, type_name);

        for tag in &ty.tags {
            union_type_state.tags.insert(tag.clone());
        }
        for context_name in &ty.contexts {
            union_type_state.contexts.insert(format!("{}__{}", graph.name, context_name));
        }

        if ty.inaccessible {
            union_type_state.inaccessible = true;
        }

        if ty.is_definition {
            union_type_state.has_definition = true;
        }

        // First description wins
        if ty.description.is_some() && union_type_state.description.is_none() {
            union_type_state.description = ty.description.clone();
        }

        for directive in &ty.ast.directives {
            union_type_state.ast.directives.push(directive.clone());
        }

        // NOTE(port): the TS shares the subgraph's `members` set; nothing mutates it later.
        union_type_state.by_graph.insert(
            graph.id.clone(),
            UnionTypeInGraph {
                members: ty.members.clone(),
                version: graph.version,
            },
        );

        for member in &ty.members {
            union_type_state.members.insert(member.clone());
        }
    }

    fn compose_supergraph_node(
        &mut self,
        union_type: &UnionTypeState,
        _graphs: &SubgraphStates,
        _helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        Ok(Definition::UnionTypeDefinition(create_union_type_node(UnionTypeAst {
            name: union_type.name.clone(),
            members: union_type.members.iter().cloned().collect(),
            tags: union_type.tags.iter().cloned().collect(),
            inaccessible: union_type.inaccessible,
            description: union_type.description.clone(),
            join: JoinAst {
                r#type: union_type
                    .by_graph
                    .keys()
                    .map(|graph_name| JoinTypeAst {
                        graph: graph_name.to_uppercase(),
                        ..Default::default()
                    })
                    .collect(),
                union_member: union_type
                    .by_graph
                    .iter()
                    .flat_map(|(graph_name, meta)| {
                        let graph = graph_name.to_uppercase();
                        meta.members
                            .iter()
                            .map(|member| JoinUnionMemberAst {
                                graph: graph.clone(),
                                member: member.clone(),
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect(),
                ..Default::default()
            },
            ast: convert_to_const(&union_type.ast.directives),
            contexts: union_type.contexts.iter().cloned().collect(),
        })))
    }
}

/// `UnionTypeState` (`kind: "union"`).
#[derive(Clone, Debug, PartialEq)]
pub struct UnionTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub has_definition: bool,
    pub description: Option<Description>,
    pub inaccessible: bool,
    pub contexts: IndexSet<String>,
    pub by_graph: MapByGraph<UnionTypeInGraph>,
    pub members: IndexSet<String>,
    pub ast: Ast,
}

/// `UnionTypeInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct UnionTypeInGraph {
    pub members: IndexSet<String>,
    pub version: FederationVersion,
}

fn get_or_create_union_type<'s>(state: &'s mut IndexMap<String, UnionTypeState>, type_name: &str) -> &'s mut UnionTypeState {
    get_or_insert_with(state, type_name, || UnionTypeState {
        name: type_name.to_string(),
        members: IndexSet::default(),
        tags: IndexSet::default(),
        inaccessible: false,
        contexts: IndexSet::default(),
        has_definition: false,
        description: None,
        by_graph: IndexMap::default(),
        ast: Ast { directives: vec![] },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supergraph::state::SupergraphState;

    fn graph(id: &str, name: &str) -> Graph {
        Graph {
            id: id.into(),
            name: name.into(),
            version: FederationVersion::V2_3,
            url: None,
        }
    }

    fn union(members: &[&str], contexts: &[&str], description: Option<&str>) -> UnionType {
        UnionType {
            name: "U".into(),
            members: members.iter().map(|m| m.to_string()).collect(),
            contexts: contexts.iter().map(|m| m.to_string()).collect(),
            tags: IndexSet::default(),
            inaccessible: false,
            is_definition: true,
            description: description.map(|d| Description {
                value: d.into(),
                block: false,
            }),
            ast: Ast::default(),
        }
    }

    #[test]
    fn merges_members_and_emits_join_directives() {
        let mut builder = union_type_builder();
        let mut state = IndexMap::default();
        builder.visit_subgraph_state(&graph("a", "A"), &mut state, "U", &union(&["X", "Y"], &["ctx"], None));
        builder.visit_subgraph_state(&graph("b", "B"), &mut state, "U", &union(&["Y", "Z"], &[], Some("second")));
        builder.visit_subgraph_state(&graph("c", "C"), &mut state, "U", &union(&[], &[], Some("third")));

        let union_state = &state["U"];
        assert_eq!(union_state.members.iter().collect::<Vec<_>>(), vec!["X", "Y", "Z"]);
        assert_eq!(union_state.contexts.iter().collect::<Vec<_>>(), vec!["A__ctx"]);
        // First description wins.
        assert_eq!(union_state.description.as_ref().unwrap().value, "second");

        let supergraph_state = SupergraphState::default();
        let map = IndexMap::default();
        let node = builder
            .compose_supergraph_node(
                union_state,
                &supergraph_state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &supergraph_state,
                    graph_name_to_id_map: &map,
                },
            )
            .unwrap();
        assert_eq!(
            graphql_rs::print(&node),
            "\"\"\"second\"\"\"\nunion U @join__type(graph: A) @join__type(graph: B) @join__type(graph: C) \
             @join__unionMember(graph: A, member: \"X\") @join__unionMember(graph: A, member: \"Y\") \
             @join__unionMember(graph: B, member: \"Y\") @join__unionMember(graph: B, member: \"Z\") \
             @context(name: \"A__ctx\") = X | Y | Z"
        );
    }
}
