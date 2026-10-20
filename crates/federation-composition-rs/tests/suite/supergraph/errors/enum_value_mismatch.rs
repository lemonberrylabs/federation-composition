//! Port of `__tests__/supergraph/errors/ENUM_VALUE_MISMATCH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn enum_value_mismatch(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID
              type: UserType
            }

            enum UserType {
              REGULAR
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              usersByType(type: UserType): [User!]!
            }

            extend type User @key(fields: "id") {
              id: ID
              type: UserType
            }

            enum UserType {
              ANONYMOUS
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Enum type "UserType" is used as both input type (for example, as type of "Query.usersByType(type:)") and output type (for example, as type of "User.type"), but value "ANONYMOUS" is not defined in all the subgraphs defining "UserType": "ANONYMOUS" is defined in subgraph "feed" but not in subgraph "users""#)
                    .code("ENUM_VALUE_MISMATCH"),
                ErrorMatcher::new()
                    .message_containing(r#"Enum type "UserType" is used as both input type (for example, as type of "Query.usersByType(type:)") and output type (for example, as type of "User.type"), but value "REGULAR" is not defined in all the subgraphs defining "UserType": "REGULAR" is defined in subgraph "users" but not in subgraph "feed""#)
                    .code("ENUM_VALUE_MISMATCH"),
            ],
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@shareable"])
            
            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID
              type: UserType @shareable
            }

            enum UserType {
              REGULAR
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@inaccessible", "@shareable"])
            
            type Query {
              usersByType(type: UserType): [User!]!
            }

            extend type User @key(fields: "id") {
              id: ID
              type: UserType @shareable
            }

            enum UserType {
              ANONYMOUS @inaccessible
              REGULAR
            }
          "#, version)),
            ])
                .supergraph_sdl()
                .is_some()
        );
    }
}
