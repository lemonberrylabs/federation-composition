//! Port of `src/contracts/add-inaccessible-to-unreachable-types.ts`.

use crate::utils::collections::IndexSet;

use super::reachable_type_filter::{add_directive_on_types, get_reachable_types, AddDirectiveOnTypesArgs};
use crate::compose::{create_composition_success_read_cache_container, CompositionSuccess};
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
    let inaccessible_directive_name = resolve_name("https://specs.apollo.dev/inaccessible", "@inaccessible");
    let federation_types: IndexSet<String> = [
        resolve_name("https://specs.apollo.dev/join", "FieldSet"),
        resolve_name("https://specs.apollo.dev/join", "Graph"),
        resolve_name("https://specs.apollo.dev/link", "Import"),
        resolve_name("https://specs.apollo.dev/link", "Purpose"),
        resolve_name("https://specs.apollo.dev/federation", "Policy"),
        resolve_name("https://specs.apollo.dev/federation", "Scope"),
        resolve_name("https://specs.apollo.dev/join", "DirectiveArguments"),
        resolve_name("https://specs.apollo.dev/join", "ContextArgument"),
        resolve_name("https://specs.apollo.dev/join", "FieldValue"),
        resolve_name("https://specs.apollo.dev/context", "ContextFieldValue"),
    ]
    .into_iter()
    .collect();

    // we retrieve the list of reachable types from the public api sdl
    let mut reachable_type_names = get_reachable_types(composition_result.public_document_node()?)?;

    // apollo router does not like @inaccessible on federation types...
    for federation_type in federation_types {
        reachable_type_names.insert(federation_type);
    }

    // then we apply the filter to the supergraph SDL (which is the source for the public api sdl)
    let supergraph_node = add_directive_on_types(AddDirectiveOnTypesArgs {
        document_node: &composition_result.supergraph_document_node,
        excluded_type_names: &reachable_type_names,
        directive_name: &inaccessible_directive_name,
    });

    Ok(create_composition_success_read_cache_container(supergraph_node))
}
