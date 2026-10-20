//! Port of `__tests__/subgraph/errors/PROVIDES_INVALID_FIELDS.spec.ts`.

use crate::shared::*;

// TS: the spec file's top-level `describe` block has the file's name.
#[allow(clippy::module_inception)]
mod provides_invalid_fields {
    use super::*;

    test_versions! {
        fn cannot_query_field(api, version) {
            assert_errors_contain(
                api.compose_services(&[
                    service("users", graphql!(r#"
              extend schema
              @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key"]
                )

              type Query {
                users: [User]
              }

              type User @key(fields: "id") {
                id: ID!
              }
            "#, version)),
                    service("feed", graphql!(r#"
            extend schema
              @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@external", "@provides"]
                )

              type Query {
                feed: [Post]
              }

              type Post @key(fields: "id") {
                id: ID!
                author: User @provides(fields: "userId")
              }

              extend type User @key(fields: "id") {
                id: ID! @external
              }
            "#, version)),
                ]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[feed] On field "Post.author", for @provides(fields: "userId"): Cannot query field "userId" on type "User" (if the field is defined in another subgraph, you need to add it to this subgraph with @external)."#,
                    )
                    .code("PROVIDES_INVALID_FIELDS")],
            );
        }
    }
}

test_versions! {
    fn empty_selection_set(api, version) {
        let result = api.compose_services(&[
            service("roles", graphql!(r#"
          type Query {
            team: Team! @provides(fields: "users")
          }

          type User @key(fields: "id") @extends {
            id: String! @external
            role: [Role!]
          }

          type Team @key(fields: "id") @extends {
            id: String! @external
            users: [User!]
          }

          type Role {
            id: ID!
            name: String
          }
        "#)),
            service("teams", graphql!(r#"
          type Team @key(fields: "id") {
            id: String!
            name: String
          }

          type User @key(fields: "id") {
            id: String!
            name: String
          }
        "#)),
            service("organization", graphql!(r#"
          type Query {
            organization(id: String!): Organization! @provides(fields: "teams")
          }

          type Organization {
            name: String
            teams: [Team!]!
          }

          type Team @key(fields: "id") @extends {
            id: String! @external
          }
        "#)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new()
                .message(
                    r#"[roles] On field "Query.team", for @provides(fields: "users"): Invalid empty selection set for field "Team.users" of non-leaf type [User!]"#,
                )
                .code("PROVIDES_INVALID_FIELDS")],
        );

        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new()
                .message(
                    r#"[organization] On field "Query.organization", for @provides(fields: "teams"): Invalid empty selection set for field "Organization.teams" of non-leaf type [Team!]!"#,
                )
                .code("PROVIDES_INVALID_FIELDS")],
        );
    }

    fn error_expected_abstract_indexes_to_be_defined(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable"]
            )

          type Query {
            a: Int!
          }

          interface SomeInterface {
            id: ID!
            someField: String!
          }

          type AAAAAAAAAAAAA implements SomeInterface @key(fields: "id") {
            id: ID!
            someField: String! @shareable
          }

          type BBBBBBBBBBBBB @shareable @key(fields: "id") {
            id: ID!
            aaaaaaaaaaaa: SomeInterface!
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@provides", "@shareable", "@external"]
            )

          type Query {
            b: Int!
          }

          type MutationResponse @shareable {
            bbbbbbbbbbbbb: BBBBBBBBBBBBB!
          }

          type BBBBBBBBBBBBB @key(fields: "id") @shareable {
            id: ID!
            aaaaaaaaaaaa: AAAAAAAAAAAAA!
          }

          type Mutation {
            mutationField: MutationResponse!
              @provides(fields: "bbbbbbbbbbbbb { aaaaaaaaaaaa { someField } }")
          }

          type AAAAAAAAAAAAA @key(fields: "id") {
            id: ID! @external
            someField: String! @external
          }
        "#)),
        ]);
        assert!(result.errors().is_none(), "expected no errors, received: {:?}", result.errors());
    }
}
