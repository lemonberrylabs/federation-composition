//! Port of `__tests__/supergraph/errors/REQUIRED_ARGUMENT_MISSING_IN_SOME_SUBGRAPH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn required_argument_missing_in_some_subgraph(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type Query {
              user(id: Int!): User!
            }

            type User @key(fields: "id") {
              id: Int!
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

            type Query {
              user: User!
            }

            type User @key(fields: "id") {
              id: Int! @external
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Argument "Query.user(id:)" is required in some subgraphs but does not appear in all subgraphs: it is required in subgraph "users" but does not appear in subgraph "feed""#)
                    .code("REQUIRED_ARGUMENT_MISSING_IN_SOME_SUBGRAPH"),
            ],
        );
    }
}
