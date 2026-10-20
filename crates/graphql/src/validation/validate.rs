//! Port of graphql-js `validation/validate`: `validateSDL` and
//! `assertValidSDL` (for documents that do not extend a schema).

use crate::ast::Document;
use crate::error::GraphQLError;
use crate::visitor::{visit, visit_in_parallel};

use super::specified_rules::specified_sdl_rules;
use super::validation_context::SDLValidationContext;

/// graphql-js `validateSDL(documentAST)` with the specified SDL rules.
pub fn validate_sdl(document_ast: &Document) -> Vec<GraphQLError> {
    let mut context = SDLValidationContext::new(document_ast);
    let visitors = specified_sdl_rules(&mut context);
    let mut visitor = visit_in_parallel(visitors);
    visit(document_ast, &mut visitor, &mut context);
    context.into_errors()
}

/// graphql-js `assertValidSDL(documentAST)`: throws an `Error` with the
/// messages of all errors joined by blank lines.
pub fn assert_valid_sdl(document_ast: &Document) -> Result<(), GraphQLError> {
    let errors = validate_sdl(document_ast);
    if !errors.is_empty() {
        let messages: Vec<&str> = errors.iter().map(|error| error.message.as_str()).collect();
        return Err(GraphQLError::new(messages.join("\n\n")));
    }
    Ok(())
}
