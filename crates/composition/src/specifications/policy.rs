//! Port of `src/specifications/policy.ts`.

pub const SDL: &str = r#"
  directive @policy(
    policies: [[policy__Policy!]!]!
  ) on ENUM | FIELD_DEFINITION | INTERFACE | OBJECT | SCALAR

  scalar policy__Policy
"#;
