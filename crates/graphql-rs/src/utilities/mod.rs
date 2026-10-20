//! Ports of graphql-js `utilities`: `stripIgnoredCharacters` and
//! `concatAST` here, and in submodules what building and validating a
//! schema uses.

pub mod build_ast_schema;
pub mod coerce_input_value;
pub mod extend_schema;
pub mod type_comparators;
pub mod validate_input_value;
pub mod value_from_ast_untyped;
pub mod value_to_literal;

use crate::ast::Document;
use crate::block_string::print_block_string;
use crate::error::GraphQLError;
use crate::lexer::{Lexer, TokenKind};

/// graphql-js `stripIgnoredCharacters`.
pub fn strip_ignored_characters(source: &str) -> Result<String, GraphQLError> {
    let mut lexer = Lexer::new(source);
    let mut stripped_body = String::with_capacity(source.len());
    let mut was_last_added_token_non_punctuator = false;
    loop {
        lexer.advance().map_err(|e| *e)?;
        let current_token = &lexer.token;
        if current_token.kind == TokenKind::Eof {
            break;
        }
        let is_non_punctuator = !current_token.kind.is_punctuator();
        if was_last_added_token_non_punctuator && (is_non_punctuator || current_token.kind == TokenKind::Spread) {
            stripped_body.push(' ');
        }
        if current_token.kind == TokenKind::BlockString {
            stripped_body.push_str(&print_block_string(current_token.value.as_deref().unwrap_or(""), true));
        } else {
            stripped_body.push_str(&source[current_token.start..current_token.end]);
        }
        was_last_added_token_non_punctuator = is_non_punctuator;
    }
    Ok(stripped_body)
}

/// graphql-js `concatAST`: one document with the definitions of all of them.
pub fn concat_ast<'a>(documents: impl IntoIterator<Item = &'a Document>) -> Document {
    let mut definitions = Vec::new();
    for doc in documents {
        definitions.extend(doc.definitions.iter().cloned());
    }
    Document { definitions }
}
