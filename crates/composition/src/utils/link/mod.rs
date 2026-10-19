//! Port of `src/utils/link/index.ts`: a simple API for Federation v2 `@link` directives.

pub mod link;
pub mod link_import;
pub mod link_url;

use graphql_js::Document;

pub use link::FederatedLink;
pub use link_import::FederatedLinkImport;
pub use link_url::FederatedLinkUrl;

use crate::utils::js::JsResult;

/// The version argument of `matchesImplementation`: TS `LinkVersion`
/// (`string | { major, minor } | null | typeof FEDERATION_V1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkVersion {
    Str(String),
    MajorMinor { major: i64, minor: i64 },
    Null,
    /// The `FEDERATION_V1` symbol.
    FederationV1,
}

/// The object returned by `extractLinkImplementations`; its two closures are methods.
#[derive(Clone, Debug)]
pub struct LinkImplementations {
    pub links: Vec<FederatedLink>,
    /// `Object.fromEntries(links.map((l) => [l.identity, l]))`: the last link per identity wins,
    /// at the position of the first.
    /// NOTE(port): plain object; identities are URLs, never integer-like.
    link_by_identity: indexmap::IndexMap<String, FederatedLink>,
    supports_federation_v2: bool,
}

impl LinkImplementations {
    /// `resolveImportName(identity, name)`.
    pub fn resolve_import_name(&self, identity: &str, name: &str) -> String {
        todo!()
    }

    /// `matchesImplementation(identity, version)`.
    pub fn matches_implementation(&self, identity: &str, version: &LinkVersion) -> bool {
        todo!()
    }
}

/// `extractLinkImplementations(typeDefs)`.
pub fn extract_link_implementations(type_defs: &Document) -> JsResult<LinkImplementations> {
    todo!()
}
