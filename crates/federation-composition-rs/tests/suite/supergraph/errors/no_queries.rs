//! Port of `__tests__/supergraph/errors/NO_QUERIES.spec.ts`.

use crate::shared::*;

test_versions! {
    fn no_queries(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type User @key(fields: "id") {
              id: ID!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing("No queries found in any subgraph: a supergraph must have a query root type.")
                    .code("NO_QUERIES"),
            ],
        );

        assert_composition_success(&api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              users: [User]
            }
          "#, version)),
        ]));

        assert_composition_success(&api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type User @key(fields: "id") {
              id: ID!
            }

            schema {
              query: RootQuery
            }

            type RootQuery {
              users: [User]
            }
          "#, version)),
        ]));
    }
}
