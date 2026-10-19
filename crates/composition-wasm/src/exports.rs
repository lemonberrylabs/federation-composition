//! One export per JavaScript export of `src/` that the shim needs.
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

use std::cell::RefCell;

use fc::contracts::reachable_type_filter::AddDirectiveOnTypesArgs;
use fc::contracts::tag_extraction::{
    SchemaContractFilter, SchemaCoordinateToTagsRegistry, TagFilterTransformResult,
};
use fc::specifications::federation::FederationVersion;
use fc::subgraph::validation::validate_subgraph::InternalOptions;
use fc::supergraph::composition::ast as core_ast;
use fc::utils::js::JsError;
use fc::utils::link::link::FederatedLink;
use fc::utils::link::link_import::FederatedLinkImport;
use fc::utils::link::link_url::{FederatedLinkUrl, SupportsVersion};
use fc::utils::link::{LinkImplementations, LinkVersion};
use fc::{CompositionResult, ServiceDefinition, ValidationResult};
use federation_composition as fc;
use graphql_js::{Directive, Document, Value};
use federation_composition::utils::collections::{IndexMap, IndexSet};
use serde::Deserialize;
use serde_json::json;
use wasm_bindgen::prelude::*;

use crate::convert::{self, LinkClassIn, LinkUrlIn};
use crate::{from_json, graphql_error_json, thrown, to_json, JsResult, ThrownError};

#[derive(Deserialize)]
struct Service {
    name: String,
    url: Option<String>,
    #[serde(rename = "typeDefs")]
    type_defs: Document,
}

impl From<Service> for ServiceDefinition {
    fn from(s: Service) -> Self {
        ServiceDefinition {
            type_defs: s.type_defs,
            name: s.name,
            url: s.url,
        }
    }
}

fn services(json: &str) -> JsResult<Vec<ServiceDefinition>> {
    Ok(from_json::<Vec<Service>>("services", json)?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// `composeServices`'s and `validate`'s `__internal` argument.
#[derive(Deserialize, Default)]
struct Internal {
    #[serde(rename = "disableValidationRules")]
    disable_validation_rules: Option<Vec<String>>,
}

fn internal(json: Option<String>) -> JsResult<Option<InternalOptions>> {
    Ok(json
        .map(|i| from_json::<Internal>("__internal", &i))
        .transpose()?
        .map(|i| InternalOptions {
            disable_validation_rules: i.disable_validation_rules,
        }))
}

#[derive(Deserialize)]
struct TagFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

fn tag_filter(json: &str) -> JsResult<SchemaContractFilter> {
    let f: TagFilter = from_json("tagFilter", json)?;
    Ok(SchemaContractFilter {
        include: f.include.into_iter().collect(),
        exclude: f.exclude.into_iter().collect(),
    })
}

/// The arguments of `FederatedLinkUrl.supports` / `FederatedLink.supports`.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum SupportsArgs {
    /// `supports("v1.2")`
    String { version: String },
    /// `supports(1, 2)`. `minor` is `null` when it was `undefined`.
    Numbers { major: f64, minor: Option<f64> },
    /// `supports(otherLinkUrl)`
    Url { url: LinkUrlIn },
    /// `supports(null)`
    Null,
    /// Anything else; `json` is `JSON.stringify(args)`, for the error message.
    Invalid { json: String },
}

/// A JS number as the crate's integer version part. NOTE: non-integer or
/// missing numbers (which no caller passes) are truncated / taken as `i64::MIN`.
fn version_part(n: Option<f64>) -> i64 {
    n.map(|n| n as i64).unwrap_or(i64::MIN)
}

/// Calls `supports` on the dispatched overload, or throws like the TS for other arguments.
fn with_supports_version<R>(args: &str, f: impl FnOnce(SupportsVersion<'_>) -> R) -> JsResult<R> {
    let args: SupportsArgs = from_json("supports arguments", args)?;
    let other_url: FederatedLinkUrl;
    let version = match args {
        SupportsArgs::String { ref version } => SupportsVersion::Str(version),
        SupportsArgs::Numbers { major, minor } => {
            SupportsVersion::MajorMinor(major as i64, version_part(minor))
        }
        SupportsArgs::Url { url } => {
            other_url = url.into();
            SupportsVersion::Url(&other_url)
        }
        SupportsArgs::Null => SupportsVersion::Null,
        SupportsArgs::Invalid { json } => {
            return Err(ThrownError::error(format!(
                "Unsupported version argument: {json} [object]."
            ))
            .into())
        }
    };
    Ok(f(version))
}

/// The `version` argument of `matchesImplementation`.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum LinkVersionIn {
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
struct AddDirectiveOnTypesIn {
    #[serde(rename = "documentNode")]
    document_node: Document,
    #[serde(rename = "excludedTypeNames")]
    excluded_type_names: Vec<String>,
    #[serde(rename = "directiveName")]
    directive_name: String,
}

fn composition_json(result: CompositionResult) -> String {
    match result {
        CompositionResult::Failure(failure) => to_json(&json!({
            "errors": failure.errors.iter().map(graphql_error_json).collect::<Vec<_>>(),
        })),
        CompositionResult::Success(success) => to_json(&json!({
            "supergraphDocumentNode": success.supergraph_document_node,
        })),
    }
}

fn document(what: &str, json: &str) -> JsResult<Document> {
    from_json(what, json)
}

// compose.ts

/// `composeServices(services, __internal)` → `CompositionJson`.
#[wasm_bindgen]
pub fn compose_services(services_json: &str, internal_json: Option<String>) -> JsResult<String> {
    let services = services(services_json)?;
    let internal = internal(internal_json)?;
    let result = fc::compose_services(&services, internal.as_ref()).map_err(thrown)?;
    Ok(composition_json(result))
}

/// The library printer (`src/graphql/printer.ts` `print`) of a supergraph:
/// the `supergraphSdl` / `publicSdl` getters → SDL string.
#[wasm_bindgen]
pub fn print_supergraph(document_json: &str) -> JsResult<String> {
    let document = document("document", document_json)?;
    Ok(fc::graphql::printer::print(&document))
}

// validate.ts

/// `validate(subgraphs, __internal)` → `{ success: false, errors }` or
/// `{ success: true, supergraph: Definition[], links, specs, federationVersion }`.
#[wasm_bindgen]
pub fn validate(subgraphs_json: &str, internal_json: Option<String>) -> JsResult<String> {
    let subgraphs = services(subgraphs_json)?;
    let internal = internal(internal_json)?;
    let result = fc::validate(&subgraphs, internal.as_ref()).map_err(thrown)?;
    Ok(to_json(&match result {
        ValidationResult::Failure { errors } => json!({
            "success": false,
            "errors": errors.iter().map(graphql_error_json).collect::<Vec<_>>(),
        }),
        ValidationResult::Success(success) => json!({
            "success": true,
            "supergraph": success.supergraph,
            "links": success.links.iter().map(convert::spec_link_json).collect::<Vec<_>>(),
            "specs": convert::specs_json(&success.specs),
            "federationVersion": success.federation_version.as_str(),
        }),
    }))
}

/// `validateSubgraph(subgraph)` → `GraphQLErrorJson[]`.
#[wasm_bindgen]
pub fn validate_subgraph(subgraph_json: &str) -> JsResult<String> {
    let subgraph: ServiceDefinition = from_json::<Service>("subgraph", subgraph_json)?.into();
    let errors = fc::validate_subgraph(&subgraph).map_err(thrown)?;
    Ok(to_json(
        &errors.iter().map(graphql_error_json).collect::<Vec<_>>(),
    ))
}

// graphql/

/// `sortSDL(doc)` → `Document`.
#[wasm_bindgen]
pub fn sort_sdl(document_json: &str) -> JsResult<String> {
    let document = document("document", document_json)?;
    Ok(to_json(&fc::sort_sdl(&document).map_err(thrown)?))
}

/// `transformSupergraphToPublicSchema(documentNode)` → `Document`.
#[wasm_bindgen]
pub fn transform_supergraph_to_public_schema(document_json: &str) -> JsResult<String> {
    let document = document("document", document_json)?;
    Ok(to_json(
        &fc::transform_supergraph_to_public_schema(&document).map_err(thrown)?,
    ))
}

/// `containsSupergraphSpec(sdl)`.
#[wasm_bindgen]
pub fn contains_supergraph_spec(sdl: &str) -> bool {
    fc::contains_supergraph_spec(sdl)
}

// contracts/

/// `composeSchemaContract(services, tagFilter, removeUnreachableTypes)` → `CompositionJson`.
#[wasm_bindgen]
pub fn compose_schema_contract(
    services_json: &str,
    tag_filter_json: &str,
    remove_unreachable_types: bool,
) -> JsResult<String> {
    let services = services(services_json)?;
    let filter = tag_filter(tag_filter_json)?;
    let result = fc::compose_schema_contract(&services, &filter, remove_unreachable_types)
        .map_err(thrown)?;
    Ok(composition_json(result))
}

/// `addInaccessibleToUnreachableTypes(resolveName, compositionResult)` → the
/// new supergraph `Document`. `resolve_name` is the JS callback
/// `(identity, name) => string`. The public document is computed from the
/// supergraph, as the getter of the `CompositionSuccess` the TS receives does.
#[wasm_bindgen]
pub fn add_inaccessible_to_unreachable_types(
    resolve_name: &js_sys::Function,
    supergraph_document_node: &str,
) -> JsResult<String> {
    let supergraph = document("supergraphDocumentNode", supergraph_document_node)?;
    let success = fc::create_composition_success_read_cache_container(supergraph);
    // A throw from the callback is kept and rethrown once the crate returns.
    let callback_error: RefCell<Option<JsValue>> = RefCell::new(None);
    let resolve = |identity: &str, name: &str| -> String {
        if callback_error.borrow().is_some() {
            return String::new();
        }
        match resolve_name.call2(
            &JsValue::NULL,
            &JsValue::from_str(identity),
            &JsValue::from_str(name),
        ) {
            // `resolveName` returns a string; anything else is stringified as JS would when using it.
            Ok(v) => v.as_string().unwrap_or_else(|| {
                js_sys::JSON::stringify(&v)
                    .map(String::from)
                    .unwrap_or_default()
            }),
            Err(e) => {
                *callback_error.borrow_mut() = Some(e);
                String::new()
            }
        }
    };
    let result = fc::add_inaccessible_to_unreachable_types(&resolve, &success);
    if let Some(e) = callback_error.into_inner() {
        return Err(e);
    }
    Ok(to_json(&result.map_err(thrown)?.supergraph_document_node))
}

/// `getReachableTypes(documentNode)` → `string[]` (the `Set`, in order).
#[wasm_bindgen]
pub fn get_reachable_types(document_json: &str) -> JsResult<String> {
    let document = document("documentNode", document_json)?;
    let types =
        fc::contracts::reachable_type_filter::get_reachable_types(&document).map_err(thrown)?;
    Ok(to_json(&types.iter().collect::<Vec<_>>()))
}

/// `addDirectiveOnTypes({ documentNode, excludedTypeNames, directiveName })` → `Document`.
#[wasm_bindgen]
pub fn add_directive_on_types(args_json: &str) -> JsResult<String> {
    let args: AddDirectiveOnTypesIn = from_json("args", args_json)?;
    let excluded: IndexSet<String> = args.excluded_type_names.into_iter().collect();
    Ok(to_json(
        &fc::contracts::reachable_type_filter::add_directive_on_types(AddDirectiveOnTypesArgs {
            document_node: &args.document_node,
            excluded_type_names: &excluded,
            directive_name: &args.directive_name,
        }),
    ))
}

fn tag_register_json(register: &SchemaCoordinateToTagsRegistry) -> String {
    to_json(
        &register
            .iter()
            .map(|(coordinate, tags)| json!([coordinate, tags.iter().collect::<Vec<_>>()]))
            .collect::<Vec<_>>(),
    )
}

/// `buildSchemaCoordinateTagRegister(documentNodes)` → `TagRegister`.
#[wasm_bindgen]
pub fn build_schema_coordinate_tag_register(document_nodes_json: &str) -> JsResult<String> {
    let documents: Vec<Document> = from_json("documentNodes", document_nodes_json)?;
    let refs: Vec<&Document> = documents.iter().collect();
    let register = fc::contracts::tag_extraction::build_schema_coordinate_tag_register(&refs)
        .map_err(thrown)?;
    Ok(tag_register_json(&register))
}

/// `TagFilterResult`: `{ typeDefs, typesWithAllFieldsInaccessible: [name, boolean][],
/// typesWithInaccessibleApplied: string[], tagDirectiveName, inaccessibleDirectiveName }`.
fn tag_filter_result_json(r: &TagFilterTransformResult) -> serde_json::Value {
    json!({
        "typeDefs": r.type_defs,
        "typesWithAllFieldsInaccessible": r.types_with_all_fields_inaccessible.iter().collect::<Vec<_>>(),
        "typesWithInaccessibleApplied": r.types_with_inaccessible_applied.iter().collect::<Vec<_>>(),
        "tagDirectiveName": r.transform_tag_directives.tag_directive_name(),
        "inaccessibleDirectiveName": r.transform_tag_directives.inaccessible_directive_name(),
    })
}

/// `applyTagFilterToInaccessibleTransformOnSubgraphSchema(documentNode, tagRegister, filter)`
/// → `TagFilterResult`. The shim rebuilds `transformTagDirectives` from the two names.
#[wasm_bindgen]
pub fn apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
    document_json: &str,
    tag_register_json: &str,
    filter_json: &str,
) -> JsResult<String> {
    let document = document("documentNode", document_json)?;
    let register: Vec<(String, Vec<String>)> = from_json("tagRegister", tag_register_json)?;
    let register: SchemaCoordinateToTagsRegistry = register
        .into_iter()
        .map(|(coordinate, tags)| (coordinate, tags.into_iter().collect()))
        .collect::<IndexMap<_, _>>();
    let filter = tag_filter(filter_json)?;
    let result = fc::contracts::tag_extraction::apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
        &document, &register, &filter,
    )
    .map_err(thrown)?;
    Ok(to_json(&tag_filter_result_json(&result)))
}

/// `applyTagFilterOnSubgraphs(subgraphs, filter)` with the subgraphs'
/// `typeDefs` → `TagFilterResult[]`, which the shim spreads over the input
/// subgraph objects.
#[wasm_bindgen]
pub fn apply_tag_filter_on_subgraphs(type_defs_json: &str, filter_json: &str) -> JsResult<String> {
    let documents: Vec<Document> = from_json("subgraphs' typeDefs", type_defs_json)?;
    let refs: Vec<&Document> = documents.iter().collect();
    let filter = tag_filter(filter_json)?;
    let results =
        fc::contracts::tag_extraction::apply_tag_filter_on_subgraph_type_defs(&refs, &filter)
            .map_err(thrown)?;
    Ok(to_json(
        &results
            .iter()
            .map(tag_filter_result_json)
            .collect::<Vec<_>>(),
    ))
}

/// The `transformTagDirectives(node, includeInaccessibleDirective)` function
/// returned with a `TagFilterResult`. `node` is `{ directives?: Directive[] }`
/// → `Directive[]`.
#[wasm_bindgen]
pub fn transform_tag_directives(
    tag_directive_name: &str,
    inaccessible_directive_name: &str,
    node_json: &str,
    include_inaccessible_directive: bool,
) -> JsResult<String> {
    #[derive(Deserialize)]
    struct WithDirectives {
        #[serde(default)]
        directives: Option<Vec<Directive>>,
    }
    let node: WithDirectives = from_json("node", node_json)?;
    let transform = fc::contracts::tag_extraction::TransformTagDirectives::new(
        tag_directive_name,
        inaccessible_directive_name,
    );
    Ok(to_json(&transform.call(
        node.directives.as_deref().unwrap_or_default(),
        include_inaccessible_directive,
    )))
}

// supergraph/composition/ast.ts. The inputs are the TS argument objects
// (`ObjectTypeAST`...), in JSON, with AST nodes inside them in AST JSON.

/// `createSchemaNode(schema)` → `SchemaDefinition`.
#[wasm_bindgen]
pub fn create_schema_node(input: &str) -> JsResult<String> {
    let input: convert::SchemaIn = from_json("createSchemaNode", input)?;
    Ok(to_json(&core_ast::create_schema_node(&input.into())))
}

/// `createObjectTypeNode(objectType)` → `ObjectTypeDefinition`.
#[wasm_bindgen]
pub fn create_object_type_node(input: &str) -> JsResult<String> {
    let input: convert::ObjectLikeIn = from_json("createObjectTypeNode", input)?;
    Ok(to_json(&core_ast::create_object_type_node(&input.into())))
}

/// `createInterfaceTypeNode(interfaceType)` → `InterfaceTypeDefinition`.
#[wasm_bindgen]
pub fn create_interface_type_node(input: &str) -> JsResult<String> {
    let input: convert::ObjectLikeIn = from_json("createInterfaceTypeNode", input)?;
    Ok(to_json(&core_ast::create_interface_type_node(
        &input.into(),
    )))
}

/// `createInputObjectTypeNode(inputObjectType)` → `InputObjectTypeDefinition`.
#[wasm_bindgen]
pub fn create_input_object_type_node(input: &str) -> JsResult<String> {
    let input: convert::InputObjectIn = from_json("createInputObjectTypeNode", input)?;
    Ok(to_json(&core_ast::create_input_object_type_node(
        &input.into(),
    )))
}

/// `createUnionTypeNode(unionType)` → `UnionTypeDefinition`.
#[wasm_bindgen]
pub fn create_union_type_node(input: &str) -> JsResult<String> {
    let input: convert::UnionIn = from_json("createUnionTypeNode", input)?;
    Ok(to_json(&core_ast::create_union_type_node(&input.into())))
}

/// `createEnumTypeNode(enumType)` → `EnumTypeDefinition`.
#[wasm_bindgen]
pub fn create_enum_type_node(input: &str) -> JsResult<String> {
    let input: convert::EnumIn = from_json("createEnumTypeNode", input)?;
    Ok(to_json(&core_ast::create_enum_type_node(&input.into())))
}

/// `createScalarTypeNode(scalarType)` → `ScalarTypeDefinition`.
#[wasm_bindgen]
pub fn create_scalar_type_node(input: &str) -> JsResult<String> {
    let input: convert::ScalarIn = from_json("createScalarTypeNode", input)?;
    Ok(to_json(&core_ast::create_scalar_type_node(&input.into())))
}

/// `createJoinGraphEnumTypeNode(graphs)` → `EnumTypeDefinition`.
#[wasm_bindgen]
pub fn create_join_graph_enum_type_node(input: &str) -> JsResult<String> {
    let input: Vec<convert::JoinGraphIn> = from_json("createJoinGraphEnumTypeNode", input)?;
    let graphs: Vec<core_ast::JoinGraphAst> = input.into_iter().map(Into::into).collect();
    Ok(to_json(&core_ast::create_join_graph_enum_type_node(
        &graphs,
    )))
}

/// `stripFederation(supergraph)` → `Document`.
#[wasm_bindgen]
pub fn strip_federation(supergraph_json: &str) -> JsResult<String> {
    let supergraph = document("supergraph", supergraph_json)?;
    Ok(to_json(&core_ast::strip_federation(&supergraph)))
}

// specifications/

/// `join.sdl(version)` → SDL string.
#[wasm_bindgen]
pub fn join_sdl(version: &str) -> JsResult<String> {
    let version = FederationVersion::from_str(version).ok_or_else(|| {
        ThrownError::error(format!(
            "composition-wasm: unknown federation version {version:?}"
        ))
    })?;
    Ok(fc::specifications::join::sdl(version))
}

/// `link.sdl` → SDL string.
#[wasm_bindgen]
pub fn link_sdl() -> String {
    fc::specifications::link::SDL.to_owned()
}

/// `tag.directive` → `DirectiveDefinition`.
#[wasm_bindgen]
pub fn tag_directive() -> String {
    to_json(&*fc::specifications::tag::DIRECTIVE)
}

/// `parseLinkUrl(urlString)` → `{ name, version, identity }`.
#[wasm_bindgen]
pub fn parse_link_url(url: &str) -> String {
    to_json(&convert::spec_link_url_json(
        &fc::specifications::link::parse_link_url(url),
    ))
}

/// `parseLinkImport(importString)` → `{ kind, name, alias? }[]`; throws `Error`.
#[wasm_bindgen]
pub fn parse_link_import(import: &str) -> JsResult<String> {
    let imports = fc::specifications::link::parse_link_import(import).map_err(thrown)?;
    Ok(to_json(
        &imports
            .iter()
            .map(convert::spec_link_import_json)
            .collect::<Vec<_>>(),
    ))
}

// subgraph/context.ts

/// `parseContextReference(input)` → `{ context?, selection? }`.
#[wasm_bindgen]
pub fn parse_context_reference(input: &str) -> String {
    to_json(&convert::context_reference_json(
        &fc::subgraph::context::parse_context_reference(input),
    ))
}

// utils/link/

/// `FederatedLinkUrl.fromUrl(urlSource)` → `LinkUrl`; throws `TypeError("Invalid URL")`.
#[wasm_bindgen]
pub fn link_url_from_url(url: &str) -> JsResult<String> {
    let url = FederatedLinkUrl::from_url(url).map_err(thrown)?;
    Ok(to_json(&convert::link_url_json(&url)))
}

/// `linkUrl.supports(...args)`.
#[wasm_bindgen]
pub fn link_url_supports(link_url: &str, args: &str) -> JsResult<bool> {
    let link_url: FederatedLinkUrl = from_json::<LinkUrlIn>("FederatedLinkUrl", link_url)?.into();
    with_supports_version(args, |version| link_url.supports(version))
}

/// `linkUrl.toString()`.
#[wasm_bindgen]
pub fn link_url_to_string(link_url: &str) -> JsResult<String> {
    let link_url: FederatedLinkUrl = from_json::<LinkUrlIn>("FederatedLinkUrl", link_url)?.into();
    Ok(link_url.to_string())
}

/// `linkImport.toString()`.
#[wasm_bindgen]
pub fn link_import_to_string(link_import: &str) -> JsResult<String> {
    let link_import: FederatedLinkImport =
        from_json::<convert::LinkImportClassIn>("FederatedLinkImport", link_import)?.into();
    Ok(link_import.to_string())
}

/// `FederatedLinkImport.fromTypedefs(node)` → `LinkImport[]`; throws `Error`.
#[wasm_bindgen]
pub fn link_import_from_typedefs(node: &str) -> JsResult<String> {
    let node: Value = from_json("ConstValueNode", node)?;
    let imports = FederatedLinkImport::from_typedefs(&node).map_err(thrown)?;
    Ok(to_json(
        &imports
            .iter()
            .map(convert::link_import_class_json)
            .collect::<Vec<_>>(),
    ))
}

/// `FederatedLink.fromTypedefs(typeDefs)` → `Link[]`.
#[wasm_bindgen]
pub fn link_from_typedefs(type_defs: &str) -> JsResult<String> {
    let document = document("typeDefs", type_defs)?;
    let links = FederatedLink::from_typedefs(&document).map_err(thrown)?;
    Ok(to_json(
        &links
            .iter()
            .map(convert::link_class_json)
            .collect::<Vec<_>>(),
    ))
}

fn link(json: &str) -> JsResult<FederatedLink> {
    Ok(from_json::<LinkClassIn>("FederatedLink", json)?.into())
}

/// `link.resolveImportName(elementName)`.
#[wasm_bindgen]
pub fn link_resolve_import_name(link_json: &str, element_name: &str) -> JsResult<String> {
    Ok(link(link_json)?.resolve_import_name(element_name))
}

/// `link.toString()`.
#[wasm_bindgen]
pub fn link_to_string(link_json: &str) -> JsResult<String> {
    Ok(link(link_json)?.to_string())
}

/// `link.supports(...args)`.
#[wasm_bindgen]
pub fn link_supports(link_json: &str, args: &str) -> JsResult<bool> {
    let link = link(link_json)?;
    with_supports_version(args, |version| link.supports(version))
}

/// `extractLinkImplementations(typeDefs).links` → `Link[]`; the shim keeps
/// them for the two functions below.
#[wasm_bindgen]
pub fn extract_link_implementations(type_defs: &str) -> JsResult<String> {
    let document = document("typeDefs", type_defs)?;
    let implementations = fc::extract_link_implementations(&document).map_err(thrown)?;
    Ok(to_json(
        &implementations
            .links
            .iter()
            .map(convert::link_class_json)
            .collect::<Vec<_>>(),
    ))
}

fn link_implementations(links_json: &str) -> JsResult<LinkImplementations> {
    let links: Vec<LinkClassIn> = from_json("links", links_json)?;
    Ok(LinkImplementations::from_links(
        links.into_iter().map(Into::into).collect(),
    ))
}

/// `extractLinkImplementations(...).resolveImportName(identity, name)`.
#[wasm_bindgen]
pub fn extract_link_resolve_import_name(
    links_json: &str,
    identity: &str,
    name: &str,
) -> JsResult<String> {
    Ok(link_implementations(links_json)?.resolve_import_name(identity, name))
}

/// `extractLinkImplementations(...).matchesImplementation(identity, version)`.
#[wasm_bindgen]
pub fn extract_link_matches_implementation(
    links_json: &str,
    identity: &str,
    version: &str,
) -> JsResult<bool> {
    let implementations = link_implementations(links_json)?;
    let version = match from_json::<LinkVersionIn>("version", version)? {
        LinkVersionIn::String { version } => LinkVersion::Str(version),
        LinkVersionIn::Numbers { major, minor } => LinkVersion::MajorMinor {
            major: version_part(major),
            minor: version_part(minor),
        },
        LinkVersionIn::Null => LinkVersion::Null,
        LinkVersionIn::FederationV1 => LinkVersion::FederationV1,
    };
    Ok(implementations.matches_implementation(identity, &version))
}

// Bridge self-checks, used by wasm/runtime.spec.ts.

/// Parses and reserializes a document: the JSON conversion both ways.
#[wasm_bindgen]
pub fn debug_roundtrip_document(document_json: &str) -> JsResult<String> {
    let document = document("document", document_json)?;
    Ok(to_json(&document))
}

/// Parses SDL with graphql_js; a syntax error is thrown as a `GraphQLError`.
#[wasm_bindgen]
pub fn debug_parse(sdl: &str) -> JsResult<String> {
    let document = graphql_js::parse(sdl).map_err(|e| thrown(JsError::GraphQL(e)))?;
    Ok(to_json(&document))
}

/// Throws a `TypeError`, as the crate reports a JS runtime error.
#[wasm_bindgen]
pub fn debug_type_error(message: &str) -> JsResult<()> {
    Err(thrown(JsError::type_error(message)))
}

/// Returns composition-style errors as a result.
#[wasm_bindgen]
pub fn debug_errors(message: &str, code: &str) -> String {
    let error = graphql_js::GraphQLError::with_code(message, code);
    to_json(&json!({ "errors": [graphql_error_json(&error)] }))
}

/// Panics with `message`.
#[wasm_bindgen]
pub fn debug_panic(message: &str) {
    panic!("{message}");
}
