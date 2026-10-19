//! Port of `src/supergraph/composition/input-object-type.ts`.

use graphql_js::Definition;
use crate::utils::collections::{IndexMap, IndexSet};

use super::ast::{create_input_object_type_node, CostAst, InputFieldAst, InputObjectTypeAst, JoinAst, JoinFieldAst, JoinTypeAst};
use super::common::{convert_to_const, get_or_insert_with, ComposeHelpers, Graph, MapByGraph, TypeBuilder};
use crate::utils::helpers::{ensure_value, math_max};
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
        let input_object_type_state = get_or_create_input_object_type(state, type_name);

        for tag in &ty.tags {
            input_object_type_state.tags.insert(tag.clone());
        }

        if ty.inaccessible {
            input_object_type_state.inaccessible = true;
        }

        if ty.description.is_some() && input_object_type_state.description.is_none() {
            input_object_type_state.description = ty.description.clone();
        }

        if ty.is_definition {
            input_object_type_state.has_definition = true;
        }

        if ty.is_one_of == Some(true) {
            input_object_type_state.is_one_of = Some(true);
        }

        // `if (type.ast.directives)`: an array is always truthy.
        for directive in &ty.ast.directives {
            input_object_type_state.ast.directives.push(directive.clone());
        }

        input_object_type_state.by_graph.insert(
            graph.id.clone(),
            InputObjectTypeStateInGraph {
                inaccessible: ty.inaccessible,
                version: graph.version,
            },
        );

        // PERF: a new type state gets room for the fields up front (only the capacity changes).
        if input_object_type_state.fields.is_empty() {
            input_object_type_state.fields.reserve(ty.fields.len());
        }

        for field in ty.fields.values() {
            let field_state = get_or_create_field(input_object_type_state, &field.name, &field.r#type, field.kind);

            for tag in &field.tags {
                field_state.tags.insert(tag.clone());
            }

            if field.r#type.ends_with('!') && !field_state.r#type.ends_with('!') {
                // Replace the nullable type with a non-nullable type
                field_state.r#type = field.r#type.clone();
            }

            if field.inaccessible {
                field_state.inaccessible = true;
            }

            if let Some(cost) = field.cost {
                field_state.cost = Some(math_max(cost, field_state.cost));
            }

            if field.description.is_some() && field_state.description.is_none() {
                field_state.description = field.description.clone();
            }

            // First deprecation wins
            if field.deprecated.is_some() && field_state.deprecated.is_none() {
                field_state.deprecated = field.deprecated.clone();
            }

            if field.default_value.is_some() {
                field_state.default_value = field.default_value.clone();
            }

            field_state.kind = field.kind;

            for directive in &field.ast.directives {
                field_state.ast.directives.push(directive.clone());
            }

            field_state.by_graph.insert(
                graph.id.clone(),
                InputObjectFieldStateInGraph {
                    r#type: field.r#type.clone(),
                    inaccessible: field.inaccessible,
                    default_value: field.default_value.clone(),
                    version: graph.version,
                },
            );
        }
    }

    fn compose_supergraph_node(
        &mut self,
        input_object_type: &InputObjectTypeState,
        _graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        let supergraph_state = helpers.supergraph_state;
        let mut fields: Vec<InputFieldAst> = Vec::new();
        for field in input_object_type
            .fields
            .values()
            .filter(|field| field.by_graph.len() == input_object_type.by_graph.len())
        {
            let field_state_in_graphs: Vec<&InputObjectFieldStateInGraph> = field.by_graph.values().collect();
            let has_different_type = field_state_in_graphs.iter().any(|f| f.r#type != field.r#type);

            fields.push(InputFieldAst {
                name: field.name.clone(),
                r#type: field.r#type.clone(),
                tags: field.tags.iter().cloned().collect(),
                inaccessible: field.inaccessible,
                cost: match field.cost {
                    Some(cost) => Some(CostAst {
                        cost,
                        directive_name: ensure_value(
                            supergraph_state.specs.cost.names.cost.clone(),
                            "Directive name of @cost is not defined",
                        )?,
                    }),
                    None => None,
                },
                default_value: if field_state_in_graphs.iter().all(|f| f.default_value.is_some()) {
                    field.default_value.clone()
                } else {
                    None
                },
                description: field.description.clone(),
                deprecated: field.deprecated.clone(),
                ast: convert_to_const(&field.ast.directives),
                join: JoinAst {
                    field: if has_different_type {
                        field
                            .by_graph
                            .iter()
                            .map(|(graph, field_by_graph)| JoinFieldAst {
                                graph: Some(graph.clone()),
                                r#type: Some(field_by_graph.r#type.clone()),
                                ..Default::default()
                            })
                            .collect()
                    } else {
                        vec![]
                    },
                    ..Default::default()
                },
            });
        }

        Ok(Definition::InputObjectTypeDefinition(create_input_object_type_node(InputObjectTypeAst {
            name: input_object_type.name.clone(),
            tags: input_object_type.tags.iter().cloned().collect(),
            inaccessible: input_object_type.inaccessible,
            description: input_object_type.description.clone(),
            is_one_of: input_object_type.is_one_of == Some(true),
            ast: convert_to_const(&input_object_type.ast.directives),
            fields,
            join: JoinAst {
                r#type: input_object_type
                    .by_graph
                    .keys()
                    .map(|graph| JoinTypeAst {
                        graph: graph.clone(),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            },
        })))
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
    get_or_insert_with(state, type_name, || InputObjectTypeState {
        name: type_name.to_string(),
        tags: IndexSet::default(),
        has_definition: false,
        inaccessible: false,
        description: None,
        is_one_of: None,
        by_graph: IndexMap::default(),
        fields: IndexMap::default(),
        ast: Ast { directives: vec![] },
    })
}

fn get_or_create_field<'s>(
    object_type_state: &'s mut InputObjectTypeState,
    field_name: &str,
    field_type: &str,
    field_kind: ArgumentKind,
) -> &'s mut InputObjectTypeFieldState {
    get_or_insert_with(&mut object_type_state.fields, field_name, || InputObjectTypeFieldState {
        name: field_name.to_string(),
        r#type: field_type.to_string(),
        kind: field_kind,
        tags: IndexSet::default(),
        inaccessible: false,
        cost: None,
        default_value: None,
        description: None,
        deprecated: None,
        by_graph: IndexMap::default(),
        ast: Ast { directives: vec![] },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subgraph::state::InputField;
    use crate::supergraph::state::SupergraphState;

    fn graph(id: &str) -> Graph {
        Graph {
            id: id.into(),
            name: id.to_lowercase(),
            version: FederationVersion::V2_3,
            url: None,
        }
    }

    fn field(name: &str, ty: &str, default_value: Option<&str>) -> InputField {
        InputField {
            name: name.into(),
            r#type: ty.into(),
            kind: ArgumentKind::Scalar,
            inaccessible: false,
            tags: IndexSet::default(),
            cost: None,
            default_value: default_value.map(String::from),
            description: None,
            deprecated: None,
            ast: Ast::default(),
        }
    }

    fn input(fields: Vec<InputField>, is_one_of: Option<bool>) -> InputObjectType {
        InputObjectType {
            name: "I".into(),
            fields: fields.into_iter().map(|f| (f.name.clone(), f)).collect(),
            extension: false,
            inaccessible: false,
            tags: IndexSet::default(),
            is_definition: true,
            description: None,
            ast: Ast::default(),
            is_one_of,
        }
    }

    #[test]
    fn keeps_fields_defined_everywhere() {
        let mut builder = input_object_type_builder();
        let mut state = IndexMap::default();
        builder.visit_subgraph_state(
            &graph("A"),
            &mut state,
            "I",
            &input(
                vec![
                    field("a", "Int", Some("1")),
                    field("b", "String", Some("\"x\"")),
                    field("onlyA", "Int", None),
                ],
                None,
            ),
        );
        let mut deprecated = field("b", "String!", None);
        deprecated.deprecated = Some(Deprecated { reason: None });
        builder.visit_subgraph_state(
            &graph("B"),
            &mut state,
            "I",
            &input(vec![field("a", "Int", Some("1")), deprecated], Some(true)),
        );

        let input_state = &state["I"];
        // The non-null type replaces the nullable one.
        assert_eq!(input_state.fields["b"].r#type, "String!");
        assert_eq!(input_state.is_one_of, Some(true));

        let supergraph_state = SupergraphState::default();
        let map = IndexMap::default();
        let node = builder
            .compose_supergraph_node(
                input_state,
                &supergraph_state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &supergraph_state,
                    graph_name_to_id_map: &map,
                },
            )
            .unwrap();
        assert_eq!(
            graphql_js::print(&node),
            "input I @join__type(graph: A) @join__type(graph: B) @oneOf {\n  a: Int = 1\n  \
             b: String! @join__field(graph: A, type: \"String\") @join__field(graph: B, type: \"String!\") @deprecated\n}"
        );
    }

    #[test]
    fn missing_cost_directive_name_throws() {
        let mut builder = input_object_type_builder();
        let mut state = IndexMap::default();
        let mut f = field("a", "Int", None);
        f.cost = Some(2.0);
        builder.visit_subgraph_state(&graph("A"), &mut state, "I", &input(vec![f], None));
        assert_eq!(state["I"].fields["a"].cost, Some(2.0));

        let supergraph_state = SupergraphState::default();
        let map = IndexMap::default();
        let err = builder
            .compose_supergraph_node(
                &state["I"],
                &supergraph_state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &supergraph_state,
                    graph_name_to_id_map: &map,
                },
            )
            .unwrap_err();
        assert_eq!(err.to_js_string(), "Error: Directive name of @cost is not defined");
    }
}
