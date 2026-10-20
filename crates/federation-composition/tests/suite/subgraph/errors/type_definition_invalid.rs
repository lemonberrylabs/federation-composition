//! Port of `__tests__/subgraph/errors/TYPE_DEFINITION_INVALID.spec.ts`.

use crate::shared::*;

test_versions! {
    fn type_definition_invalid(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "FieldSet"])

            directive @key(
              fields: FieldSet!
              resolvable: Boolean = true
            ) repeatable on OBJECT | INTERFACE

            input FieldSet {
              fields: [String!]!
            }

            type User @key(fields: "id name ") {
              id: ID!
              name: String
            }

            extend type Query {
              user(id: ID!): User
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message(
                    r#"[users] Invalid definition for type FieldSet: FieldSet should be a ScalarType but is defined as a InputObjectType"#,
                )
                .code("TYPE_DEFINITION_INVALID")],
        );
    }
}
