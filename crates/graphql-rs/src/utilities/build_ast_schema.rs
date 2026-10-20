//! Port of graphql-js `utilities/buildASTSchema` (`buildASTSchema`,
//! `buildSchema`).
//!
//! NOTE(port): the options (`assumeValid`, `assumeValidSDL`, `noLocation`,
//! `experimentalFragmentArguments`) are not ported; these are graphql-js's
//! behavior without options. Thrown values are returned as `Err`; graphql-js
//! throws an `Error` (SDL validation, invariants) or a `GraphQLError`
//! (invalid `@deprecated`/`@specifiedBy` arguments), and the message is the
//! same.

use crate::ast::Document;
use crate::error::GraphQLError;
use crate::parser::parse;
use crate::r#type::directives::specified_directives;
use crate::r#type::schema::GraphQLSchema;
use crate::validation::validate::assert_valid_sdl;

use super::extend_schema::extend_schema_impl;

/// graphql-js `buildASTSchema(documentAST)`.
pub fn build_ast_schema(document_ast: &Document) -> Result<GraphQLSchema, GraphQLError> {
    assert_valid_sdl(document_ast)?;
    let mut config = extend_schema_impl(document_ast)?;
    if config.ast_node.is_none() {
        for named_type in &config.types {
            match named_type.name() {
                "Query" => config.query = Some("Query".to_string()),
                "Mutation" => config.mutation = Some("Mutation".to_string()),
                "Subscription" => config.subscription = Some("Subscription".to_string()),
                _ => {}
            }
        }
    }
    let mut directives = config.directives.take().unwrap_or_default();
    let defined: Vec<String> = directives.iter().map(|d| d.name.clone()).collect();
    directives.extend(
        specified_directives()
            .into_iter()
            .filter(|std_directive| defined.iter().all(|name| *name != std_directive.name)),
    );
    config.directives = Some(directives);
    GraphQLSchema::new(config)
}

/// graphql-js `buildSchema(source)`.
pub fn build_schema(source: &str) -> Result<GraphQLSchema, GraphQLError> {
    let document = parse(source)?;
    build_ast_schema(&document)
}
