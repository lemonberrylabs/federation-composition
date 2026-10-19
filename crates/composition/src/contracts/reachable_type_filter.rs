//! Port of `src/contracts/reachable-type-filter.ts`.
//!
//! NOTE(port): the TS builds a `GraphQLSchema` with graphql-js `buildASTSchema` and walks it
//! with `getNamedType`/`isObjectType`/...; the port implements the subset needed here over the
//! document (root types, fields, arguments, interfaces, union members, input fields), including
//! any error `buildASTSchema` would throw for the inputs the tests use.

use std::sync::LazyLock;

use graphql_js::{ConstDirective, Document};
use indexmap::IndexSet;

use crate::utils::js::JsResult;

/// `specifiedScalarNames`.
static SPECIFIED_SCALAR_NAMES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_js::specified_scalar_types()
        .iter()
        .map(|t| t.name.to_string())
        .collect()
});

/// `getReachableTypes(documentNode)`: the names of all types reachable from the root types.
pub fn get_reachable_types(document_node: &Document) -> JsResult<IndexSet<String>> {
    todo!()
}

/// `createAddDirectiveIfNotExists(directiveName)` returns this function: `directives` plus a
/// `@directiveName` if not present.
fn add_directive_if_not_exists(directive_name: &str, directives: &[ConstDirective]) -> Vec<ConstDirective> {
    todo!()
}

/// The argument of `addDirectiveOnTypes`.
#[derive(Clone, Copy, Debug)]
pub struct AddDirectiveOnTypesArgs<'a> {
    pub document_node: &'a Document,
    /// A set of excluded types, e.g. from `get_reachable_types`.
    pub excluded_type_names: &'a IndexSet<String>,
    /// Name of the directive to add on the types.
    pub directive_name: &'a str,
}

/// `addDirectiveOnTypes(args)`: adds the directive to every named type definition not in
/// `excluded_type_names`.
pub fn add_directive_on_types(args: AddDirectiveOnTypesArgs<'_>) -> Document {
    todo!()
}
