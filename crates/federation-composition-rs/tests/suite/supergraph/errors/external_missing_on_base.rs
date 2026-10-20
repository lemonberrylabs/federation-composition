//! Port of `__tests__/supergraph/errors/EXTERNAL_MISSING_ON_BASE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn external_missing_on_base(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@requires"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID
              profile: Profile @external
              friends: [User] @requires(fields: "profile { name }")
            }

            type Profile {
              name: String
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Field "User.profile" is marked @external on all the subgraphs in which it is listed (subgraph "users")."#)
                    .code("EXTERNAL_MISSING_ON_BASE"),
            ],
        );

        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@requires", "@extends"])

            type Query {
              users: [User]
            }

            type User @extends @key(fields: "userId") {
              userId: String @external
              name: String @external
              fullName: String @requires(fields: "name")
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Field "User.name" is marked @external on all the subgraphs in which it is listed (subgraph "users")."#)
                    .code("EXTERNAL_MISSING_ON_BASE"),
            ],
        );

        assert_composition_success(&api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@extends"])

            type Query {
              users: [User]
            }

            type User @extends @key(fields: "id") {
              id: ID! @external
            }
          "#, version)),
        ]));
    }

    fn fed_v1_external_missing_on_base(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("a", graphql!(r#"
            type User @extends @key(fields: "id") {
              internalId: String @external
              id: ID! @external
              comments: [String] @requires(fields: "internalId")
            }

            type Query {
              word: String
            }
          "#)),
                service("b", graphql!(r#"
            type User @extends @key(fields: "id") {
              tags: [String!]!
              id: ID! @external
            }

            type Query {
              sentence: [String]
            }
          "#)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Field "User.internalId" is marked @external on all the subgraphs in which it is listed (subgraph "a")."#)
                    .code("EXTERNAL_MISSING_ON_BASE"),
            ],
        );

        assert_composition_success(&api.compose_services(&[
            service("a", graphql!(r#"
            type User @extends @key(fields: "id") {
              internalId: String @external
              id: ID! @external
            }

            type Query {
              word: String
            }
          "#)),
            service("b", graphql!(r#"
            type User @extends @key(fields: "id") {
              tags: [String!]!
              id: ID! @external
            }

            type Query {
              sentence: [String]
            }
          "#)),
        ]));
    }

    fn field_can_be_external_only_if_it_is_a_key_and_the_object_type_uses_extend(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external", "@requires"]
              )

            extend type SharedType @key(fields: "id") {
              id: ID! @external
              dependency: String! @external
            }

            extend type SharedType @key(fields: "secondId") {
              secondId: ID! @external
              someFieldThatRequiresDependency: String
                @requires(fields: "dependency")
            }

            type Query {
              a: String!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type SharedType @key(fields: "secondId") {
              type: String!
              secondId: ID!
              dependency: String!
            }

            type Query {
              c: String!
            }
          "#, version)),
        ]);

        assert!(result.errors().is_none());
    }

    fn external_in_types_defined_by_interface_object(api, version) {
        if satisfies_version_range("< v2.3", version) {
            // @interfaceObject is not supported before v2.3
            return;
        }

        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@interfaceObject"]
            )

          type Query {
            animal: Animal
          }

          type Animal @interfaceObject @key(fields: "id") {
            id: ID!
            age: Int
          }
        "#, version)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external"]
            )

          interface Mammal {
            id: ID!
            age: Int
          }

          interface Animal implements Mammal @key(fields: "id") {
            id: ID!
            origin: String
            age: Int
          }

          type Dog implements Mammal & Animal @key(fields: "id") {
            id: ID!
            origin: String
            age: Int @external
          }
        "#, version)),
        ]);

        assert_composition_success(&result);
    }
}
