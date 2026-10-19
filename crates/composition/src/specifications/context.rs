//! Port of `src/specifications/context.ts`.

pub const SDL: &str = r#"
  directive @context(name: String!) repeatable on OBJECT | INTERFACE | UNION

  directive @context__fromContext(
    field: context__ContextFieldValue
  ) on ARGUMENT_DEFINITION

  scalar context__ContextFieldValue
"#;
