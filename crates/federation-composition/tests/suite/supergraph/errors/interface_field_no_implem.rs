//! Port of `__tests__/supergraph/errors/INTERFACE_FIELD_NO_IMPLEM.spec.ts`.

use crate::shared::*;

test_versions! {
    fn interface_field_no_implem_entity(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            type Query {
              users: [User]
            }

            type RegisteredUser implements User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }

            interface User {
              id: ID!
              name: String!
              email: String
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
            
            interface User {
              id: ID!
              name: String!
            }

            type Author implements User @key(fields: "id") {
              id: ID!
              name: String!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Interface field "User.email" is declared in subgraph "users" but type "Author", which implements "User" in subgraph "feed" does not have field "email"."#)
                    .code("INTERFACE_FIELD_NO_IMPLEM"),
            ],
        );
    }

    fn interface_field_no_implem_data(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("foo", graphql!(r#"
            type Query {
              foo: Foo
            }

            type Foo implements Person {
              name: String
              age: Int
            }

            interface Person {
              name: String
              age: Int
            }
          "#)),
                service("bar", graphql!(r#"
            type Query {
              bar: Bar
            }

            type Bar implements Person {
              name: String
            }

            interface Person {
              name: String
            }
          "#)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Interface field "Person.age" is declared in subgraph "foo" but type "Bar", which implements "Person" in subgraph "bar" does not have field "age"."#)
                    .code("INTERFACE_FIELD_NO_IMPLEM"),
            ],
        );
    }

    fn ignoring_interface_field_no_implem_when_type_kind_mismatch_detected(api, version) {
        let result = api.compose_services(&[
            service("cars", graphql!(r#"
          type Query {
            foo(id: ID!): String
          }

          interface Vehicle {
            id: ID!
          }

          type Car implements Vehicle {
            id: ID!
          }
        "#)),
            service("vehicles", graphql!(r#"
          type Vehicle {
            id: ID!
          }
        "#)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new()
                .message(r#"Type "Vehicle" has mismatched kind: it is defined as Interface Type in subgraph "cars" but Object Type in subgraph "vehicles""#)
                .code("TYPE_KIND_MISMATCH")],
        );
    }
}
