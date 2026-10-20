//! Port of `src/specifications/tag.ts`.

use std::sync::LazyLock;

use graphql_core::{parse, Definition, DirectiveDefinition, Document};

pub const SDL: &str = r#"
  directive @tag(
    name: String!
  ) repeatable on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION | SCHEMA
"#;

/// `typeDefs = parse(sdl)`.
pub static TYPE_DEFS: LazyLock<Document> = LazyLock::new(|| parse(SDL).expect("valid @tag SDL"));

/// `directive`: the `@tag` definition. Callers clone it before changing it.
pub static DIRECTIVE: LazyLock<DirectiveDefinition> = LazyLock::new(|| {
    TYPE_DEFS
        .definitions
        .iter()
        .find_map(|d| match d {
            Definition::DirectiveDefinition(d) => Some(d.clone()),
            _ => None,
        })
        .expect("@tag SDL has a directive definition")
});
