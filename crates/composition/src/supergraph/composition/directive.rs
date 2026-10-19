//! Port of `src/supergraph/composition/directive.ts`.

use graphql_js::{Definition, Directive as DirectiveNode};
use indexmap::{IndexMap, IndexSet};

use super::ast::{create_directive_node, DirectiveAst, FieldArgumentAst};
use super::common::{convert_to_const, ComposeHelpers, Graph, MapByGraph, TypeBuilder};
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{Ast, ArgumentKind, Directive};
use crate::supergraph::state::SubgraphStates;
use crate::utils::js::JsResult;

/// `directiveBuilder()`.
#[derive(Clone, Debug, Default)]
pub struct DirectiveBuilder;

/// `directiveBuilder()`.
pub fn directive_builder() -> DirectiveBuilder {
    DirectiveBuilder
}

impl TypeBuilder<Directive, DirectiveState> for DirectiveBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, DirectiveState>,
        directive_name: &str,
        directive: &Directive,
    ) {
        // Because the visitor is called from leaf to root (it's using "leave"),
        // we can assume that @composeDirective was already visited and set the composed flag.
        // If it's not set, we can skip this directive as it shouldn't be included in the supergraph anyway.
        // If it is an executable directive, we need to add it as well, but later on check whether it should be included in the supergraph or not (See the `createSupergraphStateBuilder.build` method).
        let is_executable = directive.is_executable();
        if !directive.composed && !is_executable {
            return;
        }

        let directive_state = get_or_create_directive(state, directive_name);

        for location in &directive.locations {
            directive_state.locations.insert(location.clone());
        }

        if directive.repeatable {
            directive_state.repeatable = true;
        }

        if is_executable {
            directive_state.is_executable = true;
        }

        if directive.composed {
            directive_state.composed = true;
        }

        for arg in directive.args.values() {
            let arg_state = get_or_create_arg(directive_state, &arg.name, &arg.r#type, arg.kind);

            for tag in &arg.tags {
                arg_state.tags.insert(tag.clone());
            }

            if arg.r#type.ends_with('!') {
                arg_state.r#type = arg.r#type.clone();
            }

            for directive in &arg.ast.directives {
                arg_state.ast.directives.push(directive.clone());
            }

            if arg.default_value.is_some() {
                arg_state.default_value = arg.default_value.clone();
            }

            if arg.inaccessible {
                arg_state.inaccessible = true;
            }

            arg_state.kind = arg.kind;

            arg_state.by_graph.insert(
                graph.id.clone(),
                ArgStateInGraph {
                    r#type: arg.r#type.clone(),
                    kind: arg.kind,
                    default_value: arg.default_value.clone(),
                    version: graph.version,
                },
            );
        }

        // NOTE(port): the TS shares the subgraph's `locations` set; nothing mutates it later.
        directive_state.by_graph.insert(
            graph.id.clone(),
            DirectiveStateInGraph {
                locations: directive.locations.clone(),
                repeatable: directive.repeatable,
                version: graph.version,
            },
        );
    }

    fn compose_supergraph_node(
        &mut self,
        directive: &DirectiveState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        Ok(Definition::DirectiveDefinition(create_directive_node(&DirectiveAst {
            name: directive.name.clone(),
            locations: directive.locations.iter().cloned().collect(),
            repeatable: directive.repeatable,
            arguments: directive
                .args
                .values()
                .map(|arg| FieldArgumentAst {
                    name: arg.name.clone(),
                    r#type: arg.r#type.clone(),
                    kind: arg.kind,
                    cost: None,
                    tags: arg.tags.iter().cloned().collect(),
                    inaccessible: arg.inaccessible,
                    default_value: arg.default_value.clone(),
                    ast: convert_to_const(&arg.ast.directives),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        })))
    }
}

/// `DirectiveState` (`kind: "directive"`).
#[derive(Clone, Debug, PartialEq)]
pub struct DirectiveState {
    pub name: String,
    pub by_graph: MapByGraph<DirectiveStateInGraph>,
    pub is_executable: bool,
    pub composed: bool,
    pub locations: IndexSet<String>,
    pub repeatable: bool,
    pub args: IndexMap<String, DirectiveArgState>,
}

/// `DirectiveStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectiveStateInGraph {
    pub locations: IndexSet<String>,
    pub repeatable: bool,
    pub version: FederationVersion,
}

/// `DirectiveArgState`.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectiveArgState {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub default_value: Option<String>,
    pub by_graph: MapByGraph<ArgStateInGraph>,
    pub ast: Ast,
}

/// `ArgStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct ArgStateInGraph {
    pub r#type: String,
    pub kind: ArgumentKind,
    pub default_value: Option<String>,
    pub version: FederationVersion,
}

fn get_or_create_directive<'s>(state: &'s mut IndexMap<String, DirectiveState>, directive_name: &str) -> &'s mut DirectiveState {
    state
        .entry(directive_name.to_string())
        .or_insert_with(|| DirectiveState {
            name: directive_name.to_string(),
            locations: IndexSet::new(),
            by_graph: IndexMap::new(),
            args: IndexMap::new(),
            repeatable: false,
            is_executable: false,
            composed: false,
        })
}

fn get_or_create_arg<'s>(
    directive_state: &'s mut DirectiveState,
    arg_name: &str,
    arg_type: &str,
    arg_kind: ArgumentKind,
) -> &'s mut DirectiveArgState {
    directive_state
        .args
        .entry(arg_name.to_string())
        .or_insert_with(|| DirectiveArgState {
            name: arg_name.to_string(),
            r#type: arg_type.to_string(),
            kind: arg_kind,
            inaccessible: false,
            tags: IndexSet::new(),
            default_value: None,
            by_graph: IndexMap::new(),
            ast: Ast { directives: vec![] },
        })
}

/// `isArgumentTypeIdenticalInEverySubgraph(directiveArgState, totalSubgraphSize)`.
pub fn is_argument_type_identical_in_every_subgraph(directive_arg_state: &DirectiveArgState, total_subgraph_size: usize) -> bool {
    if directive_arg_state.by_graph.len() != total_subgraph_size {
        return false;
    }
    directive_arg_state
        .by_graph
        .values()
        .all(|arg| arg.r#type == directive_arg_state.r#type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subgraph::state::Argument;
    use crate::supergraph::state::SupergraphState;

    fn graph(id: &str) -> Graph {
        Graph {
            id: id.into(),
            name: id.to_lowercase(),
            version: FederationVersion::V2_3,
            url: None,
        }
    }

    fn arg(ty: &str, default_value: Option<&str>) -> Argument {
        Argument {
            name: "a".into(),
            r#type: ty.into(),
            kind: ArgumentKind::Scalar,
            inaccessible: false,
            tags: IndexSet::new(),
            cost: None,
            default_value: default_value.map(String::from),
            from_context: None,
            description: None,
            deprecated: None,
            ast: Ast::default(),
        }
    }

    fn directive(locations: &[&str], composed: bool, arg: Argument) -> Directive {
        Directive {
            name: "d".into(),
            composed,
            repeatable: false,
            locations: locations.iter().map(|l| l.to_string()).collect(),
            args: [("a".to_string(), arg)].into_iter().collect(),
        }
    }

    #[test]
    fn skips_non_executable_non_composed_directives() {
        let mut builder = directive_builder();
        let mut state = IndexMap::new();
        builder.visit_subgraph_state(&graph("A"), &mut state, "d", &directive(&["OBJECT"], false, arg("Int", None)));
        assert!(state.is_empty());
    }

    #[test]
    fn merges_and_composes() {
        let mut builder = directive_builder();
        let mut state = IndexMap::new();
        builder.visit_subgraph_state(
            &graph("A"),
            &mut state,
            "d",
            &directive(&["FIELD", "OBJECT"], false, arg("Int", Some("1"))),
        );
        builder.visit_subgraph_state(&graph("B"), &mut state, "d", &directive(&["QUERY"], true, arg("Int!", None)));

        let d = &state["d"];
        assert!(d.is_executable);
        assert!(d.composed);
        assert_eq!(d.locations.iter().collect::<Vec<_>>(), vec!["FIELD", "OBJECT", "QUERY"]);
        assert_eq!(d.args["a"].r#type, "Int!");
        assert_eq!(d.args["a"].default_value.as_deref(), Some("1"));
        assert!(!is_argument_type_identical_in_every_subgraph(&d.args["a"], 1));
        assert!(!is_argument_type_identical_in_every_subgraph(&d.args["a"], 2));

        let supergraph_state = SupergraphState::default();
        let map = IndexMap::new();
        let node = builder
            .compose_supergraph_node(
                d,
                &supergraph_state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &supergraph_state,
                    graph_name_to_id_map: &map,
                },
            )
            .unwrap();
        assert_eq!(graphql_js::print(&node), "directive @d(a: Int! = 1) on FIELD | OBJECT | QUERY");
    }
}
