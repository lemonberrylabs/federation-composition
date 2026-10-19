//! Port of `src/utils/link/link.ts`.

use graphql_js::{ConstArgument, ConstValue, Definition, Directive, Document};

use super::link_import::FederatedLinkImport;
use super::link_url::{FederatedLinkUrl, SupportsVersion};
use crate::utils::js::JsResult;

/// Supports federation 1 (`@core(feature:)`).
fn link_from_core_args(args: &[ConstArgument]) -> JsResult<Option<FederatedLink>> {
    let feature = args
        .iter()
        .find(|arg| arg.name.value == "feature" && matches!(arg.value, ConstValue::String(_)));
    if let Some(feature) = feature {
        if let ConstValue::String(value) = &feature.value {
            let url = FederatedLinkUrl::from_url(&value.value)?;
            return Ok(Some(FederatedLink::new(url, None, Vec::new())));
        }
    }
    Ok(None)
}

/// NOTE(port): `console.warn` calls for unexpected `url`/`as` kinds are dropped (they write to
/// stderr only).
fn link_from_args(args: &[ConstArgument]) -> JsResult<Option<FederatedLink>> {
    let mut url: Option<FederatedLinkUrl> = None;
    let mut imports: Vec<FederatedLinkImport> = Vec::new();
    let mut as_: Option<String> = None;
    for arg in args {
        match arg.name.value.as_str() {
            "url" => {
                if let ConstValue::String(value) = &arg.value {
                    url = Some(FederatedLinkUrl::from_url(&value.value)?);
                }
            }
            "import" => {
                imports = FederatedLinkImport::from_typedefs(&arg.value)?;
            }
            "as" => {
                if let ConstValue::String(value) = &arg.value {
                    as_ = Some(value.value.clone());
                }
            }
            _ => {
                // ignore. Federation should validate links.
            }
        }
    }
    if let Some(url) = url {
        return Ok(Some(FederatedLink::new(url, as_, imports)));
    }
    Ok(None)
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
        let mut links: Vec<FederatedLink> = Vec::new();
        for definition in &type_defs.definitions {
            let directives: &[Directive] = match definition {
                Definition::SchemaExtension(d) => &d.directives,
                Definition::SchemaDefinition(d) => &d.directives,
                _ => continue,
            };
            // All `@link`s are parsed before the `@core`s of the same definition.
            for directive in directives.iter().filter(|d| d.name.value == "link") {
                if let Some(link) = link_from_args(&directive.arguments)? {
                    links.push(link);
                }
            }

            // Federation 1 support... Federation 1 uses "@core" instead of "@link", but behavior is similar enough that
            //  it can be translated.
            for directive in directives.iter().filter(|d| d.name.value == "core") {
                if let Some(link) = link_from_core_args(&directive.arguments)? {
                    links.push(link);
                }
            }
        }
        Ok(links)
    }

    /// The private `namespace` getter: `this._as ?? this._url.name`.
    fn namespace(&self) -> Option<&str> {
        self.as_.as_deref().or(self.url.name.as_deref())
    }

    /// `toString()`.
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        let as_part = match &self.as_ {
            Some(as_) if !as_.is_empty() => format!(", as: \"{as_}\""),
            _ => String::new(),
        };
        let imports_part = if self.imports.is_empty() {
            String::new()
        } else {
            let imports: Vec<String> = self.imports.iter().map(FederatedLinkImport::to_string).collect();
            format!(", import: [{}]", imports.join(", "))
        };
        format!("@link(url: \"{}\"{}{})", self.url.to_string(), as_part, imports_part)
    }

    /// The private `defaultImport` getter: `this.namespace && "@" + this.namespace`.
    /// NOTE(port): an empty namespace is falsy and is returned as is (`""`).
    fn default_import(&self) -> Option<String> {
        self.namespace().map(|ns| if ns.is_empty() { String::new() } else { format!("@{ns}") })
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
        if let Some(name) = self.url.name.as_deref() {
            if !name.is_empty() && element_name == format!("@{name}") {
                // @note: default is a directive... So remove the `@`
                // `this.defaultImport!`: the namespace is `as ?? name`, never null here.
                let default_import = self.default_import().unwrap_or_default();
                return substring_1(&default_import);
            }
        }
        let imported = self.imports.iter().find(|i| i.name == element_name);
        let resolved_name = match imported {
            Some(imported) => imported.as_.clone().unwrap_or_else(|| imported.name.clone()),
            None => namespaced(self.namespace(), element_name),
        };
        // Strip the `@` prefix for directives because in all implementations of mapping or visiting a schema,
        // directive names are not prefixed with `@`. The `@` is only for SDL.
        match resolved_name.strip_prefix('@') {
            Some(rest) => rest.to_string(),
            None => resolved_name,
        }
    }

    /// `imports`.
    pub fn imports(&self) -> &[FederatedLinkImport] {
        &self.imports
    }
}

/// `s.substring(1)`.
fn substring_1(s: &str) -> String {
    let mut chars = s.chars();
    chars.next();
    chars.as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trim_multiline(s: &str) -> String {
        s.split('\n').map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
    }

    /// `__tests__/utils/link.spec.ts`.
    #[test]
    fn from_typedefs_to_string() {
        let sdls = [
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/federation/v2.0")
    "#,
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0", as: "@linkz")
        @link(url: "https://specs.apollo.dev/federation/v2.0", as: "fed", import: ["@key"])
    "#,
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/federation/v2.0", import: [{ name: "@key", as: "@lookup" }])
    "#,
        ];
        for sdl in sdls {
            let first = sdl.find("@link(").unwrap();
            let links_only = trim_multiline(&sdl[first..]);
            let links = FederatedLink::from_typedefs(&graphql_js::parse(sdl).unwrap()).unwrap();
            let printed: Vec<String> = links.iter().map(FederatedLink::to_string).collect();
            assert_eq!(printed.join("\n"), links_only);
        }
    }

    #[test]
    fn resolve_import_name() {
        let sdl = r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/federation/v2.0", import: [{ name: "@key", as: "@lookup" }, "@provides"])
        @link(url: "https://unnamed.graphql-hive.com/v0.1", import: ["@meta"])
        @link(url: "https://specs.graphql-hive.com/hive/v0.1", import: ["@group"], as: "hivelink")
    "#;
        let links = FederatedLink::from_typedefs(&graphql_js::parse(sdl).unwrap()).unwrap();
        let find = |id: &str| links.iter().find(|l| l.identity() == id).unwrap();
        let federation = find("https://specs.apollo.dev/federation");
        assert_eq!(federation.resolve_import_name("@key"), "lookup");
        assert_eq!(federation.resolve_import_name("@external"), "federation__external");
        assert_eq!(federation.resolve_import_name("@provides"), "provides");
        assert_eq!(find("https://specs.apollo.dev/link").resolve_import_name("@link"), "link");
        let unnamed = find("https://unnamed.graphql-hive.com");
        assert_eq!(unnamed.resolve_import_name("@meta"), "meta");
        assert_eq!(unnamed.resolve_import_name("@unmentioned"), "unmentioned");
        let hive = find("https://specs.graphql-hive.com/hive");
        assert_eq!(hive.resolve_import_name("@group"), "group");
        assert_eq!(hive.resolve_import_name("@eg"), "hivelink__eg");
    }

    #[test]
    fn import_errors() {
        let err = |sdl: &str| {
            FederatedLink::from_typedefs(&graphql_js::parse(sdl).unwrap())
                .unwrap_err()
                .message()
                .to_string()
        };
        assert_eq!(
            err(r#"extend schema @link(url: "https://a.dev/x/v1.0", import: "@a")"#),
            "Expected a list of @link imports but got \"StringValue\""
        );
        assert_eq!(
            err(r#"extend schema @link(url: "https://a.dev/x/v1.0", import: [1])"#),
            "Unexpected value kind \"IntValue\" in @link import declaration"
        );
        assert_eq!(
            err(r#"extend schema @link(url: "https://a.dev/x/v1.0", import: [{ name: A }])"#),
            "Expected string value for @link \"name\" field but got \"EnumValue\""
        );
        assert_eq!(err(r#"schema @core(feature: "nope") { query: Q }"#), "Invalid URL");
    }
}
