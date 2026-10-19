//! Port of `src/graphql/sort-sdl.ts`: normalizes the AST of supergraph SDLs so they can be
//! compared without worrying about ordering.

use graphql_js::{AsNodeRef, Document, SelectionSet, Value};

use crate::utils::js::JsResult;

/// `sortSDL(doc)`.
/// NOTE(port): the TS `catch` logs `"Failed to parse"` and rethrows; `visit` itself does not
/// throw for a document, so the only error source is `parseFields` inside `@key(fields:)`
/// handling, if any.
pub fn sort_sdl(doc: &Document) -> JsResult<Document> {
    todo!()
}

/// `parseFields(fields)`.
fn parse_fields(fields: &str) -> JsResult<Option<SelectionSet>> {
    todo!()
}

/// `valueNodeToString(node)`.
fn value_node_to_string(node: &Value) -> String {
    todo!()
}

/// `sortNodes(nodes)`: one generic function for the TS overloads; the list kind is decided by
/// the kind of the first node, as in TS. `lodash.sortby` is a stable sort by the key(s).
fn sort_nodes<T: AsNodeRef + Clone>(nodes: &[T]) -> Vec<T> {
    todo!()
}

/// `isOfKindList(nodes, kind)`: checks the first node's kind.
fn is_of_kind_list<T: AsNodeRef>(nodes: &[T], kinds: &[graphql_js::Kind]) -> bool {
    nodes.first().is_some_and(|n| kinds.contains(&n.as_node_ref().kind()))
}
