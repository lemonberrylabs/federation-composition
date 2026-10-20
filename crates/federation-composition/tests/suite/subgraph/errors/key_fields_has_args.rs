//! Port of `__tests__/subgraph/errors/KEY_FIELDS_HAS_ARGS.spec.ts`.

use crate::shared::*;

test_versions! {
    fn key_fields_has_args(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id tags") {
              id: ID!
              tags(limit: Int = 10): [String]
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] On type "User", for @key(fields: "id tags"): field User.tags cannot be included because it has arguments (fields with argument are not allowed in @key)"#,
                )
                .code("KEY_FIELDS_HAS_ARGS")],
        );

        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            type Query {
              users: [User]
            }

            type User @key(fields: "id tags") {
              id: ID!
              tags(limit: Int = 10): [String]
            }
          "#))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] On type "User", for @key(fields: "id tags"): field User.tags cannot be included because it has arguments (fields with argument are not allowed in @key)"#,
                )
                .code("KEY_FIELDS_HAS_ARGS")],
        );

        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            type Query {
              users: [User]
            }

            type User @key(fields: "id tags") @key(fields: "id") {
              id: ID!
              tags(tags: [String]): [String]
            }
          "#))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] On type "User", for @key(fields: "id tags"): field User.tags cannot be included because it has arguments (fields with argument are not allowed in @key)"#,
                )
                .code("KEY_FIELDS_HAS_ARGS")],
        );
    }
}
