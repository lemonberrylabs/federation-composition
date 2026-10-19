//! Port of `src/contracts/schema-contract.ts`.

use super::tag_extraction::SchemaContractFilter;
use crate::compose::CompositionResult;
use crate::types::ServiceDefinition;
use crate::utils::js::JsResult;

/// `composeSchemaContract(services, tagFilter, removeUnreachableTypes = true)`: composes a
/// federation schema contract using a tag filter.
pub fn compose_schema_contract(
    services: &[ServiceDefinition],
    tag_filter: &SchemaContractFilter,
    remove_unreachable_types: bool,
) -> JsResult<CompositionResult> {
    todo!()
}
