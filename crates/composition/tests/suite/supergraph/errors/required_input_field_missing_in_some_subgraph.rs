//! Port of `__tests__/supergraph/errors/REQUIRED_INPUT_FIELD_MISSING_IN_SOME_SUBGRAPH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn required_input_field_missing_in_some_subgraph(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User!]!
            }

            type Mutation {
              createUser(input: CreateUser!): User!
            }

            type User @key(fields: "id") {
              id: ID
            }

            input CreateUser {
              id: ID!
              profile: ProfileInput!
            }

            input ProfileInput {
              name: String!
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            extend type Mutation {
              updateUser(input: UpdateUser!): User!
            }

            extend type User @key(fields: "id") {
              id: ID
            }

            input UpdateUser {
              id: ID!
              profile: ProfileInput!
            }

            extend input ProfileInput {
              reason: String
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Input object field "ProfileInput.name" is required in some subgraphs but does not appear in all subgraphs: it is required in subgraph "users" but does not appear in subgraph "feed""#)
                    .code("REQUIRED_INPUT_FIELD_MISSING_IN_SOME_SUBGRAPH"),
            ],
        );
    }
}
