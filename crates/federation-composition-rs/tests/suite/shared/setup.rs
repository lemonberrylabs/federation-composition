//! Port of `__tests__/shared/setup.ts`: the `toEqualGraphQL` and `toContainGraphQL` matchers.
//!
//! Both sides go through `ensureDocumentNode` (a string is parsed, a definition is wrapped in
//! a document), `sortSDL` (the library's) and graphql-js `print`; then the printed strings are
//! compared for equality or containment.

use federation_composition_rs::sort_sdl;
use federation_composition_rs::utils::js::JsError;
use graphql_rs::{Definition, Document};

/// A value `toEqualGraphQL`/`toContainGraphQL` accept: a string, a `DocumentNode`, or a
/// type-system definition node. `Option::None` is `undefined`, which the matchers reject the
/// way `isStringOrNode` does.
pub trait GraphQLValue {
    /// `ensureDocumentNode(value)`, or `Err` when `isStringOrNode(value)` is false.
    fn ensure_document_node(self) -> Result<Document, ()>;
}

fn parse_or_panic(source: &str) -> Document {
    match graphql_rs::parse(source) {
        Ok(document) => document,
        Err(error) => panic!("parse threw: {}\n{source}", error.message),
    }
}

impl GraphQLValue for &str {
    fn ensure_document_node(self) -> Result<Document, ()> {
        Ok(parse_or_panic(self))
    }
}

impl GraphQLValue for &String {
    fn ensure_document_node(self) -> Result<Document, ()> {
        Ok(parse_or_panic(self))
    }
}

impl GraphQLValue for String {
    fn ensure_document_node(self) -> Result<Document, ()> {
        Ok(parse_or_panic(&self))
    }
}

impl GraphQLValue for &Document {
    fn ensure_document_node(self) -> Result<Document, ()> {
        Ok(self.clone())
    }
}

impl GraphQLValue for Document {
    fn ensure_document_node(self) -> Result<Document, ()> {
        Ok(self)
    }
}

impl GraphQLValue for &Definition {
    fn ensure_document_node(self) -> Result<Document, ()> {
        Ok(Document::new(vec![self.clone()]))
    }
}

impl GraphQLValue for Definition {
    fn ensure_document_node(self) -> Result<Document, ()> {
        Ok(Document::new(vec![self]))
    }
}

impl<T: GraphQLValue> GraphQLValue for Option<T> {
    fn ensure_document_node(self) -> Result<Document, ()> {
        match self {
            Some(value) => value.ensure_document_node(),
            None => Err(()),
        }
    }
}

/// A getter that threw (`result.publicSdl` when `transformSupergraphToPublicSchema` throws):
/// the TS expression throws before the matcher runs, so this panics with the thrown value.
impl<T: GraphQLValue> GraphQLValue for Result<T, JsError> {
    fn ensure_document_node(self) -> Result<Document, ()> {
        match self {
            Ok(value) => value.ensure_document_node(),
            Err(error) => panic!("threw: {}", error.to_js_string()),
        }
    }
}

/// `print(sortSDL(ensureDocumentNode(value)))` for both sides, or the matcher's failure message
/// when a side is not a string or AST node.
fn printed(received: impl GraphQLValue, expected: impl GraphQLValue) -> Result<(String, String), &'static str> {
    let received = received.ensure_document_node().map_err(|()| "received value is not a string or AST node")?;
    let expected = expected.ensure_document_node().map_err(|()| "expected value is not a string or AST node")?;
    let print_sorted = |document: &Document| match sort_sdl(document) {
        Ok(sorted) => graphql_rs::print(&sorted),
        Err(error) => panic!("sortSDL threw: {}", error.to_js_string()),
    };
    Ok((print_sorted(&received), print_sorted(&expected)))
}

/// `expect(received).toEqualGraphQL(expected)`.
#[track_caller]
pub fn to_equal_graphql(received: impl GraphQLValue, expected: impl GraphQLValue) {
    let (received, expected) = printed(received, expected).unwrap_or_else(|message| panic!("{message}"));
    if received != expected {
        panic!("expected to be equal\n--- received ---\n{received}\n--- expected ---\n{expected}\n");
    }
}

/// `expect(received).not.toEqualGraphQL(expected)`. As in vitest, a side that is not a string
/// or AST node makes the matcher fail, so the negation passes.
#[track_caller]
pub fn not_to_equal_graphql(received: impl GraphQLValue, expected: impl GraphQLValue) {
    let Ok((received, expected)) = printed(received, expected) else {
        return;
    };
    if received == expected {
        panic!("expected not to be equal {expected}\n--- received ---\n{received}\n");
    }
}

/// `expect(received).toContainGraphQL(expected)`.
#[track_caller]
pub fn to_contain_graphql(received: impl GraphQLValue, expected: impl GraphQLValue) {
    let (received, expected) = printed(received, expected).unwrap_or_else(|message| panic!("{message}"));
    if !received.contains(&expected) {
        panic!("expected to find\n--- received ---\n{received}\n--- expected ---\n{expected}\n");
    }
}

/// `expect(received).not.toContainGraphQL(expected)`. As in vitest, a side that is not a string
/// or AST node makes the matcher fail, so the negation passes.
#[track_caller]
pub fn not_to_contain_graphql(received: impl GraphQLValue, expected: impl GraphQLValue) {
    let Ok((received, expected)) = printed(received, expected) else {
        return;
    };
    if received.contains(&expected) {
        panic!("expected to not find {expected}\n--- received ---\n{received}\n");
    }
}
