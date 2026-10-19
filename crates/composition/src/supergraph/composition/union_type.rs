//! Port of `src/supergraph/composition/union-type.ts`.

use graphql_js::{Definition, Directive as DirectiveNode};
use indexmap::{IndexMap, IndexSet};

use super::common::{ComposeHelpers, Graph, MapByGraph, TypeBuilder};
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
        todo!()
    }

    fn compose_supergraph_node(
        &mut self,
        union_type: &UnionTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        todo!()
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
    todo!()
}
