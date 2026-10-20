//! The public error type. Not a port of an upstream file: upstream's functions throw JavaScript
//! errors, which the port carries internally as `utils::js::JsError` and hands to users as
//! [`Error`].

use std::fmt;

use graphql_rs::GraphQLError;

use crate::utils::js::JsError;

/// An error returned by the library's functions where upstream's TypeScript throws.
///
/// Composition problems in the subgraphs (an invalid field type, an unsatisfiable query) are not
/// reported through this type: they are the errors of a [`CompositionFailure`] or of a failed
/// [`ValidationResult`](crate::ValidationResult).
/// An `Error` is what the TypeScript library throws instead of returning, such as a duplicate
/// subgraph name (`A subgraph named a already exists`) or an invalid `@link` URL.
///
/// Some of these are GraphQL errors with an `extensions.code`, readable with
/// [`code`](Error::code) and [`graphql_error`](Error::graphql_error); the others are plain
/// errors with a message.
///
/// [`CompositionFailure`]: crate::CompositionFailure
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    inner: JsError,
}

impl Error {
    /// The error message, exactly as the TypeScript library's thrown error has it.
    pub fn message(&self) -> &str {
        self.inner.message()
    }

    /// The thrown GraphQL error, when the TypeScript library throws a `GraphQLError`.
    pub fn graphql_error(&self) -> Option<&GraphQLError> {
        match &self.inner {
            JsError::GraphQL(error) => Some(error),
            JsError::Error { .. } => None,
        }
    }

    /// `extensions.code` of the thrown GraphQL error, if it has one.
    pub fn code(&self) -> Option<&str> {
        self.graphql_error().and_then(GraphQLError::code)
    }

    /// The error as a [`GraphQLError`]: the thrown GraphQL error, or one whose message is the
    /// JavaScript `String(error)` of a plain error (`"Error: message"`), as upstream does where
    /// it reports a caught error as a GraphQL error.
    pub fn into_graphql_error(self) -> GraphQLError {
        self.inner.into_graphql_error()
    }
}

impl fmt::Display for Error {
    /// The message, without the JavaScript `Error: ` prefix.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for Error {}

#[doc(hidden)]
impl From<JsError> for Error {
    fn from(inner: JsError) -> Self {
        Error { inner }
    }
}

#[doc(hidden)]
impl From<Error> for JsError {
    fn from(error: Error) -> Self {
        error.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync + 'static>() {}

    #[test]
    fn error_surface() {
        assert_send_sync::<Error>();

        let plain = Error::from(JsError::error("boom"));
        assert_eq!(plain.to_string(), "boom");
        assert_eq!(plain.message(), "boom");
        assert_eq!(plain.code(), None);
        assert!(plain.graphql_error().is_none());
        assert_eq!(plain.into_graphql_error().message, "Error: boom");

        let graphql = Error::from(JsError::GraphQL(GraphQLError::with_code("bad", "INVALID_GRAPHQL")));
        assert_eq!(graphql.to_string(), "bad");
        assert_eq!(graphql.code(), Some("INVALID_GRAPHQL"));
        assert_eq!(graphql.into_graphql_error().code(), Some("INVALID_GRAPHQL"));
    }
}
