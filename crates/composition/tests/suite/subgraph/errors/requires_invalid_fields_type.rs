//! Port of `__tests__/subgraph/errors/REQUIRES_INVALID_FIELDS_TYPE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn requires_invalid_fields_type_int(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
            }
          "#, version)),
                service("friends", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@requires"])

            extend type User @key(fields: "id") {
              id: ID
              friends: [User!]! @requires(fields: 123)
            }
          "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[friends] On field "User.friends", for @requires(fields: 123): Invalid value for argument "fields": must be a string."#,
                )
                .code("REQUIRES_INVALID_FIELDS_TYPE")],
        );
    }

    fn requires_invalid_fields_type_enum(api, version) {
        assert_composition_success(&api.compose_services(&[
            service("users", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type Query {
            users: [User!]!
          }

          type User @key(fields: "id") {
            id: ID
          }
        "#, version)),
            service("friends", graphql!(r#"
            extend type User @key(fields: "id") {
              id: ID
              friends: [User!]! @requires(fields: id)
            }
          "#)),
        ]));
    }
}
