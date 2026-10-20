//! Port of `src/contracts/schema-contract.ts`.

use super::add_inaccessible_to_unreachable_types::add_inaccessible_to_unreachable_types;
use super::tag_extraction::{apply_tag_filter_on_subgraphs, SchemaContractFilter};
use crate::compose::{compose_services, CompositionResult};
use crate::types::ServiceDefinition;
use crate::utils::js::JsResult;
use crate::utils::link::extract_link_implementations;

/// `composeSchemaContract(services, tagFilter, removeUnreachableTypes = true)`: composes a
/// federation schema contract using a tag filter.
pub fn compose_schema_contract(
    services: &[ServiceDefinition],
    tag_filter: &SchemaContractFilter,
    remove_unreachable_types: bool,
) -> JsResult<CompositionResult> {
    let filtered_subgraphs = apply_tag_filter_on_subgraphs(services, tag_filter)?;
    let composition_result = compose_services(&filtered_subgraphs, None)?;

    // `!compositionResult.errors && compositionResult.supergraphDocumentNode && removeUnreachableTypes`
    if let CompositionResult::Success(success) = &composition_result {
        if remove_unreachable_types {
            let link_implementations = extract_link_implementations(&success.supergraph_document_node)?;
            let resolve_import_name =
                |identity: &str, name: &str| link_implementations.resolve_import_name(identity, name);
            return Ok(CompositionResult::Success(add_inaccessible_to_unreachable_types(
                &resolve_import_name,
                success,
            )?));
        }
    }

    Ok(composition_result)
}
