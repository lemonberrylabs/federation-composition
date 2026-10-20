//! Port of `src/types.ts`.

use graphql_rs::Document;

/// A subgraph to compose or validate: upstream's `ServiceDefinition`
/// (`{ typeDefs, name, url? }`).
#[derive(Clone, Debug, PartialEq)]
pub struct ServiceDefinition {
    /// The subgraph's schema (`typeDefs`), as parsed by [`graphql_rs::parse`].
    pub type_defs: Document,
    /// The subgraph's name, unique among the subgraphs composed together.
    pub name: String,
    /// The subgraph's routing URL, written to the supergraph's `join__Graph` enum.
    pub url: Option<String>,
}

impl ServiceDefinition {
    /// A subgraph without a URL.
    pub fn new(name: impl Into<String>, type_defs: Document) -> Self {
        ServiceDefinition {
            type_defs,
            name: name.into(),
            url: None,
        }
    }

    /// The subgraph with its routing URL set.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}
