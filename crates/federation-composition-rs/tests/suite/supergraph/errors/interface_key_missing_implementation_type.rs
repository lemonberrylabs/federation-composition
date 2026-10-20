//! Port of `__tests__/supergraph/errors/INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn interface_key_missing_implementation_type(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User]
            }

            interface User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }
          "#, version)),
                service("extra", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type RegisteredUser implements User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }

            extend interface User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }
          "#, version)),
                service("blocked", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type BlockedUser implements User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }

            extend interface User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }
          "#, version)),
            ]),
            &if satisfies_version_range(">= v2.3", version) {
                vec![
                    // Federation should ignore a situation where an interface has a `@key` field but the subgraph doesn't implement the interface.
                    // It should check if at least one subgraph does it and if it does, then it should ignore the missing implementation (or create a hint instead).
                    ErrorMatcher::new()
                        .message_containing(r#"[users] Interface type "User" has a resolvable key (@key(fields: "id")) in subgraph "users" but that subgraph is missing some of the supergraph implementation types of "User". Subgraph "users" should define types "BlockedUser" and "RegisteredUser" (and have them implement "User")."#)
                        .code("INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE"),
                    ErrorMatcher::new()
                        .message_containing(r#"[extra] Interface type "User" has a resolvable key (@key(fields: "id")) in subgraph "extra" but that subgraph is missing some of the supergraph implementation types of "User". Subgraph "extra" should define type "BlockedUser" (and have it implement "User")."#)
                        .code("INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE"),
                    ErrorMatcher::new()
                        .message_containing(r#"[blocked] Interface type "User" has a resolvable key (@key(fields: "id")) in subgraph "blocked" but that subgraph is missing some of the supergraph implementation types of "User". Subgraph "blocked" should define type "RegisteredUser" (and have it implement "User")."#)
                        .code("INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE"),
                ]
            } else {
                vec![
                    ErrorMatcher::new()
                        .message_containing(r#"[users] Cannot use @key on interface "User": @key is not yet supported on interfaces"#)
                        .code("KEY_UNSUPPORTED_ON_INTERFACE"),
                ]
            },
        );

        assert_composition_success(&api.compose_services(&[
            service("a", graphql!(r#"
            type Query {
              a: String
            }

            extend interface Node @key(fields: "id") {
              id: ID!
              name: String
            }
          "#)),
            service("b", graphql!(r#"
            type Query {
              b: String
            }

            interface Node {
              id: ID!
            }

            type User implements Node @key(fields: "id") {
              id: ID!
              name: String
            }
          "#)),
        ]));
    }

    fn interface_key_missing_implementation_type_multiple_keys(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User]
            }

            interface User @key(fields: "email") @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }
          "#, version)),
                service("extra", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type RegisteredUser implements User @key(fields: "id") @key(fields: "email") {
              id: ID!
              name: String!
              email: String
            }

            extend interface User @key(fields: "id") @key(fields: "email") {
              id: ID!
              name: String!
              email: String
            }
          "#, version)),
                service("blocked", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type BlockedUser implements User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }

            extend interface User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }
          "#, version)),
            ]),
            &if satisfies_version_range(">= v2.3", version) {
                vec![
                    // Federation should ignore a situation where an interface has a `@key` field but the subgraph doesn't implement the interface.
                    // It should check if at least one subgraph does it and if it does, then it should ignore the missing implementation (or create a hint instead).
                    ErrorMatcher::new()
                        .message_containing(r#"[users] Interface type "User" has a resolvable key (@key(fields: "email")) in subgraph "users" but that subgraph is missing some of the supergraph implementation types of "User". Subgraph "users" should define types "BlockedUser" and "RegisteredUser" (and have them implement "User")."#)
                        .code("INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE"),
                    ErrorMatcher::new()
                        .message_containing(r#"[extra] Interface type "User" has a resolvable key (@key(fields: "id")) in subgraph "extra" but that subgraph is missing some of the supergraph implementation types of "User". Subgraph "extra" should define type "BlockedUser" (and have it implement "User")."#)
                        .code("INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE"),
                    ErrorMatcher::new()
                        .message_containing(r#"[blocked] Interface type "User" has a resolvable key (@key(fields: "id")) in subgraph "blocked" but that subgraph is missing some of the supergraph implementation types of "User". Subgraph "blocked" should define type "RegisteredUser" (and have it implement "User")."#)
                        .code("INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE"),
                ]
            } else {
                vec![
                    ErrorMatcher::new()
                        .message_containing(r#"[users] Cannot use @key on interface "User": @key is not yet supported on interfaces"#)
                        .code("KEY_UNSUPPORTED_ON_INTERFACE"),
                ]
            },
        );
    }

    fn interface_key_missing_implementation_type_interface_object_interface_is_valid(api, version) {
        assert_composition_success(&api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            interface Node @key(fields: "id") {
              id: ID!
            }

            type Query {
              a: String
            }
          "#)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type Node @key(fields: "id", resolvable: false) @interfaceObject {
              id: ID!
            }

            type Query {
              b: String
            }
          "#)),
        ]));
    }
}
