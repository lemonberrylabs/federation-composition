//! Port of `src/specifications/requires-scopes.ts`.

pub const SDL: &str = r#"
  directive @requiresScopes(
    scopes: [[requiresScopes__Scope!]!]!
  ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM

  scalar requiresScopes__Scope
"#;
