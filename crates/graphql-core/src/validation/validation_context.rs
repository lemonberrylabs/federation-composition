//! Port of graphql-js `validation/ValidationContext`: `SDLValidationContext`.

use crate::ast::Document;
use crate::error::GraphQLError;

/// graphql-js `SDLValidationContext`, for validating a document that is not
/// extending a schema (`getSchema()` is `undefined`, the only case
/// `buildASTSchema` uses), with an `onError` that collects the errors.
///
/// NOTE(port): the rules read `context.getSchema()` to also check against an
/// existing schema; with no schema those branches never run and are left
/// out of the rule ports. `hideSuggestions` is always `false` here.
pub struct SDLValidationContext<'a> {
    document: &'a Document,
    errors: Vec<GraphQLError>,
}

impl<'a> SDLValidationContext<'a> {
    pub fn new(document: &'a Document) -> Self {
        SDLValidationContext {
            document,
            errors: Vec::new(),
        }
    }

    /// graphql-js `reportError(error)`.
    pub fn report_error(&mut self, error: GraphQLError) {
        self.errors.push(error);
    }

    /// graphql-js `getDocument()`.
    pub fn get_document(&self) -> &'a Document {
        self.document
    }

    /// The reported errors, in order.
    pub fn into_errors(self) -> Vec<GraphQLError> {
        self.errors
    }
}
