//! Port of `src/types.ts`.

use graphql_js::Document;

/// `ServiceDefinition`.
#[derive(Clone, Debug, PartialEq)]
pub struct ServiceDefinition {
    pub type_defs: Document,
    pub name: String,
    pub url: Option<String>,
}
