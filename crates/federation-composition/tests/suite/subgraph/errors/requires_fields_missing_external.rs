//! Port of `__tests__/subgraph/errors/REQUIRES_FIELDS_MISSING_EXTERNAL.spec.ts`.

use crate::shared::*;

test_versions! {
    fn requires_fields_missing_external(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@requires"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              internalId: ID!
              profile: Profile @requires(fields: "internalId")
            }

            type Profile {
              name: String
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] On field "User.profile", for @requires(fields: "internalId"): field "User.internalId" should not be part of a @requires since it is already provided by this subgraph (it is not marked @external)"#,
                )
                .code("REQUIRES_FIELDS_MISSING_EXTERNAL")],
        );

        // The TS runs this assertion in the `testVersions` callback, outside any `test`, so it
        // runs (once per version) while the file is collected; here it runs in the version's test.
        assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
          type Query {
            users: [User]
          }

          type User @key(fields: "id") {
            id: ID!
            internalId: ID!
            profile: Profile @requires(fields: "internalId")
          }

          type Profile {
            name: String
          }
        "#))]));
    }
}
