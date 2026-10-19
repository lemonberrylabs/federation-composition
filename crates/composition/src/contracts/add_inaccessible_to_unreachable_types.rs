//! Port of `src/contracts/add-inaccessible-to-unreachable-types.ts`.

use crate::compose::CompositionSuccess;
use crate::utils::js::JsResult;

/// `addInaccessibleToUnreachableTypes(resolveName, compositionResult)`: adds `@inaccessible`
/// to types unreachable from the public schema (except federation types) and returns a new
/// success container. `composition_result` stands for the TS
/// `Pick<CompositionSuccess, "supergraphDocumentNode" | "publicDocumentNode">`; reading its
/// public document can throw.
pub fn add_inaccessible_to_unreachable_types(
    resolve_name: &dyn Fn(&str, &str) -> String,
    composition_result: &CompositionSuccess,
) -> JsResult<CompositionSuccess> {
    todo!()
}
