//! Port of `__tests__/supergraph/errors/DEFAULT_VALUE_USES_INACCESSIBLE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn default_value_uses_inaccessible(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )

            type Query {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
              friends(type: FriendType = FAMILY): [User!]!
            }

            enum FriendType {
              FAMILY @inaccessible
              FRIEND
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Enum value "FriendType.FAMILY" is @inaccessible but is used in the default value of "User.friends(type:)", which is in the API schema."#)
                    .code("DEFAULT_VALUE_USES_INACCESSIBLE"),
            ],
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@inaccessible"]
            )

          type Query {
            users: [User!]!
          }

          type User @key(fields: "id") {
            id: ID
            friends(type: FriendType = FAMILY @inaccessible): [User!]!
          }

          enum FriendType {
            FAMILY @inaccessible
            FRIEND
          }
         "#, version)),
            ])
                .supergraph_sdl()
                .is_some()
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@inaccessible"]
              )

            enum FooEnum @inaccessible {
              a @inaccessible
            }

            input FooInput @inaccessible {
              foo: FooEnum! = a @inaccessible
            }

            type Query {
              a: String!
              b(foo: FooInput @inaccessible): String! @inaccessible
            }
          "#, version)),
            ])
                .supergraph_sdl()
                .is_some()
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@inaccessible"]
              )

            enum FooEnum @inaccessible {
              a
            }

            input FooInput @inaccessible {
              foo: FooEnum! = a @inaccessible
            }

            type Query {
              a: String!
              b(foo: FooInput @inaccessible): String! @inaccessible
            }
          "#, version)),
            ])
                .supergraph_sdl()
                .is_some()
        );
    }
}
