//! Port of graphql-js `validation/validate`: `validateSDL` and
//! `assertValidSDL` (for documents that do not extend a schema).

use crate::ast::Document;
use crate::error::GraphQLError;
use crate::visitor::{visit, visit_in_parallel};

use super::specified_rules::specified_sdl_rules;
use super::validation_context::SDLValidationContext;

/// graphql-js `validateSDL(documentAST)` with the specified SDL rules.
///
/// Visits the document, so it recurses once per nesting level, as [`visit`](crate::visit) does;
/// [`assert_valid_sdl`] checks the depth first.
pub fn validate_sdl(document_ast: &Document) -> Vec<GraphQLError> {
    let mut context = SDLValidationContext::new(document_ast);
    let visitors = specified_sdl_rules(&mut context);
    let mut visitor = visit_in_parallel(visitors);
    visit(document_ast, &mut visitor, &mut context);
    context.into_errors()
}

/// graphql-js `assertValidSDL(documentAST)`: throws an `Error` with the
/// messages of all errors joined by blank lines.
///
/// Not in graphql-js: first returns the error of
/// [`check_nesting_depth`](crate::check_nesting_depth) with
/// [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH) for a document that nests deeper.
pub fn assert_valid_sdl(document_ast: &Document) -> Result<(), GraphQLError> {
    crate::nesting::check_nesting_depth(document_ast, crate::parser::MAX_NESTING_DEPTH)?;
    let errors = validate_sdl(document_ast);
    if !errors.is_empty() {
        let messages: Vec<&str> = errors.iter().map(|error| error.message.as_str()).collect();
        return Err(GraphQLError::new(messages.join("\n\n")));
    }
    Ok(())
}
