//! Port of `src/utils/link/index.ts`: a simple API for Federation v2 `@link` directives.

// TS: `src/utils/link/link.ts` is in `src/utils/link/`, so the module has its parent's name.
#[allow(clippy::module_inception)]
pub mod link;
pub mod link_import;
pub mod link_url;

use graphql_core::Document;

pub use link::FederatedLink;
pub use link_import::FederatedLinkImport;
pub use link_url::FederatedLinkUrl;

use crate::utils::collections::IndexMap;

use crate::Error;
use link_url::SupportsVersion;

/// The version argument of [`LinkImplementations::matches_implementation`]: upstream's
/// `LinkVersion` (`string | { major, minor } | null | typeof FEDERATION_V1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkVersion {
    /// A version string such as `"v2.3"`.
    Str(String),
    /// A major and minor version.
    MajorMinor {
        /// The major version.
        major: i64,
        /// The minor version.
        minor: i64,
    },
    /// No version (`null`): matches a link without a version.
    Null,
    /// The `FEDERATION_V1` symbol: matches when the schema has no `@link` or `@core`.
    FederationV1,
}

/// Upstream's `FEDERATION_V1` symbol, for [`LinkImplementations::matches_implementation`].
pub const FEDERATION_V1: LinkVersion = LinkVersion::FederationV1;

/// The `@link`s of a schema, from [`extract_link_implementations`](crate::extract_link_implementations):
/// the object upstream's `extractLinkImplementations` returns, whose two closures are methods.
#[derive(Clone, Debug)]
pub struct LinkImplementations {
    links: Vec<FederatedLink>,
    /// `Object.fromEntries(links.map((l) => [l.identity, l]))`: the last link per identity wins,
    /// at the position of the first.
    /// NOTE(port): plain object; identities are URLs, never integer-like.
    link_by_identity: crate::utils::collections::IndexMap<String, FederatedLink>,
    supports_federation_v2: bool,
}

impl LinkImplementations {
    /// The schema's links (`links`), in document order.
    pub fn links(&self) -> &[FederatedLink] {
        &self.links
    }

    /// The schema's links, owned.
    pub fn into_links(self) -> Vec<FederatedLink> {
        self.links
    }

    /// `resolveImportName(identity, name)`: the name `name` (`@key`, `FieldSet`) of the spec
    /// `identity` (`https://specs.apollo.dev/federation`) has in the schema, without the `@` of
    /// a directive. For a spec the schema does not link, `name` without its `@`.
    pub fn resolve_import_name(&self, identity: &str, name: &str) -> String {
        match self.link_by_identity.get(identity) {
            None => name.strip_prefix('@').unwrap_or(name).to_string(),
            Some(matching_link) => matching_link.resolve_import_name(name),
        }
    }

    /// `matchesImplementation(identity, version)`: whether the schema links the spec
    /// `identity` at a version that supports `version`.
    pub fn matches_implementation(&self, identity: &str, version: &LinkVersion) -> bool {
        // Assume Federation 1 means there is no link or identity and so it
        // always matches _if_ the typedefs dont use link or core.
        if let LinkVersion::FederationV1 = version {
            return !self.supports_federation_v2;
        }
        let Some(matching_link) = self.link_by_identity.get(identity) else {
            return false;
        };
        match version {
            LinkVersion::Str(version) => matching_link.supports(SupportsVersion::Str(version)),
            LinkVersion::Null => matching_link.supports(SupportsVersion::Null),
            LinkVersion::MajorMinor { major, minor } => {
                matching_link.supports(SupportsVersion::MajorMinor(*major, *minor))
            }
            LinkVersion::FederationV1 => !self.supports_federation_v2,
        }
    }
}

/// `extractLinkImplementations(typeDefs)`.
pub fn extract_link_implementations(type_defs: &Document) -> Result<LinkImplementations, Error> {
    let links = FederatedLink::from_typedefs(type_defs)?;
    Ok(LinkImplementations::from_links(links))
}

impl LinkImplementations {
    /// The rest of `extractLinkImplementations`, given `FederatedLink.fromTypedefs(typeDefs)`.
    /// Not part of the stable API.
    #[doc(hidden)]
    pub fn from_links(links: Vec<FederatedLink>) -> LinkImplementations {
        let mut link_by_identity: IndexMap<String, FederatedLink> = IndexMap::default();
        for l in &links {
            link_by_identity.insert(l.identity().to_string(), l.clone());
        }
        // Any schema with a `@link` directive present is considered federation 2
        // although according to federation docs, schemas require linking specifically
        // the federation 2.x spec. The reason for not being so picky is that supergraphs also
        // use @link, but do not necessarily link to the federation 2.x spec.

        // Check if any @link or @core features are used.
        let supports_federation_v2 = !link_by_identity.is_empty();

        LinkImplementations {
            links,
            link_by_identity,
            supports_federation_v2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `__tests__/utils/extract-link.spec.ts` (resolution and version matching).
    #[test]
    fn extract() {
        let doc = graphql_core::parse(
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/federation/v2.3")
        @link(url: "https://specs.graphql-hive.com/metadata/v0.1", import: ["@meta"])
        @link(url: "https://specs.graphql-hive.com/foo/v1.1")
    "#,
        )
        .unwrap();
        let li = extract_link_implementations(&doc).unwrap();
        assert_eq!(li.links().len(), 3);
        assert_eq!(
            li.resolve_import_name("https://specs.graphql-hive.com/metadata", "@meta"),
            "meta"
        );
        assert_eq!(
            li.resolve_import_name("https://specs.graphql-hive.com/foo", "@bar"),
            "foo__bar"
        );
        assert_eq!(li.resolve_import_name("https://unknown.dev", "@bar"), "bar");
        let v = |s: &str| LinkVersion::Str(s.to_string());
        assert!(li.matches_implementation("https://specs.graphql-hive.com/metadata", &v("v0.1")));
        assert!(!li.matches_implementation("https://specs.graphql-hive.com/metadata", &v("v0.2")));
        assert!(li.matches_implementation("https://specs.graphql-hive.com/foo", &v("v1.0")));
        assert!(li.matches_implementation(
            "https://specs.graphql-hive.com/foo",
            &LinkVersion::MajorMinor { major: 1, minor: 1 }
        ));
        assert!(!li.matches_implementation("https://specs.graphql-hive.com/foo", &LinkVersion::Null));
        assert!(!li.matches_implementation("x", &LinkVersion::FederationV1));
        let empty = extract_link_implementations(&graphql_core::parse("type Query { a: Int }").unwrap()).unwrap();
        assert!(empty.matches_implementation("x", &LinkVersion::FederationV1));
    }
}
