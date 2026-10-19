//! Port of `src/specifications/link.ts`.

use graphql_js::{ConstArgument, ConstDirective};

use crate::utils::js::JsResult;

/// `LinkImport`: one entry of `@link(import:)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LinkImport {
    pub kind: LinkImportKind,
    /// `@`-prefixed for directives.
    pub name: String,
    /// `as`, absent when not aliased.
    pub alias: Option<String>,
}

/// `"directive" | "type"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkImportKind {
    Directive,
    Type,
}

impl LinkImportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkImportKind::Directive => "directive",
            LinkImportKind::Type => "type",
        }
    }
}

/// `Link = ReturnType<typeof parseLink>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub name: Option<String>,
    /// NOTE(port): `mergeLinks` produces `""` (not `null`) when no version was seen; that is
    /// `Some("")` here. Both are falsy where the TS tests them.
    pub version: Option<String>,
    pub identity: String,
    pub imports: Vec<LinkImport>,
}

/// The result of `parseLinkUrl`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkUrl {
    pub name: Option<String>,
    pub version: Option<String>,
    pub identity: String,
}

/// `printLink(link)`: the `@link(url:, import:)` directive printed with the library printer.
pub fn print_link(link: &Link) -> String {
    todo!()
}

/// `parseLinkUrl(urlString)`. A string `new URL` rejects is an opaque identity.
/// NOTE(port): needs WHATWG URL parsing for `origin`, `protocol` and `pathname`.
pub fn parse_link_url(url_string: &str) -> LinkUrl {
    todo!()
}

/// `parseLinkImport(importString)`: parses the printed `import:` value with JSON5.
/// Throws `Invalid import binding: ${importString}: ${String(error)}`.
/// NOTE(port): needs a JSON5 parser for the subset `print` produces (lists, objects with
/// unquoted keys, double-quoted strings) and the same `String(error)` texts for the inputs
/// tests use.
pub fn parse_link_import(import_string: &str) -> JsResult<Vec<LinkImport>> {
    todo!()
}

/// `mergeLinks(links)`: groups links by identity, keeping the highest version and the union of
/// directive imports. Accepts the supergraph's `links` (`Link & { graph }`) through `&Link`.
pub fn merge_links<'l>(links: impl IntoIterator<Item = &'l Link>) -> Vec<Link> {
    todo!()
}

/// `parseLink(urlString, importString)`.
pub fn parse_link(url_string: &str, import_string: &str) -> JsResult<Link> {
    todo!()
}

/// `parseLinkDirective(directive)`: `None` when there is no string `url` argument.
pub fn parse_link_directive(directive: &ConstDirective) -> JsResult<Option<Link>> {
    todo!()
}

fn is_url_argument(arg: &ConstArgument) -> bool {
    arg.name.value == "url" && matches!(arg.value, graphql_js::Value::String(_))
}

fn is_import_argument(arg: &ConstArgument) -> bool {
    arg.name.value == "import" && matches!(arg.value, graphql_js::Value::List(_))
}

pub const SDL: &str = r#"
  directive @link(
    url: String
    as: String
    for: link__Purpose
    import: [link__Import]
  ) repeatable on SCHEMA

  scalar link__Import

  enum link__Purpose {
    """
    `SECURITY` features provide metadata necessary to securely resolve fields.
    """
    SECURITY

    """
    `EXECUTION` features provide metadata necessary for operation execution.
    """
    EXECUTION
  }
"#;
