//! Port of `__tests__/supergraph/errors/FIELD_ARGUMENT_DEFAULT_MISMATCH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn field_argument_default_mismatch(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
              friends(type: FriendType = FAMILY): [User!]!
            }

            enum FriendType {
              FAMILY
              FRIEND
            }
          "#, version)),
                service("friends", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            extend type User @key(fields: "id") {
              id: ID
              friends(type: FriendType = FRIEND): [User!]!
            }

            enum FriendType {
              FAMILY
              FRIEND
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Argument "User.friends(type:)" has incompatible default values across subgraphs: it has default value FRIEND in subgraph "friends" but default value FAMILY in subgraph "users""#)
                    .code("FIELD_ARGUMENT_DEFAULT_MISMATCH"),
            ],
        );
    }
}
