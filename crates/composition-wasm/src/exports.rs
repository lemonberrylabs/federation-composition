//! One export per JavaScript export of `src/` that the shim needs. Each
//! parses its arguments into the bridge types below, so the JSON contract is
//! checked now; the bodies return [`not_wired`] until they call
//! `federation_composition`.
//!
//! JSON shapes (TypeScript names, as `JSON.stringify` writes them):
//!
//! * `Document`, `Definition`, `DirectiveDefinition`...: graphql-js AST JSON.
//! * `GraphQLErrorJson`: `{ message, extensions? }`.
//! * `CompositionJson`: `{ errors: GraphQLErrorJson[] }` or
//!   `{ supergraphDocumentNode: Document }`; the shim adds the lazy
//!   `supergraphSdl`, `publicDocumentNode` and `publicSdl` getters.
//! * `Service`: `{ name, url?, typeDefs: Document }`.
//! * `TagFilter`: `{ include: string[], exclude: string[] }` (the `Set`s).
//! * `TagRegister`: `[coordinate, tags[]][]` (the `Map<string, Set<string>>`).
//! * `LinkUrl`: `{ identity, name, version }`; `LinkImport`: `{ name, as }`;
//!   `Link`: `{ url: LinkUrl, as, imports: LinkImport[] }`.

use graphql_js::{Document, Value};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::{from_json, not_wired, to_json, JsResult, ThrownError};

#[derive(Deserialize)]
#[allow(dead_code)]
struct Service {
    name: String,
    url: Option<String>,
    #[serde(rename = "typeDefs")]
    type_defs: Document,
}

/// `composeServices`'s and `validate`'s `__internal` argument.
#[derive(Deserialize, Default)]
#[allow(dead_code)]
struct Internal {
    #[serde(rename = "disableValidationRules")]
    disable_validation_rules: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct TagFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

type TagRegister = Vec<(String, Vec<String>)>;

#[derive(Deserialize)]
#[allow(dead_code)]
struct LinkUrl {
    identity: String,
    name: Option<String>,
    version: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct LinkImport {
    name: String,
    #[serde(rename = "as")]
    alias: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Link {
    url: LinkUrl,
    #[serde(rename = "as")]
    alias: Option<String>,
    imports: Vec<LinkImport>,
}

/// The arguments of `FederatedLinkUrl.supports` / `FederatedLink.supports`.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[allow(dead_code)]
enum SupportsArgs {
    /// `supports("v1.2")`
    String { version: String },
    /// `supports(1, 2)`. `minor` is `null` when it was `undefined`, which JS
    /// compares like `NaN`.
    Numbers { major: f64, minor: Option<f64> },
    /// `supports(otherLinkUrl)`
    Url { url: LinkUrl },
    /// `supports(null)`
    Null,
    /// Anything else; `json` is `JSON.stringify(args)`, for the error message.
    Invalid { json: String },
}

/// The `version` argument of `matchesImplementation`.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[allow(dead_code)]
enum LinkVersion {
    String {
        version: String,
    },
    /// `{ major, minor }`; `null` where the property was `undefined`.
    Numbers {
        major: Option<f64>,
        minor: Option<f64>,
    },
    Null,
    /// `FEDERATION_V1`
    FederationV1,
}

/// `addDirectiveOnTypes`'s argument, with the `Set` as an array.
#[derive(Deserialize)]
#[allow(dead_code)]
struct AddDirectiveOnTypesArgs {
    #[serde(rename = "documentNode")]
    document_node: Document,
    #[serde(rename = "excludedTypeNames")]
    excluded_type_names: Vec<String>,
    #[serde(rename = "directiveName")]
    directive_name: String,
}

// compose.ts

/// `composeServices(services, __internal)` → `CompositionJson`.
#[wasm_bindgen]
pub fn compose_services(services: &str, internal: Option<String>) -> JsResult<String> {
    let _: Vec<Service> = from_json("services", services)?;
    let _: Internal = internal
        .map(|i| from_json("__internal", &i))
        .transpose()?
        .unwrap_or_default();
    not_wired("composeServices")
}

/// The library printer (`src/graphql/printer.ts` `print`) of a supergraph:
/// the `supergraphSdl` / `publicSdl` getters → SDL string.
#[wasm_bindgen]
pub fn print_supergraph(document: &str) -> JsResult<String> {
    let _: Document = from_json("document", document)?;
    not_wired("print (src/graphql/printer.ts)")
}

// validate.ts

/// `validate(subgraphs, __internal)` → `{ success: false, errors }` or
/// `{ success: true, supergraph: Definition[], links, specs, federationVersion }`
/// with `links` and `specs` shaped like the TS objects.
#[wasm_bindgen]
pub fn validate(subgraphs: &str, internal: Option<String>) -> JsResult<String> {
    let _: Vec<Service> = from_json("subgraphs", subgraphs)?;
    let _: Internal = internal
        .map(|i| from_json("__internal", &i))
        .transpose()?
        .unwrap_or_default();
    not_wired("validate")
}

/// `validateSubgraph(subgraph)` → `GraphQLErrorJson[]`.
#[wasm_bindgen]
pub fn validate_subgraph(subgraph: &str) -> JsResult<String> {
    let _: Service = from_json("subgraph", subgraph)?;
    not_wired("validateSubgraph")
}

// graphql/

/// `sortSDL(doc)` → `Document`.
#[wasm_bindgen]
pub fn sort_sdl(document: &str) -> JsResult<String> {
    let _: Document = from_json("document", document)?;
    not_wired("sortSDL")
}

/// `transformSupergraphToPublicSchema(documentNode)` → `Document`.
#[wasm_bindgen]
pub fn transform_supergraph_to_public_schema(document: &str) -> JsResult<String> {
    let _: Document = from_json("document", document)?;
    not_wired("transformSupergraphToPublicSchema")
}

/// `containsSupergraphSpec(sdl)`.
#[wasm_bindgen]
pub fn contains_supergraph_spec(sdl: &str) -> JsResult<bool> {
    let _ = sdl;
    not_wired("containsSupergraphSpec")
}

// contracts/

/// `composeSchemaContract(services, tagFilter, removeUnreachableTypes)` →
/// `CompositionJson`.
#[wasm_bindgen]
pub fn compose_schema_contract(
    services: &str,
    tag_filter: &str,
    remove_unreachable_types: bool,
) -> JsResult<String> {
    let _: Vec<Service> = from_json("services", services)?;
    let _: TagFilter = from_json("tagFilter", tag_filter)?;
    let _ = remove_unreachable_types;
    not_wired("composeSchemaContract")
}

/// `addInaccessibleToUnreachableTypes(resolveName, compositionResult)` →
/// the new supergraph `Document`. `resolve_name` is the JS callback
/// `(identity, name) => string`; `public_document_node` is a thunk returning
/// the JSON of `compositionResult.publicDocumentNode`, called where the TS
/// reads that property.
#[wasm_bindgen]
pub fn add_inaccessible_to_unreachable_types(
    resolve_name: &js_sys::Function,
    supergraph_document_node: &str,
    public_document_node: &js_sys::Function,
) -> JsResult<String> {
    let _ = (resolve_name, public_document_node);
    let _: Document = from_json("supergraphDocumentNode", supergraph_document_node)?;
    not_wired("addInaccessibleToUnreachableTypes")
}

/// `getReachableTypes(documentNode)` → `string[]` (the `Set`, in order).
#[wasm_bindgen]
pub fn get_reachable_types(document: &str) -> JsResult<String> {
    let _: Document = from_json("documentNode", document)?;
    not_wired("getReachableTypes")
}

/// `addDirectiveOnTypes({ documentNode, excludedTypeNames, directiveName })` → `Document`.
#[wasm_bindgen]
pub fn add_directive_on_types(args: &str) -> JsResult<String> {
    let _: AddDirectiveOnTypesArgs = from_json("args", args)?;
    not_wired("addDirectiveOnTypes")
}

/// `buildSchemaCoordinateTagRegister(documentNodes)` → `TagRegister`.
#[wasm_bindgen]
pub fn build_schema_coordinate_tag_register(document_nodes: &str) -> JsResult<String> {
    let _: Vec<Document> = from_json("documentNodes", document_nodes)?;
    not_wired("buildSchemaCoordinateTagRegister")
}

/// `applyTagFilterToInaccessibleTransformOnSubgraphSchema(documentNode, tagRegister, filter)`
/// → `TagFilterResult`: `{ typeDefs: Document, typesWithAllFieldsInaccessible:
/// [name, boolean][], typesWithInaccessibleApplied: string[],
/// tagDirectiveName, inaccessibleDirectiveName }`. The shim rebuilds
/// `transformTagDirectives` from the two names.
#[wasm_bindgen]
pub fn apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
    document: &str,
    tag_register: &str,
    filter: &str,
) -> JsResult<String> {
    let _: Document = from_json("documentNode", document)?;
    let _: TagRegister = from_json("tagRegister", tag_register)?;
    let _: TagFilter = from_json("filter", filter)?;
    not_wired("applyTagFilterToInaccessibleTransformOnSubgraphSchema")
}

/// `applyTagFilterOnSubgraphs(subgraphs, filter)` with the subgraphs'
/// `typeDefs` → `TagFilterResult[]`, which the shim spreads over the input
/// subgraph objects.
#[wasm_bindgen]
pub fn apply_tag_filter_on_subgraphs(type_defs: &str, filter: &str) -> JsResult<String> {
    let _: Vec<Document> = from_json("subgraphs' typeDefs", type_defs)?;
    let _: TagFilter = from_json("filter", filter)?;
    not_wired("applyTagFilterOnSubgraphs")
}

/// The `transformTagDirectives(node, includeInaccessibleDirective)` function
/// returned with a `TagFilterResult`. `node` is `{ directives?: Directive[] }`
/// → `Directive[]`.
#[wasm_bindgen]
pub fn transform_tag_directives(
    tag_directive_name: &str,
    inaccessible_directive_name: &str,
    node: &str,
    include_inaccessible_directive: bool,
) -> JsResult<String> {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct WithDirectives {
        #[serde(default)]
        directives: Option<Vec<graphql_js::Directive>>,
    }
    let _ = (
        tag_directive_name,
        inaccessible_directive_name,
        include_inaccessible_directive,
    );
    let _: WithDirectives = from_json("node", node)?;
    not_wired("transformTagDirectives")
}

// supergraph/composition/ast.ts. The inputs are the TS argument objects
// (`ObjectTypeAST`...), in JSON, with AST nodes inside them in AST JSON.

macro_rules! ast_factory {
    ($($(#[$doc:meta])* $rust:ident => $js:literal,)*) => {
        $(
            $(#[$doc])*
            #[wasm_bindgen]
            pub fn $rust(input: &str) -> JsResult<String> {
                let _: serde_json::Value = from_json($js, input)?;
                not_wired($js)
            }
        )*
    };
}

ast_factory! {
    /// `createSchemaNode(schema)` → `SchemaDefinition`.
    create_schema_node => "createSchemaNode",
    /// `createObjectTypeNode(objectType)` → `ObjectTypeDefinition`.
    create_object_type_node => "createObjectTypeNode",
    /// `createInterfaceTypeNode(interfaceType)` → `InterfaceTypeDefinition`.
    create_interface_type_node => "createInterfaceTypeNode",
    /// `createInputObjectTypeNode(inputObjectType)` → `InputObjectTypeDefinition`.
    create_input_object_type_node => "createInputObjectTypeNode",
    /// `createUnionTypeNode(unionType)` → `UnionTypeDefinition`.
    create_union_type_node => "createUnionTypeNode",
    /// `createEnumTypeNode(enumType)` → `EnumTypeDefinition`.
    create_enum_type_node => "createEnumTypeNode",
    /// `createScalarTypeNode(scalarType)` → `ScalarTypeDefinition`.
    create_scalar_type_node => "createScalarTypeNode",
    /// `createJoinGraphEnumTypeNode(graphs)` → `EnumTypeDefinition`.
    create_join_graph_enum_type_node => "createJoinGraphEnumTypeNode",
}

/// `stripFederation(supergraph)` → `Document`.
#[wasm_bindgen]
pub fn strip_federation(supergraph: &str) -> JsResult<String> {
    let _: Document = from_json("supergraph", supergraph)?;
    not_wired("stripFederation")
}

// specifications/

/// `join.sdl(version)` → SDL string.
#[wasm_bindgen]
pub fn join_sdl(version: &str) -> JsResult<String> {
    let _ = version;
    not_wired("sdl (specifications/join)")
}

/// `link.sdl` → SDL string.
#[wasm_bindgen]
pub fn link_sdl() -> JsResult<String> {
    not_wired("sdl (specifications/link)")
}

/// `tag.directive` → `DirectiveDefinition`.
#[wasm_bindgen]
pub fn tag_directive() -> JsResult<String> {
    not_wired("directive (specifications/tag)")
}

/// `parseLinkUrl(urlString)` → `{ name, version, identity }`.
#[wasm_bindgen]
pub fn parse_link_url(url: &str) -> JsResult<String> {
    let _ = url;
    not_wired("parseLinkUrl")
}

/// `parseLinkImport(importString)` → `{ kind, name, alias? }[]`; throws `Error`.
#[wasm_bindgen]
pub fn parse_link_import(import: &str) -> JsResult<String> {
    let _ = import;
    not_wired("parseLinkImport")
}

// subgraph/context.ts

/// `parseContextReference(input)` → `{ context?, selection? }`.
#[wasm_bindgen]
pub fn parse_context_reference(input: &str) -> JsResult<String> {
    let _ = input;
    not_wired("parseContextReference")
}

// utils/link/

/// `FederatedLinkUrl.fromUrl(urlSource)` → `LinkUrl`; throws `Error("Invalid URL")`.
#[wasm_bindgen]
pub fn link_url_from_url(url: &str) -> JsResult<String> {
    let _ = url;
    not_wired("FederatedLinkUrl.fromUrl")
}

/// `linkUrl.supports(...args)`.
#[wasm_bindgen]
pub fn link_url_supports(link_url: &str, args: &str) -> JsResult<bool> {
    let _: LinkUrl = from_json("FederatedLinkUrl", link_url)?;
    let _: SupportsArgs = from_json("supports arguments", args)?;
    not_wired("FederatedLinkUrl.supports")
}

/// `linkUrl.toString()`.
#[wasm_bindgen]
pub fn link_url_to_string(link_url: &str) -> JsResult<String> {
    let _: LinkUrl = from_json("FederatedLinkUrl", link_url)?;
    not_wired("FederatedLinkUrl.toString")
}

/// `linkImport.toString()`.
#[wasm_bindgen]
pub fn link_import_to_string(link_import: &str) -> JsResult<String> {
    let _: LinkImport = from_json("FederatedLinkImport", link_import)?;
    not_wired("FederatedLinkImport.toString")
}

/// `FederatedLinkImport.fromTypedefs(node)` → `LinkImport[]`; throws `Error`.
#[wasm_bindgen]
pub fn link_import_from_typedefs(node: &str) -> JsResult<String> {
    let _: Value = from_json("ConstValueNode", node)?;
    not_wired("FederatedLinkImport.fromTypedefs")
}

/// `FederatedLink.fromTypedefs(typeDefs)` → `Link[]`.
#[wasm_bindgen]
pub fn link_from_typedefs(type_defs: &str) -> JsResult<String> {
    let _: Document = from_json("typeDefs", type_defs)?;
    not_wired("FederatedLink.fromTypedefs")
}

/// `link.resolveImportName(elementName)`.
#[wasm_bindgen]
pub fn link_resolve_import_name(link: &str, element_name: &str) -> JsResult<String> {
    let _: Link = from_json("FederatedLink", link)?;
    let _ = element_name;
    not_wired("FederatedLink.resolveImportName")
}

/// `link.toString()`.
#[wasm_bindgen]
pub fn link_to_string(link: &str) -> JsResult<String> {
    let _: Link = from_json("FederatedLink", link)?;
    not_wired("FederatedLink.toString")
}

/// `link.supports(...args)`.
#[wasm_bindgen]
pub fn link_supports(link: &str, args: &str) -> JsResult<bool> {
    let _: Link = from_json("FederatedLink", link)?;
    let _: SupportsArgs = from_json("supports arguments", args)?;
    not_wired("FederatedLink.supports")
}

/// `extractLinkImplementations(typeDefs).links` → `Link[]`; the shim keeps
/// them for the two functions below.
#[wasm_bindgen]
pub fn extract_link_implementations(type_defs: &str) -> JsResult<String> {
    let _: Document = from_json("typeDefs", type_defs)?;
    not_wired("extractLinkImplementations")
}

/// `extractLinkImplementations(...).resolveImportName(identity, name)`.
#[wasm_bindgen]
pub fn extract_link_resolve_import_name(
    links: &str,
    identity: &str,
    name: &str,
) -> JsResult<String> {
    let _: Vec<Link> = from_json("links", links)?;
    let _ = (identity, name);
    not_wired("extractLinkImplementations().resolveImportName")
}

/// `extractLinkImplementations(...).matchesImplementation(identity, version)`.
#[wasm_bindgen]
pub fn extract_link_matches_implementation(
    links: &str,
    identity: &str,
    version: &str,
) -> JsResult<bool> {
    let _: Vec<Link> = from_json("links", links)?;
    let _: LinkVersion = from_json("version", version)?;
    let _ = identity;
    not_wired("extractLinkImplementations().matchesImplementation")
}

// Bridge self-checks, used by wasm/runtime.spec.ts.

/// Parses and reserializes a document: the JSON conversion both ways.
#[wasm_bindgen]
pub fn debug_roundtrip_document(document: &str) -> JsResult<String> {
    let document: Document = from_json("document", document)?;
    Ok(to_json(&document))
}

/// Parses SDL with graphql_js; a syntax error is thrown as a `GraphQLError`.
#[wasm_bindgen]
pub fn debug_parse(sdl: &str) -> JsResult<String> {
    let document = graphql_js::parse(sdl).map_err(ThrownError::graphql)?;
    Ok(to_json(&document))
}

/// Returns composition-style errors as a result.
#[wasm_bindgen]
pub fn debug_errors(message: &str, code: &str) -> String {
    let error = graphql_js::GraphQLError::with_code(message, code);
    to_json(&serde_json::json!({ "errors": [crate::graphql_error_json(&error)] }))
}

/// Panics with `message`.
#[wasm_bindgen]
pub fn debug_panic(message: &str) {
    panic!("{message}");
}
