//! Port of `src/graphql/helpers.ts`.

use graphql_js::{Definition, Document};

/// `isDirectiveDefinition(node)`.
pub fn is_directive_definition(node: &Definition) -> bool {
    matches!(node, Definition::DirectiveDefinition(_))
}

/// `kindOrderWeightMap[kind] ?? 3`.
fn kind_order_weight(node: &Definition) -> u8 {
    match node {
        Definition::SchemaDefinition(_) => 0,
        Definition::SchemaExtension(_) => 1,
        Definition::DirectiveDefinition(_) => 2,
        _ => 3,
    }
}

/// `moveSchemaAndDirectiveDefinitionsToTop(ast)`: a new document with a stable sort by weight.
pub fn move_schema_and_directive_definitions_to_top(ast: &Document) -> Document {
    let mut definitions = ast.definitions.clone();
    definitions.sort_by_key(kind_order_weight);
    Document { definitions }
}
