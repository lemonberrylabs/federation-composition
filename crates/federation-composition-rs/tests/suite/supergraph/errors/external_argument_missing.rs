//! Port of `__tests__/supergraph/errors/EXTERNAL_ARGUMENT_MISSING.spec.ts`.

use crate::shared::*;

test_versions! {
    fn external_argument_missing(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              tags(limit: Int!): [String]
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@provides", "@external"])

            type Query {
              feed: [Article]
            }

            type Article @key(fields: "id") {
              id: ID!
              author: User! @provides(fields: "tags")
            }

            extend type User @key(fields: "id") {
              id: ID!
              tags: [String] @external
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Field "User.tags" is missing argument "User.tags(limit:)" in some subgraphs where it is marked @external: argument "User.tags(limit:)" is declared in subgraph "users" but not in subgraph "feed" (where "User.tags" is @external)"#)
                    .code("EXTERNAL_ARGUMENT_MISSING"),
            ],
        );

        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              tags(limit: Int): [String]
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@requires", "@external"])

            type Query {
              feed: [Article]
            }

            type Article @key(fields: "id") {
              id: ID!
              author: User! @requires(fields: "author { tags }")
            }

            extend type User @key(fields: "id") {
              id: ID!
              tags: [String] @external
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Field "User.tags" is missing argument "User.tags(limit:)" in some subgraphs where it is marked @external: argument "User.tags(limit:)" is declared in subgraph "users" but not in subgraph "feed" (where "User.tags" is @external)"#)
                    .code("EXTERNAL_ARGUMENT_MISSING"),
            ],
        );
    }
}
