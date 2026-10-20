//! Port of `__tests__/supergraph/errors/INPUT_FIELD_DEFAULT_MISMATCH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn input_field_default_mismatch(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
            }

            input Filter {
              limit: Int = 5
            }
          "#, version)),
                service("friends", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            input Filter {
              limit: Int = 10
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Input field "Filter.limit" has incompatible default values across subgraphs: it has default value 10 in subgraph "friends" but default value 5 in subgraph "users""#)
                    .code("INPUT_FIELD_DEFAULT_MISMATCH"),
            ],
        );

        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
            }

            enum FriendType {
              FAMILY
              FRIEND
            }

            input Filter {
              type: FriendType = FAMILY
            }
          "#, version)),
                service("friends", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            enum FriendType {
              FAMILY
              FRIEND
            }

            input Filter {
              type: FriendType = FRIEND
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Input field "Filter.type" has incompatible default values across subgraphs: it has default value FRIEND in subgraph "friends" but default value FAMILY in subgraph "users""#)
                    .code("INPUT_FIELD_DEFAULT_MISMATCH"),
            ],
        );
    }
}
