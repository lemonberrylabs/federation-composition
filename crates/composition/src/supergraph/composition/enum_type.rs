//! Port of `src/supergraph/composition/enum-type.ts`.

use graphql_js::{Definition, Directive as DirectiveNode};
use indexmap::{IndexMap, IndexSet};

use super::common::{ComposeHelpers, Graph, MapByGraph, TypeBuilder};
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
        todo!()
    }

    fn compose_supergraph_node(
        &mut self,
        enum_type: &EnumTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        todo!()
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
    todo!()
}

fn get_or_create_enum_value<'s>(enum_type_state: &'s mut EnumTypeState, enum_value_name: &str) -> &'s mut EnumValueState {
    todo!()
}
