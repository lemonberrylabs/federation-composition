//! Port of `__tests__/subgraph/errors/ROOT_QUERY_USED.spec.ts`.

use crate::shared::*;

test_versions! {
    fn root_query_used(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User!]!
            }

            schema {
              query: RootQuery
            }

            type RootQuery {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
              friends: [User!]!
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"The schema has a type named "Query" but it is not set as the query root type ("RootQuery" is instead): this is not supported by federation. If a root type does not use its default name, there should be no other type with that default name."#,
                )
                .code("ROOT_QUERY_USED")],
        );
    }
}
