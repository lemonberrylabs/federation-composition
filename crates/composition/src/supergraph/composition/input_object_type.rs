//! Port of `src/supergraph/composition/input-object-type.ts`.

use graphql_js::{Definition, Directive as DirectiveNode};
use indexmap::{IndexMap, IndexSet};

use super::common::{ComposeHelpers, Graph, MapByGraph, TypeBuilder};
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{Ast, ArgumentKind, Deprecated, Description, InputObjectType};
use crate::supergraph::state::SubgraphStates;
use crate::utils::js::JsResult;

/// `inputObjectTypeBuilder()`.
#[derive(Clone, Debug, Default)]
pub struct InputObjectTypeBuilder;

/// `inputObjectTypeBuilder()`.
pub fn input_object_type_builder() -> InputObjectTypeBuilder {
    InputObjectTypeBuilder
}

impl TypeBuilder<InputObjectType, InputObjectTypeState> for InputObjectTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, InputObjectTypeState>,
        type_name: &str,
        ty: &InputObjectType,
    ) {
        todo!()
    }

    fn compose_supergraph_node(
        &mut self,
        input_object_type: &InputObjectTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        todo!()
    }
}

/// `InputObjectTypeState` (`kind: "input"`).
#[derive(Clone, Debug, PartialEq)]
pub struct InputObjectTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub has_definition: bool,
    pub description: Option<Description>,
    pub is_one_of: Option<bool>,
    pub by_graph: MapByGraph<InputObjectTypeStateInGraph>,
    pub fields: IndexMap<String, InputObjectTypeFieldState>,
    pub ast: Ast,
}

/// `InputObjectTypeFieldState`.
#[derive(Clone, Debug, PartialEq)]
pub struct InputObjectTypeFieldState {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub cost: Option<f64>,
    pub default_value: Option<String>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub by_graph: MapByGraph<InputObjectFieldStateInGraph>,
    pub ast: Ast,
}

/// `InputObjectTypeStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct InputObjectTypeStateInGraph {
    pub inaccessible: bool,
    pub version: FederationVersion,
}

/// `InputObjectFieldStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct InputObjectFieldStateInGraph {
    pub r#type: String,
    pub inaccessible: bool,
    pub default_value: Option<String>,
    pub version: FederationVersion,
}

fn get_or_create_input_object_type<'s>(
    state: &'s mut IndexMap<String, InputObjectTypeState>,
    type_name: &str,
) -> &'s mut InputObjectTypeState {
    todo!()
}

fn get_or_create_field<'s>(
    object_type_state: &'s mut InputObjectTypeState,
    field_name: &str,
    field_type: &str,
    field_kind: ArgumentKind,
) -> &'s mut InputObjectTypeFieldState {
    todo!()
}
