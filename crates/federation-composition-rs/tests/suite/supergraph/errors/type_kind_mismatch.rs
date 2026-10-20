//! Port of `__tests__/supergraph/errors/TYPE_KIND_MISMATCH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn type_kind_mismatch(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              usernames: [String!]!
            }

            type User {
              id: ID!
            }
          "#, version)),
                service("friends", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            extend interface User {
              id: ID!
              friends: [User!]!
            }
          "#, version)),
                service("pets", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            input Pet {
              name: String!
            }

            extend input User {
              id: ID!
              pets: [Pet!]!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type "User" has mismatched kind: it is defined as Interface Type in subgraph "friends" but InputObject Type in subgraph "pets" and Object Type in subgraph "users""#)
                    .code("TYPE_KIND_MISMATCH"),
            ],
        );
    }
}
