//! Port of `src/compose.ts`.

use std::cell::OnceCell;

use graphql_js::{Document, GraphQLError};

use crate::subgraph::validation::validate_subgraph::InternalOptions;
use crate::types::ServiceDefinition;
use crate::utils::js::JsResult;

/// `composeServices(services, __internal?)`.
/// `Err` is a throw escaping the TS (`validate` and the parse of the core schema).
pub fn compose_services(services: &[ServiceDefinition], internal: Option<&InternalOptions>) -> JsResult<CompositionResult> {
    todo!()
}

/// `createLinkImportValue(name, alias)`.
fn create_link_import_value(name: &str, alias: Option<&str>) -> String {
    match alias {
        Some(alias) if !alias.is_empty() => format!("{{ name: \"{name}\", as: \"{alias}\" }}"),
        _ => format!("\"{name}\""),
    }
}

/// `CompositionResult = CompositionFailure | CompositionSuccess`.
#[derive(Debug)]
pub enum CompositionResult {
    Failure(CompositionFailure),
    Success(CompositionSuccess),
}

/// `CompositionFailure`.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionFailure {
    pub errors: Vec<GraphQLError>,
}

/// `CompositionSuccess`, as built by `createCompositionSuccessReadCacheContainer`: the SDL and
/// public schema are computed on first access and cached (`OnceCell`).
#[derive(Debug)]
pub struct CompositionSuccess {
    pub supergraph_document_node: Document,
    supergraph_sdl: OnceCell<String>,
    public_document_node: OnceCell<Document>,
    public_sdl: OnceCell<String>,
}

impl CompositionSuccess {
    /// The `supergraphSdl` getter: `print(supergraphDocumentNode)` (library printer).
    pub fn supergraph_sdl(&self) -> &str {
        todo!()
    }

    /// The `publicDocumentNode` getter. `transformSupergraphToPublicSchema` can throw.
    pub fn public_document_node(&self) -> JsResult<&Document> {
        todo!()
    }

    /// The `publicSdl` getter.
    pub fn public_sdl(&self) -> JsResult<&str> {
        todo!()
    }
}

/// `assertCompositionSuccess(compositionResult, message?)`: throws
/// `message || "Unexpected test failure"` on failure.
pub fn assert_composition_success(composition_result: &CompositionResult, message: Option<&str>) -> JsResult<()> {
    todo!()
}

/// `assertCompositionFailure(compositionResult, message?)`.
pub fn assert_composition_failure(composition_result: &CompositionResult, message: Option<&str>) -> JsResult<()> {
    todo!()
}

/// `compositionHasErrors(compositionResult)`.
pub fn composition_has_errors(composition_result: &CompositionResult) -> bool {
    matches!(composition_result, CompositionResult::Failure(_))
}

/// `createCompositionSuccessReadCacheContainer(supergraphDocumentNode)`: only prints the SDL
/// and builds the public document when requested.
pub fn create_composition_success_read_cache_container(supergraph_document_node: Document) -> CompositionSuccess {
    CompositionSuccess {
        supergraph_document_node,
        supergraph_sdl: OnceCell::new(),
        public_document_node: OnceCell::new(),
        public_sdl: OnceCell::new(),
    }
}
