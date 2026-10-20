//! graphql-js `GraphQLError`, reduced to what the library observes: the
//! message and the extensions. Locations, nodes and paths are not tracked.

use std::fmt;

use crate::collections::IndexMap;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Default, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    /// Insertion-ordered, like the JS object graphql-js keeps.
    #[serde(default)]
    pub extensions: IndexMap<String, serde_json::Value>,
}

impl GraphQLError {
    pub fn new(message: impl Into<String>) -> Self {
        GraphQLError {
            message: message.into(),
            extensions: IndexMap::default(),
        }
    }

    pub fn with_extensions(
        message: impl Into<String>,
        extensions: IndexMap<String, serde_json::Value>,
    ) -> Self {
        GraphQLError {
            message: message.into(),
            extensions,
        }
    }

    /// An error with `extensions: { code }`, the shape composition errors use.
    pub fn with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        let mut extensions = IndexMap::default();
        extensions.insert("code".to_owned(), serde_json::Value::String(code.into()));
        GraphQLError::with_extensions(message, extensions)
    }

    /// `extensions.code` when it is a string.
    pub fn code(&self) -> Option<&str> {
        self.extensions.get("code").and_then(|c| c.as_str())
    }

    /// graphql-js `syntaxError`: the message is prefixed with `Syntax Error: `.
    /// Boxed, as the lexer and parser pass errors through many small results.
    pub(crate) fn syntax(description: impl fmt::Display) -> Box<Self> {
        Box::new(GraphQLError::new(format!("Syntax Error: {description}")))
    }
}

impl fmt::Display for GraphQLError {
    /// graphql-js `toString()` without source locations: the message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GraphQLError {}

impl Serialize for GraphQLError {
    /// graphql-js `toJSON()`: `extensions` only when non-empty.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("message", &self.message)?;
        if !self.extensions.is_empty() {
            m.serialize_entry("extensions", &self.extensions)?;
        }
        m.end()
    }
}
