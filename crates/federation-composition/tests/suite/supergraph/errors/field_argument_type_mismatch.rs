//! Port of `__tests__/supergraph/errors/FIELD_ARGUMENT_TYPE_MISMATCH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn field_argument_type_mismatch(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users(type: UserType!): [User!]!
            }

            type User @key(fields: "id") {
              id: ID
              type: UserType
            }

            enum UserType {
              REGULAR
              ADMIN
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            extend type Query {
              users(type: String!): [User!]!
            }

            extend type User @key(fields: "id") {
              id: ID
              type: UserType
            }

            enum UserType {
              REGULAR
              ADMIN
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type of argument "Query.users(type:)" is incompatible across subgraphs: it has type "String!" in subgraph "feed" but type "UserType!" in subgraph "users""#)
                    .code("FIELD_ARGUMENT_TYPE_MISMATCH"),
            ],
        );
    }

    fn field_argument_type_mismatch_directive_definition_arguments_do_not_match(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: String) on FIELD

          type Query {
            b: Int
          }
          "#, version)),
        ]);

        assert_eq!(result.errors().map(|errors| errors.len()), Some(1));

        assert_errors_contain(
            &result.errors().unwrap()[..1],
            &[ErrorMatcher::new()
                .message(r#"Type of argument "@a(n:)" is incompatible across subgraphs: it has type "Int" in subgraph "a" but type "String" in subgraph "b""#)
                .code("FIELD_ARGUMENT_TYPE_MISMATCH")],
        );
    }

    fn field_argument_type_mismatch_directive_definition_arguments_do_not_match_3_different_definitions(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: String) on FIELD

          type Query {
            b: Int
          }
          "#, version)),
        ]);

        assert_eq!(result.errors().map(|errors| errors.len()), Some(1));

        assert_errors_contain(
            &result.errors().unwrap()[..1],
            &[ErrorMatcher::new()
                .message(r#"Type of argument "@a(n:)" is incompatible across subgraphs: it has type "Int" in subgraph "a" but type "String" in subgraph "b""#)
                .code("FIELD_ARGUMENT_TYPE_MISMATCH")],
        );
    }

    fn field_argument_type_mismatch_directive_definition_arguments_do_not_match_plural(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            b: Int
          }
          "#, version)),
            service_with_url("c", "http://c.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Boolean) on FIELD

          type Query {
            c: Int
          }
          "#, version)),
        ]);

        assert_eq!(result.errors().map(|errors| errors.len()), Some(1));

        assert_errors_contain(
            &result.errors().unwrap()[..1],
            &[ErrorMatcher::new()
                .message(r#"Type of argument "@a(n:)" is incompatible across subgraphs: it has type "Int" in subgraphs "a" and "b" but type "Boolean" in subgraph "c""#)
                .code("FIELD_ARGUMENT_TYPE_MISMATCH")],
        );
    }
}
