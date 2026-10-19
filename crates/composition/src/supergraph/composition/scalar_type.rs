//! Port of `src/supergraph/composition/scalar-type.ts`.

use graphql_js::Definition;
use crate::utils::collections::{IndexMap, IndexSet};

use super::ast::{create_scalar_type_node, CostAst, JoinAst, JoinTypeAst, ScalarTypeAst};
use super::common::{convert_to_const, ComposeHelpers, Graph, MapByGraph, TypeBuilder};
use crate::utils::auth::merge_scope_policies;
use crate::utils::helpers::{ensure_value, math_max};
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
        let scalar_type_state = get_or_create_scalar_type(state, type_name);

        for tag in &ty.tags {
            scalar_type_state.tags.insert(tag.clone());
        }

        if ty.inaccessible {
            scalar_type_state.inaccessible = true;
        }

        if ty.authenticated {
            scalar_type_state.authenticated = true;
        }

        // `if (type.policies)`: an array is always truthy.
        scalar_type_state.policies = merge_scope_policies(&scalar_type_state.policies, &ty.policies);

        scalar_type_state.scopes = merge_scope_policies(&scalar_type_state.scopes, &ty.scopes);

        if let Some(cost) = ty.cost {
            scalar_type_state.cost = Some(math_max(cost, scalar_type_state.cost));
        }

        if ty.description.is_some() && scalar_type_state.description.is_none() {
            scalar_type_state.description = ty.description.clone();
        }

        // `type.specifiedBy && !scalarTypeState.specifiedBy`: strings, so `""` is falsy.
        if ty.specified_by.as_deref().is_some_and(|s| !s.is_empty())
            && !scalar_type_state.specified_by.as_deref().is_some_and(|s| !s.is_empty())
        {
            scalar_type_state.specified_by = ty.specified_by.clone();
        }

        for directive in &ty.ast.directives {
            scalar_type_state.ast.directives.push(directive.clone());
        }

        scalar_type_state.by_graph.insert(
            graph.id.clone(),
            ScalarTypeStateInGraph {
                inaccessible: ty.inaccessible,
                version: graph.version,
            },
        );
    }

    fn compose_supergraph_node(
        &mut self,
        scalar_type: &ScalarTypeState,
        _graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        let supergraph_state = helpers.supergraph_state;
        let cost = match scalar_type.cost {
            Some(cost) => Some(CostAst {
                cost,
                directive_name: ensure_value(
                    supergraph_state.specs.cost.names.cost.clone(),
                    "Directive name of @cost is not defined",
                )?,
            }),
            None => None,
        };

        Ok(Definition::ScalarTypeDefinition(create_scalar_type_node(&ScalarTypeAst {
            name: scalar_type.name.clone(),
            tags: scalar_type.tags.iter().cloned().collect(),
            inaccessible: scalar_type.inaccessible,
            authenticated: scalar_type.authenticated,
            policies: scalar_type.policies.clone(),
            scopes: scalar_type.scopes.clone(),
            cost,
            description: scalar_type.description.clone(),
            specified_by: scalar_type.specified_by.clone(),
            join: JoinAst {
                r#type: scalar_type
                    .by_graph
                    .keys()
                    .map(|graph_name| JoinTypeAst {
                        graph: graph_name.to_uppercase(),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            },
            ast: convert_to_const(&scalar_type.ast.directives),
        })))
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
    state
        .entry(type_name.to_string())
        .or_insert_with(|| ScalarTypeState {
            name: type_name.to_string(),
            tags: IndexSet::default(),
            inaccessible: false,
            authenticated: false,
            policies: vec![],
            scopes: vec![],
            cost: None,
            by_graph: IndexMap::default(),
            description: None,
            specified_by: None,
            ast: Ast { directives: vec![] },
        })
}
