//! Port of `src/supergraph/composition/enum-type.ts`.

use graphql_js::Definition;
use crate::utils::collections::{IndexMap, IndexSet};

use super::ast::{create_enum_type_node, CostAst, EnumTypeAst, EnumValueAst, JoinAst, JoinEnumValueAst, JoinTypeAst};
use super::common::{convert_to_const, get_or_insert_with, ComposeHelpers, Graph, MapByGraph, TypeBuilder};
use crate::utils::auth::merge_scope_policies;
use crate::utils::helpers::{ensure_value, math_max};
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{Ast, Deprecated, Description, EnumType};
use crate::supergraph::state::SubgraphStates;
use crate::utils::js::JsResult;

/// `enumTypeBuilder()`.
#[derive(Clone, Debug, Default)]
pub struct EnumTypeBuilder;

/// `enumTypeBuilder()`.
pub fn enum_type_builder() -> EnumTypeBuilder {
    EnumTypeBuilder
}

impl TypeBuilder<EnumType, EnumTypeState> for EnumTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, EnumTypeState>,
        type_name: &str,
        ty: &EnumType,
    ) {
        let enum_type_state = get_or_create_enum_type(state, type_name);

        for tag in &ty.tags {
            enum_type_state.tags.insert(tag.clone());
        }

        if ty.inaccessible {
            enum_type_state.inaccessible = true;
        }

        if ty.authenticated {
            enum_type_state.authenticated = true;
        }

        // `if (type.policies)`: an array is always truthy.
        enum_type_state.policies = merge_scope_policies(&enum_type_state.policies, &ty.policies);

        enum_type_state.scopes = merge_scope_policies(&enum_type_state.scopes, &ty.scopes);

        if let Some(cost) = ty.cost {
            enum_type_state.cost = Some(math_max(cost, enum_type_state.cost));
        }

        if ty.is_definition {
            enum_type_state.has_definition = true;
        }

        // First description wins
        if ty.description.is_some() && enum_type_state.description.is_none() {
            enum_type_state.description = ty.description.clone();
        }

        if ty.referenced_by_input_type {
            enum_type_state.referenced_by_input_type = true;

            for r in &ty.input_type_references {
                enum_type_state.input_type_references.insert(r.clone());
            }
        }

        if ty.referenced_by_output_type {
            enum_type_state.referenced_by_output_type = true;

            for r in &ty.output_type_references {
                enum_type_state.output_type_references.insert(r.clone());
            }
        }

        for directive in &ty.ast.directives {
            enum_type_state.ast.directives.push(directive.clone());
        }

        enum_type_state.by_graph.insert(
            graph.id.clone(),
            EnumTypeStateInGraph {
                inaccessible: ty.inaccessible,
                version: graph.version,
            },
        );

        for value in ty.values.values() {
            let value_state = get_or_create_enum_value(enum_type_state, &value.name);

            for tag in &value.tags {
                value_state.tags.insert(tag.clone());
            }

            if value.inaccessible {
                value_state.inaccessible = true;
            }

            // First deprecation wins
            if value.deprecated.is_some() && value_state.deprecated.is_none() {
                value_state.deprecated = value.deprecated.clone();
            }

            // First description wins
            if value.description.is_some() && value_state.description.is_none() {
                value_state.description = value.description.clone();
            }

            for directive in &value.ast.directives {
                value_state.ast.directives.push(directive.clone());
            }

            value_state.by_graph.insert(
                graph.id.clone(),
                EnumValueStateInGraph {
                    inaccessible: value.inaccessible,
                    version: graph.version,
                },
            );
        }
    }

    fn compose_supergraph_node(
        &mut self,
        enum_type: &EnumTypeState,
        _graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        let supergraph_state = helpers.supergraph_state;
        let merge_method =
            decide_on_enum_merge_strategy(enum_type.referenced_by_input_type, enum_type.referenced_by_output_type);

        let values: Vec<(&String, &EnumValueState)> = if merge_method == EnumMergeStrategy::Intersection {
            intersection_of_enum_values(enum_type)
        } else {
            enum_type.values.iter().collect()
        };

        // intersection -> get values defined in all graphs
        // union -> get values defined in any graph
        // equal -> values are equal across all graphs

        // NOTE(port): JS `toUpperCase()` is `str::to_uppercase()` (both use the full Unicode case
        // mapping); graph ids are already upper-case ASCII, so this is a no-op in practice. The
        // same holds in `scalar_type.rs` and `union_type.rs`.

        let cost = match enum_type.cost {
            Some(cost) => Some(CostAst {
                cost,
                directive_name: ensure_value(
                    supergraph_state.specs.cost.names.cost.clone(),
                    "Directive name of @cost is not defined",
                )?,
            }),
            None => None,
        };

        Ok(Definition::EnumTypeDefinition(create_enum_type_node(EnumTypeAst {
            name: enum_type.name.clone(),
            cost,
            values: values
                .into_iter()
                .map(|(_, value)| EnumValueAst {
                    name: value.name.clone(),
                    join: JoinAst {
                        enum_value: value
                            .by_graph
                            .keys()
                            .map(|graph| JoinEnumValueAst {
                                graph: graph.to_uppercase(),
                            })
                            .collect(),
                        ..Default::default()
                    },
                    tags: value.tags.iter().cloned().collect(),
                    inaccessible: value.inaccessible,
                    description: value.description.clone(),
                    deprecated: value.deprecated.clone(),
                    ast: convert_to_const(&value.ast.directives),
                })
                .collect(),
            tags: enum_type.tags.iter().cloned().collect(),
            inaccessible: enum_type.inaccessible,
            authenticated: enum_type.authenticated,
            policies: enum_type.policies.clone(),
            scopes: enum_type.scopes.clone(),
            description: enum_type.description.clone(),
            join: JoinAst {
                r#type: enum_type
                    .by_graph
                    .keys()
                    .map(|graph_name| JoinTypeAst {
                        graph: graph_name.to_uppercase(),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            },
            ast: convert_to_const(&enum_type.ast.directives),
        })))
    }
}

/// The result of `decideOnEnumMergeStrategy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumMergeStrategy {
    /// `"equal"`.
    Equal,
    /// `"intersection"`.
    Intersection,
    /// `"union"`.
    Union,
}

fn decide_on_enum_merge_strategy(referenced_by_input_type: bool, referenced_by_output_type: bool) -> EnumMergeStrategy {
    if referenced_by_input_type == referenced_by_output_type {
        return EnumMergeStrategy::Equal;
    }
    if referenced_by_input_type {
        return EnumMergeStrategy::Intersection;
    }
    if referenced_by_output_type {
        return EnumMergeStrategy::Union;
    }
    EnumMergeStrategy::Equal
}

/// `intersectionOfEnumValues(enumType)`: the values defined in every graph of the enum.
fn intersection_of_enum_values(enum_type: &EnumTypeState) -> Vec<(&String, &EnumValueState)> {
    let number_of_graphs = enum_type.by_graph.len();
    enum_type
        .values
        .iter()
        .filter(|(_, value)| value.by_graph.len() == number_of_graphs)
        .collect()
}

/// `EnumTypeState` (`kind: "enum"`).
#[derive(Clone, Debug, PartialEq)]
pub struct EnumTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub has_definition: bool,
    pub description: Option<Description>,
    pub by_graph: MapByGraph<EnumTypeStateInGraph>,
    /// Used only by input types: intersection of values from subgraphs.
    pub referenced_by_input_type: bool,
    /// Used only by output types: union of values from subgraphs.
    pub referenced_by_output_type: bool,
    pub input_type_references: IndexSet<String>,
    pub output_type_references: IndexSet<String>,
    pub values: IndexMap<String, EnumValueState>,
    pub ast: Ast,
}

/// `EnumValueState`.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumValueState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub deprecated: Option<Deprecated>,
    pub description: Option<Description>,
    pub ast: Ast,
    pub by_graph: MapByGraph<EnumValueStateInGraph>,
}

/// `EnumTypeStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumTypeStateInGraph {
    pub inaccessible: bool,
    pub version: FederationVersion,
}

/// `EnumValueStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumValueStateInGraph {
    pub inaccessible: bool,
    pub version: FederationVersion,
}

fn get_or_create_enum_type<'s>(state: &'s mut IndexMap<String, EnumTypeState>, type_name: &str) -> &'s mut EnumTypeState {
    get_or_insert_with(state, type_name, || EnumTypeState {
        name: type_name.to_string(),
        values: IndexMap::default(),
        tags: IndexSet::default(),
        has_definition: false,
        inaccessible: false,
        authenticated: false,
        policies: vec![],
        scopes: vec![],
        cost: None,
        description: None,
        referenced_by_input_type: false,
        referenced_by_output_type: false,
        input_type_references: IndexSet::default(),
        output_type_references: IndexSet::default(),
        by_graph: IndexMap::default(),
        ast: Ast { directives: vec![] },
    })
}

fn get_or_create_enum_value<'s>(enum_type_state: &'s mut EnumTypeState, enum_value_name: &str) -> &'s mut EnumValueState {
    get_or_insert_with(&mut enum_type_state.values, enum_value_name, || EnumValueState {
        name: enum_value_name.to_string(),
        tags: IndexSet::default(),
        inaccessible: false,
        deprecated: None,
        description: None,
        by_graph: IndexMap::default(),
        ast: Ast { directives: vec![] },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supergraph::state::SupergraphState;

    fn value(name: &str, graphs: &[&str]) -> EnumValueState {
        EnumValueState {
            name: name.into(),
            tags: IndexSet::default(),
            inaccessible: false,
            deprecated: None,
            description: None,
            ast: Ast::default(),
            by_graph: graphs
                .iter()
                .map(|g| {
                    (
                        g.to_string(),
                        EnumValueStateInGraph {
                            inaccessible: false,
                            version: FederationVersion::V2_3,
                        },
                    )
                })
                .collect(),
        }
    }

    fn enum_state(referenced_by_input_type: bool, referenced_by_output_type: bool) -> EnumTypeState {
        let mut state = IndexMap::default();
        let enum_state = get_or_create_enum_type(&mut state, "E");
        enum_state.referenced_by_input_type = referenced_by_input_type;
        enum_state.referenced_by_output_type = referenced_by_output_type;
        for g in ["a", "b"] {
            enum_state.by_graph.insert(
                g.into(),
                EnumTypeStateInGraph {
                    inaccessible: false,
                    version: FederationVersion::V2_3,
                },
            );
        }
        enum_state.values.insert("BOTH".into(), value("BOTH", &["a", "b"]));
        enum_state.values.insert("ONLY_B".into(), value("ONLY_B", &["b"]));
        state.shift_remove("E").unwrap()
    }

    fn compose(enum_type: &EnumTypeState, supergraph_state: &SupergraphState) -> JsResult<String> {
        let map = IndexMap::default();
        enum_type_builder()
            .compose_supergraph_node(
                enum_type,
                &supergraph_state.subgraphs,
                ComposeHelpers {
                    supergraph_state,
                    graph_name_to_id_map: &map,
                },
            )
            .map(|node| graphql_js::print(&node))
    }

    #[test]
    fn merge_strategy() {
        assert_eq!(decide_on_enum_merge_strategy(true, true), EnumMergeStrategy::Equal);
        assert_eq!(decide_on_enum_merge_strategy(false, false), EnumMergeStrategy::Equal);
        assert_eq!(decide_on_enum_merge_strategy(true, false), EnumMergeStrategy::Intersection);
        assert_eq!(decide_on_enum_merge_strategy(false, true), EnumMergeStrategy::Union);
    }

    #[test]
    fn input_only_enums_keep_the_intersection() {
        let supergraph_state = SupergraphState::default();
        assert_eq!(
            compose(&enum_state(true, false), &supergraph_state).unwrap(),
            "enum E @join__type(graph: A) @join__type(graph: B) {\n  \
             BOTH @join__enumValue(graph: A) @join__enumValue(graph: B)\n}"
        );
        assert_eq!(
            compose(&enum_state(false, true), &supergraph_state).unwrap(),
            "enum E @join__type(graph: A) @join__type(graph: B) {\n  \
             BOTH @join__enumValue(graph: A) @join__enumValue(graph: B)\n  ONLY_B @join__enumValue(graph: B)\n}"
        );
    }

    #[test]
    fn cost_uses_the_supergraph_directive_name() {
        let mut supergraph_state = SupergraphState::default();
        let mut enum_type = enum_state(false, false);
        enum_type.cost = Some(3.0);
        assert_eq!(
            compose(&enum_type, &supergraph_state).unwrap_err().to_js_string(),
            "Error: Directive name of @cost is not defined"
        );
        supergraph_state.specs.cost.names.cost = Some("myCost".into());
        assert!(compose(&enum_type, &supergraph_state)
            .unwrap()
            .starts_with("enum E @join__type(graph: A) @join__type(graph: B) @myCost(weight: 3) {"));
    }
}
