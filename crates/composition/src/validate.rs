//! Port of `src/validate.ts`.

use graphql_js::{Definition, Document, GraphQLError};

use crate::specifications::federation::FederationVersion;
use crate::specifications::link::Link;
use crate::subgraph::validation::validate_subgraph::InternalOptions;
use crate::supergraph::state::SupergraphSpecs;
use crate::utils::js::JsResult;

/// The `subgraph` input of `validateSubgraph`/`validate`: `{ name, url?, typeDefs }`.
pub use crate::types::ServiceDefinition as SubgraphInputDefinition;

/// `numberAtStartRegex = /^\d/`.
fn starts_with_number(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

/// An entry of `buildGraphList`'s `graphs`: `{ id, name, typeDefs, url? }`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphListEntry {
    pub id: String,
    pub name: String,
    pub type_defs: Document,
    pub url: Option<String>,
}

/// The result of `buildGraphList`: `{ success: true, graphs }` or `{ success: false, errors }`.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphList {
    Success { graphs: Vec<GraphListEntry> },
    Failure { errors: Vec<GraphQLError> },
}

/// `buildGraphList(subgraphs)`: assigns ids and sorts by id (`localeCompare`). Throws
/// `A subgraph named X already exists`.
fn build_graph_list(subgraphs: &[SubgraphInputDefinition]) -> JsResult<GraphList> {
    todo!()
}

/// The `constant-case` package's `constantCase(input, { stripRegexp: /[^A-Z0-9_]+/gi })`, as
/// `buildGraphList` calls it (after replacing `/[^A-Z0-9]/gi` with `_`).
/// NOTE(port): reproduce `constant-case`'s word splitting (camelCase and digit boundaries)
/// exactly; ids are observable in the supergraph SDL.
fn constant_case(input: &str) -> String {
    todo!()
}

/// `validateSubgraph(subgraph)` (the public one in `validate.ts`).
pub fn validate_subgraph(subgraph: &SubgraphInputDefinition) -> JsResult<Vec<GraphQLError>> {
    todo!()
}

/// The success value of `validate`.
#[derive(Clone, Debug)]
pub struct ValidationSuccess {
    pub supergraph: Vec<Definition>,
    pub links: Vec<Link>,
    pub specs: SupergraphSpecs,
    pub federation_version: FederationVersion,
}

/// The result of `validate`: `{ success: true, ... }` or `{ success: false, errors }`.
#[derive(Clone, Debug)]
pub enum ValidationResult {
    Success(ValidationSuccess),
    Failure { errors: Vec<GraphQLError> },
}

/// `validate(subgraphs, __internal?)`.
pub fn validate(subgraphs: &[SubgraphInputDefinition], internal: Option<&InternalOptions>) -> JsResult<ValidationResult> {
    todo!()
}

/// `isVersionHigher(sourceVersion, targetVersion)`: `Number("2.1") > Number("2.0")`.
fn is_version_higher(source_version: FederationVersion, target_version: FederationVersion) -> bool {
    todo!()
}
