//! Port of `src/utils/link/link.ts`.

use graphql_js::{ConstArgument, Document};

use super::link_import::FederatedLinkImport;
use super::link_url::{FederatedLinkUrl, SupportsVersion};
use crate::utils::js::JsResult;

/// Supports federation 1 (`@core(feature:)`).
fn link_from_core_args(args: &[ConstArgument]) -> JsResult<Option<FederatedLink>> {
    todo!()
}

/// NOTE(port): `console.warn` calls for unexpected `url`/`as` kinds are dropped (they write to
/// stderr only).
fn link_from_args(args: &[ConstArgument]) -> JsResult<Option<FederatedLink>> {
    todo!()
}

fn namespaced(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(ns) if !ns.is_empty() => {
            if let Some(rest) = name.strip_prefix('@') {
                format!("@{ns}__{rest}")
            } else {
                format!("{ns}__{name}")
            }
        }
        _ => name.to_string(),
    }
}

/// `FederatedLink`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FederatedLink {
    url: FederatedLinkUrl,
    /// TS `_as`.
    as_: Option<String>,
    imports: Vec<FederatedLinkImport>,
}

impl FederatedLink {
    pub fn new(url: FederatedLinkUrl, as_: Option<String>, imports: Vec<FederatedLinkImport>) -> Self {
        FederatedLink { url, as_, imports }
    }

    /// `FederatedLink.fromTypedefs(typeDefs)`: all `@link`s (and Federation 1 `@core`s) on
    /// schema definitions and extensions.
    pub fn from_typedefs(type_defs: &Document) -> JsResult<Vec<FederatedLink>> {
        todo!()
    }

    /// The private `namespace` getter: `this._as ?? this._url.name`.
    fn namespace(&self) -> Option<&str> {
        self.as_.as_deref().or(self.url.name.as_deref())
    }

    /// `toString()`.
    pub fn to_string(&self) -> String {
        todo!()
    }

    /// The private `defaultImport` getter: `this.namespace && "@" + this.namespace`.
    fn default_import(&self) -> Option<String> {
        todo!()
    }

    /// `identity`.
    pub fn identity(&self) -> &str {
        &self.url.identity
    }

    /// `supports(...)`, forwarded to the url.
    pub fn supports(&self, version: SupportsVersion<'_>) -> bool {
        self.url.supports(version)
    }

    /// `resolveImportName(elementName)`.
    pub fn resolve_import_name(&self, element_name: &str) -> String {
        todo!()
    }

    /// `imports`.
    pub fn imports(&self) -> &[FederatedLinkImport] {
        &self.imports
    }
}
