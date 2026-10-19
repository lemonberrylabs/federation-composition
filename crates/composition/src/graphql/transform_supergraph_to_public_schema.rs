//! Port of `src/graphql/transform-supergraph-to-public-schema.ts`.

use std::sync::LazyLock;

use graphql_js::Document;
use indexmap::IndexSet;

use crate::utils::js::JsResult;

/// `specifiedDirectives` names without `oneOf` (older graphql versions do not support it).
static SPECIFIED_DIRECTIVES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_js::specified_directives()
        .iter()
        .map(|d| d.name.value.clone())
        .filter(|d| d != "oneOf")
        .collect()
});

/// `getAdditionalDirectivesToStrip(documentNode)`: the `as:` names of `@link`s on the schema
/// definition; `None` when the schema definition has no directives.
fn get_additional_directives_to_strip(document_node: &Document) -> Option<IndexSet<String>> {
    todo!()
}

const FEDERATION_INACCESSIBLE_DIRECTIVE_URL_PREFIX: &str = "https://specs.apollo.dev/inaccessible";

/// `getInaccessibleDirectiveName(documentNode)`.
fn get_inaccessible_directive_name(document_node: &Document) -> String {
    todo!()
}

/// `transformSupergraphToPublicSchema(documentNode)`: the public API schema, as served by a
/// gateway. The TS visitor callbacks (`removeFederationOrSpecifiedDirectives`,
/// `removeInaccessibleNode`, ...) are closures over local sets; port them as a
/// `graphql_js::EditVisitor` struct holding those sets.
/// Fallible because `extractLinkImplementations` can throw.
pub fn transform_supergraph_to_public_schema(document_node: &Document) -> JsResult<Document> {
    todo!()
}
