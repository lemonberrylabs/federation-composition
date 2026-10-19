//! Port of `src/specifications/authenticated.ts`.

pub const SDL: &str = r#"
  directive @authenticated on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
"#;
