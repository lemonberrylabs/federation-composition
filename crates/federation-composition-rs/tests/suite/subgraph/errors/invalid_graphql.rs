//! Port of `__tests__/subgraph/errors/INVALID_GRAPHQL.spec.ts`.

use crate::shared::*;

test_versions! {
    fn invalid_graphql_invalid_value_for_fields_input_object(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.2"
                import: ["@key"]
              )

            directive @key(
              fields: FieldSet!
              resolvable: Boolean = true
            ) repeatable on OBJECT | INTERFACE

            input FieldSet {
              fields: [String!]!
            }

            type User @key(fields: "id name ") {
              id: ID!
              name: String
            }

            extend type Query {
              user(id: ID!): User
            }
          "#))]),
            &[ErrorMatcher::new()
                .message(r#"[users] Invalid value for "@key(fields:)" of type "FieldSet!" in application of "@key" to "User"."#)
                .code("INVALID_GRAPHQL")],
        );
    }

    fn invalid_graphql_provides_without_fields(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("billing", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@provides"]
              )

            type Payment @key(fields: "id") {
              id: ID!
              amount: Int!
            }

            type Invoice @key(fields: "id") {
              id: ID!
              amount: Int!
              payment: Payment @provides
            }
          "#, version))]),
            &[ErrorMatcher::new()
                // TODO: federation__FieldSet for Apollo `String!` for guild but it should be the same
                .message_containing(r#"Argument "@provides(fields:)" of type "FieldSet!" is required, but it was not provided."#)
                .code("INVALID_GRAPHQL")],
        );
    }

    fn invalid_graphql_unknown_type(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID
              profile: Profile
            }
          "#, version))]),
            &[ErrorMatcher::new().message_containing(r#"[users] Unknown type Profile"#).code("INVALID_GRAPHQL")],
        );
    }

    fn invalid_graphql_default_value(api, version) {
        let result = api.compose_services(&[service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key"]
            )

          type Query {
            users(role: Role = "Oopsie"): [User]
            allUsers(role: Role = OOPSIE): [User]
            usersByIDs(ids: [ID!]! = []): [User]
            usersByID(id: ID! = NULL): [User]
            usersByID2(id: ID! = null): [User]
            filterUsers(filter: Filter = {
              role: "Oopsie"
            }): [User]
          }

          input Filter {
            role: Role = OOPSIE
          }

          enum Role {
            ADMIN
            USER
          }

          type User @key(fields: "id") {
            id: ID
            name: String
          }
        "#, version))]);
        assert_composition_failure(&result);

        assert_errors_contain(
            &result,
            &[
                ErrorMatcher::new()
                    .message_containing(r#"[users] Invalid default value (got: "Oopsie") provided for argument Query.users(role:) of type Role."#)
                    .code("INVALID_GRAPHQL"),
                ErrorMatcher::new()
                    .message_containing(r#"[users] Invalid default value (got: OOPSIE) provided for argument Query.allUsers(role:) of type Role."#)
                    .code("INVALID_GRAPHQL"),
                ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Invalid default value (got: {role: "Oopsie"}) provided for argument Query.filterUsers(filter:) of type Filter."#,
                    )
                    .code("INVALID_GRAPHQL"),
                ErrorMatcher::new()
                    .message_containing(r#"[users] Invalid default value (got: OOPSIE) provided for input field Filter.role of type Role."#)
                    .code("INVALID_GRAPHQL"),
                ErrorMatcher::new()
                    .message_containing(r#"[users] Invalid default value (got: null) provided for argument Query.usersByID2(id:) of type ID!."#)
                    .code("INVALID_GRAPHQL"),
            ],
        );

        assert_no_error_matches(
            result.errors(),
            &ErrorMatcher::new().message_containing(r#"Query.usersByIDs(ids:)"#).code("INVALID_GRAPHQL"),
        );

        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new().message_containing(r#"Query.usersByID(id:)"#).code("INVALID_GRAPHQL")],
        );
    }

    fn invalid_graphql_compose_directive_gets_a_boolean_instead_of_string(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@composeDirective"]
              )
              @link(url: "https://myspecs.dev/hello/v1.0", import: ["@hello"])
              @composeDirective(name: true)

            directive @hello(name: String) on FIELD_DEFINITION

            type Query {
              words: [String!]! @hello(name: true)
            }
          "#, version))]),
            &if version == "v2.0" {
                // We don't want to follow Apollo's example here as returning the INVALID_LINK_DIRECTIVE_USAGE error makes more sense.
                vec![ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot import unknown element "@composeDirective"."#)
                    .code("INVALID_LINK_DIRECTIVE_USAGE")]
            } else {
                vec![
                    ErrorMatcher::new()
                        .message(
                            r#"[users] Invalid value for "@composeDirective(name:)" of type "String!" in application of "@composeDirective" to "schema"."#,
                        )
                        .code("INVALID_GRAPHQL"),
                    ErrorMatcher::new()
                        .message(r#"[users] Invalid value for "@hello(name:)" of type "String" in application of "@hello" to "Query.words"."#)
                        .code("INVALID_GRAPHQL"),
                ]
            },
        );
    }

    fn invalid_graphql_composed_directive_gets_an_int_instead_of_input_type(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@composeDirective"]
              )
              @link(url: "https://myspecs.dev/hello/v1.0", import: ["@hello", "Person"])
              @composeDirective(name: "@hello")

            directive @hello(person: Person) on FIELD_DEFINITION

            input Person {
              name: String!
            }

            type Query {
              words: [String!]! @hello(person: 123)
            }
          "#, version))]),
            &[if version == "v2.0" {
                // We don't want to follow Apollo's example here as returning the INVALID_LINK_DIRECTIVE_USAGE error makes more sense.
                ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot import unknown element "@composeDirective"."#)
                    .code("INVALID_LINK_DIRECTIVE_USAGE")
            } else {
                ErrorMatcher::new()
                    .message(r#"[users] Invalid value for "@hello(person:)" of type "Person" in application of "@hello" to "Query.words"."#)
                    .code("INVALID_GRAPHQL")
            }],
        );
    }

    fn invalid_graphql_composed_directive_gets_a_float_instead_of_input_type(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@composeDirective"]
              )
              @link(url: "https://myspecs.dev/hello/v1.0", import: ["@hello", "Person"])
              @composeDirective(name: "@hello")

            directive @hello(person: Person) on FIELD_DEFINITION

            input Person {
              name: String!
            }

            type Query {
              words: [String!]! @hello(person: 123.4)
            }
          "#, version))]),
            &[if version == "v2.0" {
                // We don't want to follow Apollo's example here as returning the INVALID_LINK_DIRECTIVE_USAGE error makes more sense.
                ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot import unknown element "@composeDirective"."#)
                    .code("INVALID_LINK_DIRECTIVE_USAGE")
            } else {
                ErrorMatcher::new()
                    .message(r#"[users] Invalid value for "@hello(person:)" of type "Person" in application of "@hello" to "Query.words"."#)
                    .code("INVALID_GRAPHQL")
            }],
        );
    }

    fn no_invalid_graphql_when_directive_gets_instead_of_string(api, version) {
        assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@shareable"]
            )

          directive @hello(names: [String]) on FIELD_DEFINITION

          type Query {
            words: [String!]! @shareable @hello(names: [])
          }
        "#, version))]));
    }

    fn directive_overwritten_specified_directive_like_deprecated_on_wrong_location(api, version) {
        assert_composition_failure(&api.compose_services(&[service("mono", graphql!(r#"
            schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"]) {
              query: Query
            }

            directive @deprecated(reason: String = "No longer supported") on FIELD_DEFINITION

            directive @shareable repeatable on OBJECT | FIELD_DEFINITION

            type Query @shareable {
              view(input: Input): View!
            }

            type View @shareable {
              user: String
              post: String!
            }

            input Input {
              input: String @deprecated(reason: "impossibru")
            }
          "#, version))]));
    }

    fn interface_field_object_field_subtype(api, version) {
        let result = api.compose_services(&[service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.6"
              import: ["@key"]
            )

          type Query {
            a: Admin
          }

          type Admin implements Node {
            id: ID
          }

          interface Node {
            id: ID!
          }
        "#))]);

        assert_composition_failure(&result);

        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new()
                .message("[a] Interface field Node.id expects type ID! but Admin.id of type ID is not a proper subtype.")
                .code("INVALID_GRAPHQL")],
        );
    }
}
