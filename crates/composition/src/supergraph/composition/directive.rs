//! Port of `src/supergraph/composition/directive.ts`.

use graphql_js::{Definition, Directive as DirectiveNode};
use indexmap::{IndexMap, IndexSet};

use super::common::{ComposeHelpers, Graph, MapByGraph, TypeBuilder};
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
        todo!()
    }

    fn compose_supergraph_node(
        &mut self,
        directive: &DirectiveState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        todo!()
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
    todo!()
}

fn get_or_create_arg<'s>(
    directive_state: &'s mut DirectiveState,
    arg_name: &str,
    arg_type: &str,
    arg_kind: ArgumentKind,
) -> &'s mut DirectiveArgState {
    todo!()
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
