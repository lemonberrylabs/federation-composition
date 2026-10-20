//! Port of `src/specifications/federation.ts`.

use graphql_rs::{
    is_specified_directive, is_specified_scalar_type, parse, Definition, Directive, DirectiveDefinition, Document,
    GraphQLError, Name, Type, TypeDefinition, Value,
};
use std::sync::OnceLock;

use crate::utils::collections::IndexSet;

use super::link::{parse_link, Link, LinkImport};
use super::{inaccessible, tag};
use crate::graphql::printer::print;
use crate::utils::js::{JsError, JsResult};

/// `FederationVersion = keyof typeof federationSpecFactory`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FederationVersion {
    V1_0,
    V2_0,
    V2_1,
    V2_2,
    V2_3,
    V2_4,
    V2_5,
    V2_6,
    V2_7,
    V2_8,
    V2_9,
}

impl FederationVersion {
    /// The keys of `federationSpecFactory`, in object order.
    pub const ALL: [FederationVersion; 11] = [
        FederationVersion::V1_0,
        FederationVersion::V2_0,
        FederationVersion::V2_1,
        FederationVersion::V2_2,
        FederationVersion::V2_3,
        FederationVersion::V2_4,
        FederationVersion::V2_5,
        FederationVersion::V2_6,
        FederationVersion::V2_7,
        FederationVersion::V2_8,
        FederationVersion::V2_9,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FederationVersion::V1_0 => "v1.0",
            FederationVersion::V2_0 => "v2.0",
            FederationVersion::V2_1 => "v2.1",
            FederationVersion::V2_2 => "v2.2",
            FederationVersion::V2_3 => "v2.3",
            FederationVersion::V2_4 => "v2.4",
            FederationVersion::V2_5 => "v2.5",
            FederationVersion::V2_6 => "v2.6",
            FederationVersion::V2_7 => "v2.7",
            FederationVersion::V2_8 => "v2.8",
            FederationVersion::V2_9 => "v2.9",
        }
    }

    /// `isFederationVersion(version) ? version : undefined`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(version: &str) -> Option<FederationVersion> {
        FederationVersion::ALL.into_iter().find(|v| v.as_str() == version)
    }
}

impl std::fmt::Display for FederationVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `FederationImports = readonly LinkImport[]`.
pub type FederationImports = Vec<LinkImport>;

/// `{ directives, types }` returned by `createSpecSchema` and `createTypeDefinitions`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpecSchema {
    pub directives: Vec<DirectiveDefinition>,
    pub types: Vec<TypeDefinition>,
}

/// `{ version, imports }` returned by `detectFederationVersion`; also the `federation`
/// argument of `validateSubgraph` and `createSubgraphValidationContext`.
#[derive(Clone, Debug, PartialEq)]
pub struct DetectedFederation {
    pub version: FederationVersion,
    pub imports: Vec<LinkImport>,
}

/// `isFederationVersion(version)`.
pub fn is_federation_version(version: &str) -> bool {
    FederationVersion::from_str(version).is_some()
}

/// `createSpecSchema(version, imports?)`. Takes a string because `validate-subgraph` calls it
/// with an unchecked `@link` version; throws a `GraphQLError` (`UNKNOWN_FEDERATION_LINK_VERSION`)
/// for an unknown one.
/// NOTE(port): the v1.0 spec includes the module-level `@tag`/`@inaccessible` definitions by
/// reference in TS; `getAvailableFederationTypeAndDirectiveDefinitions` renames spec nodes in
/// place only for aliased imports, and v1.0 has no imports, so owned clones behave the same.
pub fn create_spec_schema(version: &str, imports: Option<&[LinkImport]>) -> JsResult<SpecSchema> {
    let Some(version) = FederationVersion::from_str(version) else {
        return Err(GraphQLError::with_code(
            format!("Invalid version {version} for the federation feature in @link directive on schema"),
            "UNKNOWN_FEDERATION_LINK_VERSION",
        )
        .into());
    };

    if version != FederationVersion::V1_0 {
        let mut spec = federation_spec_factory(version, "", imports);
        let namespaced_spec = federation_spec_factory(version, "federation__", None);

        spec.directives.extend(namespaced_spec.directives);
        spec.types.extend(namespaced_spec.types);
        return Ok(spec);
    }

    let mut spec = federation_spec_factory(version, "", None);
    spec.directives.push(tag::DIRECTIVE.clone());
    spec.directives.push(inaccessible::DIRECTIVE.clone());

    Ok(spec)
}

/// `createSpecSchema(version)` (no imports), computed once per version. Same `Err` as
/// `createSpecSchema` for an unknown version.
/// PERF: callers that only read the result share one copy instead of building it per subgraph;
/// without imports the result depends only on the version.
pub fn known_spec_schema(version: &str) -> JsResult<&'static SpecSchema> {
    static CACHE: [OnceLock<SpecSchema>; FederationVersion::ALL.len()] =
        [const { OnceLock::new() }; FederationVersion::ALL.len()];
    let Some(known_version) = FederationVersion::from_str(version) else {
        return create_spec_schema(version, None).map(|_| unreachable!("the version is unknown"));
    };
    Ok(CACHE[known_version as usize].get_or_init(|| {
        create_spec_schema(version, None).expect("createSpecSchema accepts every FederationVersion")
    }))
}

/// `federationSpecFactory[version](prefix, imports)`.
fn federation_spec_factory(
    version: FederationVersion,
    prefix: &str,
    imports: Option<&[LinkImport]>,
) -> SpecSchema {
    // The v1.0 factory takes no imports.
    let imports = if version == FederationVersion::V1_0 { None } else { imports };
    create_type_definitions(version, prefix, imports)
}

/// The SDL behind `federationSpecFactory[version]`.
fn federation_spec_sdl(version: FederationVersion) -> &'static str {
    match version {
        FederationVersion::V1_0 => FEDERATION_V1_0_SDL,
        FederationVersion::V2_0 => FEDERATION_V2_0_SDL,
        FederationVersion::V2_1 => FEDERATION_V2_1_SDL,
        FederationVersion::V2_2 => FEDERATION_V2_2_SDL,
        FederationVersion::V2_3 => FEDERATION_V2_3_SDL,
        FederationVersion::V2_4 => FEDERATION_V2_4_SDL,
        FederationVersion::V2_5 => FEDERATION_V2_5_SDL,
        FederationVersion::V2_6 => FEDERATION_V2_6_SDL,
        FederationVersion::V2_7 => FEDERATION_V2_7_SDL,
        FederationVersion::V2_8 => FEDERATION_V2_8_SDL,
        FederationVersion::V2_9 => FEDERATION_V2_9_SDL,
    }
}

const FEDERATION_V1_0_SDL: &str = r#"
        directive @key(
          fields: _FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @requires(fields: _FieldSet!) on FIELD_DEFINITION
        directive @provides(fields: _FieldSet!) on FIELD_DEFINITION
        directive @external on OBJECT | FIELD_DEFINITION
        directive @extends on OBJECT | INTERFACE
        # @override is not supported in v1 but somehow it's supported by Apollo Composition
        directive @override(from: String!) on FIELD_DEFINITION
        scalar _FieldSet
      "#;

const FEDERATION_V2_0_SDL: &str = r#"
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @external on OBJECT | FIELD_DEFINITION
        directive @shareable on FIELD_DEFINITION | OBJECT
        directive @extends on OBJECT | INTERFACE
        directive @override(from: String!) on FIELD_DEFINITION
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
      "#;

const FEDERATION_V2_1_SDL: &str = r#"
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @override(from: String!) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
      "#;

const FEDERATION_V2_2_SDL: &str = r#"
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @override(from: String!) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
      "#;

const FEDERATION_V2_3_SDL: &str = r#"
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @interfaceObject on OBJECT
        directive @override(from: String!) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
      "#;

const FEDERATION_V2_4_SDL: &str = r#"
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @interfaceObject on OBJECT
        directive @override(from: String!) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
      "#;

const FEDERATION_V2_5_SDL: &str = r#"
        directive @authenticated on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @requiresScopes(
          scopes: [[Scope!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @interfaceObject on OBJECT
        directive @override(from: String!) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
        scalar Scope
      "#;

const FEDERATION_V2_6_SDL: &str = r#"
        directive @policy(
          policies: [[federation__Policy!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @authenticated on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @requiresScopes(
          scopes: [[federation__Scope!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @interfaceObject on OBJECT
        directive @override(from: String!) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
        scalar federation__Policy
        scalar federation__Scope
      "#;

const FEDERATION_V2_7_SDL: &str = r#"
        directive @policy(
          policies: [[federation__Policy!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @authenticated on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @requiresScopes(
          scopes: [[federation__Scope!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @interfaceObject on OBJECT
        directive @override(from: String!, label: String) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        scalar FieldSet
        scalar federation__Policy
        scalar federation__Scope
      "#;

const FEDERATION_V2_8_SDL: &str = r#"
        directive @policy(
          policies: [[federation__Policy!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @authenticated on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @requiresScopes(
          scopes: [[federation__Scope!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @interfaceObject on OBJECT
        directive @override(from: String!, label: String) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        directive @cost(
          weight: Int!
        ) on ARGUMENT_DEFINITION | ENUM | FIELD_DEFINITION | INPUT_FIELD_DEFINITION | OBJECT | SCALAR
        directive @listSize(
          assumedSize: Int
          slicingArguments: [String!]
          sizedFields: [String!]
          requireOneSlicingArgument: Boolean = true
        ) on FIELD_DEFINITION
        directive @context(
          name: String!
        ) repeatable on INTERFACE | OBJECT | UNION
        directive @fromContext(
          field: federation__ContextFieldValue
        ) on ARGUMENT_DEFINITION
        scalar FieldSet
        scalar federation__Policy
        scalar federation__Scope
        scalar federation__ContextFieldValue
      "#;

const FEDERATION_V2_9_SDL: &str = r#"
        directive @policy(
          policies: [[federation__Policy!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @authenticated on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @requiresScopes(
          scopes: [[federation__Scope!]!]!
        ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
        directive @composeDirective(name: String!) repeatable on SCHEMA
        directive @extends on OBJECT | INTERFACE
        directive @external on OBJECT | FIELD_DEFINITION
        directive @key(
          fields: FieldSet!
          resolvable: Boolean = true
        ) repeatable on OBJECT | INTERFACE
        directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        directive @interfaceObject on OBJECT
        directive @override(from: String!, label: String) on FIELD_DEFINITION
        directive @provides(fields: FieldSet!) on FIELD_DEFINITION
        directive @requires(fields: FieldSet!) on FIELD_DEFINITION
        directive @shareable repeatable on FIELD_DEFINITION | OBJECT
        directive @tag(
          name: String!
        ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION
        directive @cost(
          weight: Int!
        ) on ARGUMENT_DEFINITION | ENUM | FIELD_DEFINITION | INPUT_FIELD_DEFINITION | OBJECT | SCALAR
        directive @listSize(
          assumedSize: Int
          slicingArguments: [String!]
          sizedFields: [String!]
          requireOneSlicingArgument: Boolean = true
        ) on FIELD_DEFINITION
        directive @context(
          name: String!
        ) repeatable on INTERFACE | OBJECT | UNION
        directive @fromContext(
          field: federation__ContextFieldValue
        ) on ARGUMENT_DEFINITION
        scalar FieldSet
        scalar federation__Policy
        scalar federation__Scope
        scalar federation__ContextFieldValue
      "#;

/// `getLatestFederationVersion()`: `Object.keys(federationSpecFactory).sort().pop()`.
pub fn get_latest_federation_version() -> FederationVersion {
    // String sort of the keys; the last one is "v2.9".
    let mut keys: Vec<&str> = FederationVersion::ALL.iter().map(|v| v.as_str()).collect();
    keys.sort();
    FederationVersion::from_str(keys.last().expect("non-empty")).expect("known version")
}

/// The spec SDL of `version` parsed, with `prefix` applied to every definition: the
/// `directives` and `types` arrays of `createTypeDefinitions` before filtering.
/// PERF: `createTypeDefinitions` parses the spec SDL on every call (twice per subgraph). Parsing
/// and `applyPrefix` depend only on the version and the prefix, so the result is computed once
/// per `(version, prefix)` and cloned. Only `""` and `"federation__"` are used as prefixes.
fn prefixed_spec_definitions(version: FederationVersion, prefix: &str) -> &'static SpecSchema {
    static CACHE: [[OnceLock<SpecSchema>; 2]; FederationVersion::ALL.len()] =
        [const { [const { OnceLock::new() }, const { OnceLock::new() }] }; FederationVersion::ALL.len()];
    let slot = match prefix {
        "" => 0,
        "federation__" => 1,
        _ => unreachable!("createTypeDefinitions is called with the prefixes \"\" and \"federation__\" only"),
    };
    CACHE[version as usize][slot].get_or_init(|| {
        let doc_ast = parse(federation_spec_sdl(version)).expect("valid federation spec SDL");

        let mut directives: Vec<DirectiveDefinition> = Vec::new();
        let mut types: Vec<TypeDefinition> = Vec::new();

        for node in doc_ast.definitions {
            match node {
                Definition::DirectiveDefinition(node) => directives.push(apply_prefix_to_directive(node, prefix)),
                other => {
                    // NOTE(port): the spec SDLs contain only directive and scalar definitions.
                    if let Ok(node) = TypeDefinition::try_from(other) {
                        types.push(apply_prefix_to_type(node, prefix));
                    }
                }
            }
        }

        SpecSchema { directives, types }
    })
}

/// `createTypeDefinitions(doc, prefix, imports?)`; `doc` is the spec SDL of `version`.
fn create_type_definitions(version: FederationVersion, prefix: &str, imports: Option<&[LinkImport]>) -> SpecSchema {
    let should_filter = imports.is_some();
    let mut to_include: IndexSet<String> = imports
        .unwrap_or_default()
        .iter()
        .map(|i| i.name.strip_prefix('@').unwrap_or(&i.name).to_string())
        .collect();
    // TODO: this is a hack to make sure we include FieldSet if we include any of the other directives
    // we do it differently, but only in next iterations.
    if imports.is_none_or(|i| i.is_empty())
        || to_include.contains("key")
        || to_include.contains("requires")
        || to_include.contains("provides")
    {
        to_include.insert("FieldSet".to_string());
        to_include.insert("federation__FieldSet".to_string());
    }

    if to_include.contains("requiresScopes") {
        to_include.insert("federation__Scope".to_string());
    }

    if to_include.contains("policy") {
        to_include.insert("federation__Policy".to_string());
    }

    let SpecSchema { directives, types } = prefixed_spec_definitions(version, prefix);

    SpecSchema {
        directives: directives
            .iter()
            .filter(|d| {
                !is_specified_directive(&d.name.value) && (!should_filter || to_include.contains(&d.name.value))
            })
            .cloned()
            .collect(),
        types: types.iter().filter(|t| to_include.contains(&t.name().value)).cloned().collect(),
    }
}

/// `applyPrefix(node, prefix)` for a directive definition (also prefixes non-specified
/// argument types).
fn apply_prefix_to_directive(mut node: DirectiveDefinition, prefix: &str) -> DirectiveDefinition {
    if prefix.is_empty() {
        return node;
    }

    node.name.value = format!("{prefix}{}", node.name.value);

    for arg in &mut node.arguments {
        let name_node = resolve_named_type(&mut arg.ty);

        // apply prefix to arguments with non-standard scalars (FieldSet -> PrefixFieldSet but not String -> PrefixString)
        if !is_specified_scalar_type(&name_node.value) {
            name_node.value = format!("{prefix}{}", name_node.value);
        }
    }

    node
}

/// `applyPrefix(node, prefix)` for a type definition.
fn apply_prefix_to_type(mut node: TypeDefinition, prefix: &str) -> TypeDefinition {
    if prefix.is_empty() {
        return node;
    }

    let name = match &mut node {
        TypeDefinition::ScalarTypeDefinition(x) => &mut x.name,
        TypeDefinition::ObjectTypeDefinition(x) => &mut x.name,
        TypeDefinition::InterfaceTypeDefinition(x) => &mut x.name,
        TypeDefinition::UnionTypeDefinition(x) => &mut x.name,
        TypeDefinition::EnumTypeDefinition(x) => &mut x.name,
        TypeDefinition::InputObjectTypeDefinition(x) => &mut x.name,
    };
    name.value = format!("{prefix}{}", name.value);

    node
}

/// `resolveNamedType(node)`: the innermost name of a type reference.
fn resolve_named_type(node: &mut Type) -> &mut Name {
    match node {
        Type::List(l) => resolve_named_type(&mut l.ty),
        Type::NonNull(n) => resolve_named_type(&mut n.ty),
        Type::Named(n) => &mut n.name,
    }
}

/// `isFederationLink(link)`.
pub fn is_federation_link(link: &super::link::Link) -> bool {
    link.identity == "https://specs.apollo.dev/federation"
}

/// `detectFederationVersion(typeDefs)`. Throws for a `@link` without `url` and for an
/// unsupported federation version.
pub fn detect_federation_version(type_defs: &Document) -> JsResult<DetectedFederation> {
    for definition in &type_defs.definitions {
        let directives = match definition {
            Definition::SchemaExtension(d) => &d.directives,
            Definition::SchemaDefinition(d) => &d.directives,
            _ => continue,
        };
        let links: Vec<&Directive> = directives.iter().filter(|d| d.name.value == "link").collect();

        if links.is_empty() {
            continue;
        }

        let mut parsed_links: Vec<Link> = Vec::with_capacity(links.len());
        for l in links {
            let url = l.arguments.iter().find(|a| a.name.value == "url");
            let import_arg = l.arguments.iter().find(|a| a.name.value == "import");

            let Some(url) = url else {
                return Err(JsError::error("Invalid @link directive"));
            };

            // `(url.value as any).value`
            // NOTE(port): for kinds without a string `value`, the TS passes `undefined` or a
            // boolean to `new URL`, which rejects it, so the link gets an opaque identity that
            // never matches the federation spec. The stand-ins below are rejected the same way.
            let url_string = match &url.value {
                Value::String(v) => v.value.clone(),
                Value::Int(v) => v.value.clone(),
                Value::Float(v) => v.value.clone(),
                Value::Enum(v) => v.value.clone(),
                Value::Boolean(v) => v.value.to_string(),
                _ => "undefined".to_string(),
            };

            let import_string = match import_arg {
                Some(import_arg) => print(&import_arg.value),
                None => "[]".to_string(),
            };
            parsed_links.push(parse_link(&url_string, &import_string)?);
        }

        let fed_link = parsed_links
            .iter()
            .find(|l| l.identity == "https://specs.apollo.dev/federation");

        if let Some(fed_link) = fed_link {
            if let Some(version) = fed_link.version.as_deref().filter(|v| !v.is_empty()) {
                let Some(federation_version) = FederationVersion::from_str(version) else {
                    return Err(JsError::error(format!("Unsupported federation version: {version}")));
                };

                return Ok(DetectedFederation {
                    version: federation_version,
                    imports: fed_link.imports.clone(),
                });
            }
        }
    }

    Ok(DetectedFederation {
        version: FederationVersion::V1_0,
        imports: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(spec: &SpecSchema) -> (Vec<String>, Vec<String>) {
        (
            spec.directives.iter().map(|d| d.name.value.clone()).collect(),
            spec.types.iter().map(|t| t.name().value.clone()).collect(),
        )
    }

    #[test]
    fn spec_schema() {
        let v1 = create_spec_schema("v1.0", None).unwrap();
        assert_eq!(
            names(&v1),
            (
                ["key", "requires", "provides", "external", "extends", "override", "tag", "inaccessible"]
                    .map(String::from)
                    .to_vec(),
                vec![]
            )
        );
        let imports = vec![LinkImport {
            kind: super::super::link::LinkImportKind::Directive,
            name: "@requiresScopes".to_string(),
            alias: None,
        }];
        let v25 = create_spec_schema("v2.5", Some(&imports)).unwrap();
        let (directives, types) = names(&v25);
        assert_eq!(directives[0], "requiresScopes");
        assert_eq!(directives[1], "federation__authenticated");
        assert!(directives.contains(&"federation__key".to_string()));
        assert_eq!(types, ["federation__FieldSet"]);
        let key = v25.directives.iter().find(|d| d.name.value == "federation__key").unwrap();
        assert_eq!(crate::graphql::printer::print(key), "directive @federation__key(fields: federation__FieldSet!, resolvable: Boolean = true) repeatable on OBJECT | INTERFACE");

        let err = create_spec_schema("v3.0", None).unwrap_err().into_graphql_error();
        assert_eq!(err.message, "Invalid version v3.0 for the federation feature in @link directive on schema");
        assert_eq!(err.code(), Some("UNKNOWN_FEDERATION_LINK_VERSION"));
    }

    #[test]
    fn detect() {
        let detect = |sdl: &str| detect_federation_version(&parse(sdl).unwrap());
        let d = detect(
            r#"extend schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key", {name: "@tag", as: "@mytag"}])"#,
        )
        .unwrap();
        assert_eq!(d.version, FederationVersion::V2_3);
        assert_eq!(d.imports.len(), 2);
        assert_eq!(detect("type Query { a: Int }").unwrap().version, FederationVersion::V1_0);
        assert_eq!(
            detect(r#"extend schema @link(url: "https://specs.apollo.dev/federation/v2.99")"#).unwrap_err().message(),
            "Unsupported federation version: v2.99"
        );
        assert_eq!(detect("extend schema @link(import: [])").unwrap_err().message(), "Invalid @link directive");
        assert_eq!(
            detect(r#"extend schema @link(url: true) @link(url: "https://specs.apollo.dev/federation/v2.0")"#)
                .unwrap()
                .version,
            FederationVersion::V2_0
        );
    }
}
