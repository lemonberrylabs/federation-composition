//! Port of `src/specifications/federation.ts`.

use graphql_js::{DirectiveDefinition, Document, Name, Type, TypeDefinition};

use super::link::LinkImport;
use crate::utils::js::JsResult;

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
pub fn create_spec_schema(version: &str, imports: Option<&[LinkImport]>) -> JsResult<SpecSchema> {
    todo!()
}

/// `federationSpecFactory[version](prefix, imports)`.
fn federation_spec_factory(
    version: FederationVersion,
    prefix: &str,
    imports: Option<&[LinkImport]>,
) -> SpecSchema {
    todo!()
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

/// `createTypeDefinitions(doc, prefix, imports?)`.
fn create_type_definitions(doc: &str, prefix: &str, imports: Option<&[LinkImport]>) -> SpecSchema {
    todo!()
}

/// `applyPrefix(node, prefix)` for a directive definition (also prefixes non-specified
/// argument types).
fn apply_prefix_to_directive(node: DirectiveDefinition, prefix: &str) -> DirectiveDefinition {
    todo!()
}

/// `applyPrefix(node, prefix)` for a type definition.
fn apply_prefix_to_type(node: TypeDefinition, prefix: &str) -> TypeDefinition {
    todo!()
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
    todo!()
}
