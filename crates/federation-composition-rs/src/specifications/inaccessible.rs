//! Port of `src/specifications/inaccessible.ts`.

use std::sync::LazyLock;

use graphql_rs::{parse, Definition, DirectiveDefinition, Document};

pub const SDL: &str = r#"
  directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
"#;

/// `typeDefs = parse(sdl)`.
pub static TYPE_DEFS: LazyLock<Document> =
    LazyLock::new(|| parse(SDL).expect("valid @inaccessible SDL"));

/// `directive`: the `@inaccessible` definition. Callers clone it before changing it.
pub static DIRECTIVE: LazyLock<DirectiveDefinition> = LazyLock::new(|| {
    TYPE_DEFS
        .definitions
        .iter()
        .find_map(|d| match d {
            Definition::DirectiveDefinition(d) => Some(d.clone()),
            _ => None,
        })
        .expect("@inaccessible SDL has a directive definition")
});
