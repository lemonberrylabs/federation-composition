//! Port of `src/supergraph/validation/rules/satisfiablity/errors.ts`.

use std::rc::Rc;

use super::helpers::Lazy;
use crate::utils::js::JsResult;

/// `SatisfiabilityErrorKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SatisfiabilityErrorKind {
    /// cannot move to subgraph "X" using @key(fields: "a b c") of "User", the key field(s)
    /// cannot be resolved from subgraph "Y".
    Key,
    /// cannot satisfy @require conditions on field "User.name".
    Require,
    /// field "User.name" is not resolvable because marked @external
    External,
    /// cannot find field "User.name".
    MissingField,
    /// cannot move to subgraph "X", which has field "User.name", because type "User" has no
    /// @key defined in subgraph "Y".
    NoKey,
    /// no subgraph can be reached to resolve the implementation type of @interfaceObject type "X".
    NoImplementation,
    /// could not find a match for required context for field "User.name".
    RequiredContext,
}

/// `SatisfiabilityError` (an `Error` subclass in TS; `toString()` is the message).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SatisfiabilityError {
    pub kind: SatisfiabilityErrorKind,
    pub source_graph_name: String,
    pub type_name: String,
    pub field_name: Option<String>,
    pub message: String,
}

impl SatisfiabilityError {
    pub fn for_key(source_graph_name: &str, target_graph_name: &str, type_name: &str, key_fields: &str) -> Self {
        Self::new(
            SatisfiabilityErrorKind::Key,
            source_graph_name,
            type_name,
            None,
            format!(
                "cannot move to subgraph \"{target_graph_name}\" using @key(fields: \"{key_fields}\") of \"{type_name}\", the key field(s) cannot be resolved from subgraph \"{source_graph_name}\"."
            ),
        )
    }

    pub fn for_require(source_graph_name: &str, type_name: &str, field_name: &str) -> Self {
        Self::new(
            SatisfiabilityErrorKind::Require,
            source_graph_name,
            type_name,
            Some(field_name),
            format!("cannot satisfy @require conditions on field \"{type_name}.{field_name}\"."),
        )
    }

    pub fn for_external(source_graph_name: &str, type_name: &str, field_name: &str) -> Self {
        Self::new(
            SatisfiabilityErrorKind::External,
            source_graph_name,
            type_name,
            Some(field_name),
            format!("field \"{type_name}.{field_name}\" is not resolvable because marked @external."),
        )
    }

    pub fn for_missing_field(source_graph_name: &str, type_name: &str, field_name: &str) -> Self {
        Self::new(
            SatisfiabilityErrorKind::MissingField,
            source_graph_name,
            type_name,
            Some(field_name),
            format!("cannot find field \"{type_name}.{field_name}\"."),
        )
    }

    pub fn for_no_key(source_graph_name: &str, target_graph_name: &str, type_name: &str, field_name: &str) -> Self {
        Self::new(
            SatisfiabilityErrorKind::NoKey,
            source_graph_name,
            type_name,
            Some(field_name),
            format!(
                "cannot move to subgraph \"{target_graph_name}\", which has field \"{type_name}.{field_name}\", because type \"{type_name}\" has no @key defined in subgraph \"{target_graph_name}\"."
            ),
        )
    }

    pub fn for_required_context(source_graph_name: &str, type_name: &str, field_name: &str) -> Self {
        Self::new(
            SatisfiabilityErrorKind::RequiredContext,
            source_graph_name,
            type_name,
            Some(field_name),
            format!("could not find a match for required context for field \"{type_name}.{field_name}\"."),
        )
    }

    pub fn for_no_implementation(source_graph_name: &str, type_name: &str) -> Self {
        Self::new(
            SatisfiabilityErrorKind::NoImplementation,
            source_graph_name,
            type_name,
            None,
            format!(
                "no subgraph can be reached to resolve the implementation type of @interfaceObject type \"{type_name}\"."
            ),
        )
    }

    fn new(
        kind: SatisfiabilityErrorKind,
        source_graph_name: &str,
        type_name: &str,
        field_name: Option<&str>,
        message: String,
    ) -> Self {
        SatisfiabilityError {
            kind,
            source_graph_name: source_graph_name.to_string(),
            type_name: type_name.to_string(),
            field_name: field_name.map(str::to_string),
            message,
        }
    }

    pub fn is_matching_field(&self, type_name: &str, field_name: &str) -> bool {
        if self.type_name != type_name {
            return false;
        }
        match &self.field_name {
            Some(f) if !f.is_empty() => f == field_name,
            _ => true,
        }
    }

    /// `toString()`: the message.
    pub fn to_string(&self) -> String {
        self.message.clone()
    }
}

/// The value of a lazy error: `T | T[]`.
#[derive(Clone, Debug, PartialEq)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

/// A shared lazy error value: `Lazy<T | T[]>`.
///
/// NOTE(port): the value is a `JsResult` because a TS lazy error factory can throw when it is
/// evaluated (`findDirectPaths` calls `graph.nodesOf(typeName)` inside one); the throw reaches
/// whoever evaluates it (`toArray()`, from `WalkTracker.listErrors()`).
pub type LazyError<'a, T> = Rc<Lazy<'a, JsResult<OneOrMany<T>>>>;

/// `LazyErrors<T>`: errors collected lazily and evaluated only when reported.
#[derive(Clone, Debug)]
pub struct LazyErrors<'a, T> {
    lazy_error: Vec<LazyError<'a, T>>,
}

impl<T> Default for LazyErrors<'_, T> {
    fn default() -> Self {
        LazyErrors { lazy_error: Vec::new() }
    }
}

impl<'a, T: Clone> LazyErrors<'a, T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `add(lazyError)` for a single lazy error (or list of errors).
    pub fn add(&mut self, lazy_error: LazyError<'a, T>) -> &mut Self {
        self.lazy_error.push(lazy_error);
        self
    }

    /// `add(otherLazyErrors)`: appends the other collection's lazy errors.
    pub fn add_all(&mut self, other: &LazyErrors<'a, T>) -> &mut Self {
        self.lazy_error.extend(other.lazy_error.iter().cloned());
        self
    }

    pub fn get_lazy_errors(&self) -> &[LazyError<'a, T>] {
        &self.lazy_error
    }

    /// `toArray()`: evaluates every lazy error and flattens (`filter(isDefined)` has nothing to
    /// remove: values are never `undefined`). Throws if a factory throws.
    pub fn to_array(&self) -> JsResult<Vec<T>> {
        let mut out = Vec::new();
        for e in &self.lazy_error {
            match e.get()? {
                OneOrMany::One(v) => out.push(v),
                OneOrMany::Many(vs) => out.extend(vs),
            }
        }
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.lazy_error.is_empty()
    }
}
