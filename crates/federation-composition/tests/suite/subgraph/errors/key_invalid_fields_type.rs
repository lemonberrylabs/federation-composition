//! Port of `__tests__/subgraph/errors/KEY_INVALID_FIELDS_TYPE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn key_invalid_fields_type(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User!]!
            }

            type User @key(fields: true) {
              id: ID
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] On type "User", for @key(fields: true): Invalid value for argument "fields": must be a string."#,
                )
                .code("KEY_INVALID_FIELDS_TYPE")],
        );

        // The TS runs this assertion in the `testVersions` callback, outside any `test`, so it
        // runs (once per version) while the file is collected; here it runs in the version's test.
        assert_composition_success(&api.compose_services(&[service("a", graphql!(r#"
          type User @key(fields: ["id", "uuid"]) {
            id: ID!
            uuid: ID!
            name: String
          }

          type Query {
            users: [User]
          }
        "#))]));
    }
}
