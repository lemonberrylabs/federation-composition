//! Port of `src/supergraph/composition/scalar-type.ts`.

use graphql_js::{Definition, Directive as DirectiveNode};
use indexmap::{IndexMap, IndexSet};

use super::common::{ComposeHelpers, Graph, MapByGraph, TypeBuilder};
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{Ast, Description, ScalarType};
use crate::supergraph::state::SubgraphStates;
use crate::utils::js::JsResult;

/// `scalarTypeBuilder()`.
#[derive(Clone, Debug, Default)]
pub struct ScalarTypeBuilder;

/// `scalarTypeBuilder()`.
pub fn scalar_type_builder() -> ScalarTypeBuilder {
    ScalarTypeBuilder
}

impl TypeBuilder<ScalarType, ScalarTypeState> for ScalarTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, ScalarTypeState>,
        type_name: &str,
        ty: &ScalarType,
    ) {
        todo!()
    }

    fn compose_supergraph_node(
        &mut self,
        scalar_type: &ScalarTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        todo!()
    }
}

/// `ScalarTypeState` (`kind: "scalar"`).
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub by_graph: MapByGraph<ScalarTypeStateInGraph>,
    pub description: Option<Description>,
    pub specified_by: Option<String>,
    pub ast: Ast,
}

/// `ScalarTypeStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarTypeStateInGraph {
    pub inaccessible: bool,
    pub version: FederationVersion,
}

fn get_or_create_scalar_type<'s>(state: &'s mut IndexMap<String, ScalarTypeState>, type_name: &str) -> &'s mut ScalarTypeState {
    todo!()
}
