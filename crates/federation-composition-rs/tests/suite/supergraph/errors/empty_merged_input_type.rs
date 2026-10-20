//! Port of `__tests__/supergraph/errors/EMPTY_MERGED_INPUT_TYPE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn empty_merged_input_type(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            input CreateUserInput {
              name: String
              email: String
            }

            type User @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              users: [User]
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            extend input CreateUserInput {
              tags: [String]
            }

            type Drink @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              drinks: [Drink]
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"None of the fields of input object type "CreateUserInput" are consistently defined in all the subgraphs defining that type. As only fields common to all subgraphs are merged, this would result in an empty type."#)
                    .code("EMPTY_MERGED_INPUT_TYPE"),
            ],
        );
    }
}
