//! Port of `__tests__/composition.spec.ts`.

use crate::shared::*;
use federation_composition_rs::specifications::federation::FederationVersion;
use federation_composition_rs::specifications::tag::DIRECTIVE as TAG_DIRECTIVE;
use federation_composition_rs::specifications::{join, link};
use graphql_rs::Definition;

/// `JSON.stringify(result.errors)` in a failure message: `undefined` when there are no
/// errors, otherwise each error's `message` and `extensions.code` (`GraphQLError.toJSON`
/// can add `locations` and other extensions; this only changes the text of a failing test).
fn json_stringify_errors(errors: Option<&Vec<GraphQLError>>) -> String {
    match errors {
        None => "undefined".to_string(),
        Some(errors) => serde_json::Value::Array(
            errors
                .iter()
                .map(|e| serde_json::json!({ "message": e.message, "extensions": { "code": e.code() } }))
                .collect(),
        )
        .to_string(),
    }
}

test_implementations! {
    fn enum_value_as_string_as_default_value(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String
          }

          type Query {
            users(type: UserType! = "Regular"): [User!]!
          }

          enum UserType {
            Regular
            Admin
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query @join__type(graph: A) {
        users(type: UserType! = Regular): [User!]!
      }
    "#);
    }

    fn duplicated_query_fields(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String
          }

          type Query {
            userById(id: ID!): User
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String
          }

          type Query {
            userById(id: ID!): User
          }
        "#)),
        ]);

        assert_composition_failure(&result);
    }

    fn join_field_used_overridden_true_when_field_is_overridden_but_defined_in_an_interface(api) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key"]
            )

          interface Post {
            id: ID!
            createdAt: String!
          }

          type ImagePost implements Post @key(fields: "id") {
            id: ID!
            createdAt: String!
          }

          type Query {
            feed: [Post]
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable", "@override"]
            )

          interface Post {
            id: ID!
            createdAt: String!
          }

          type TextPost implements Post @key(fields: "id") {
            id: ID!
            createdAt: String!
            body: String!
          }

          interface AnotherPost {
            id: ID!
            createdAt: String!
          }

          type ImagePost implements AnotherPost @key(fields: "id") {
            id: ID!
            createdAt: String! @override(from: "a")
          }

          type Query {
            anotherFeed: [AnotherPost]
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type ImagePost implements Post & AnotherPost
        @join__implements(graph: A, interface: "Post")
        @join__implements(graph: B, interface: "AnotherPost")
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id") {
        createdAt: String!
          @join__field(graph: A, usedOverridden: true)
          @join__field(graph: B, override: "a")
        id: ID!
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key"]
            )

          type ImagePost @key(fields: "id") {
            id: ID!
            createdAt: String!
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable", "@override"]
            )

          interface Post {
            id: ID!
            createdAt: String!
          }

          type TextPost implements Post @key(fields: "id") {
            id: ID!
            createdAt: String!
            body: String!
          }

          interface AnotherPost {
            id: ID!
            createdAt: String!
          }

          type ImagePost implements AnotherPost @key(fields: "id") {
            id: ID!
            createdAt: String! @override(from: "a")
          }

          type Query {
            anotherFeed: [AnotherPost]
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type ImagePost implements AnotherPost
        @join__implements(graph: B, interface: "AnotherPost")
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id") {
        createdAt: String! @join__field(graph: B, override: "a")
        id: ID!
      }
    "#);
    }

    fn join_field_external_true_when_field_is_overridden(api) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@requires", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
            aName: String! @requires(fields: "name")
          }

          type Query {
            userA: User
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@override"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @override(from: "c")
          }

          type Query {
            userB: User
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@requires", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
            cName: String! @requires(fields: "name")
          }

          type Query {
            userC: User
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        id: ID!
        name: String!
          @join__field(external: true, graph: A)
          @join__field(external: true, graph: C)
          @join__field(graph: B, override: "c")
        aName: String! @join__field(graph: A, requires: "name")
        cName: String! @join__field(graph: C, requires: "name")
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@requires", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
            aName: String! @requires(fields: "name")
          }

          type Query {
            userA: User
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@override"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @override(from: "c")
          }

          type Query {
            userB: User
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@requires", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            cName: String!
          }

          type Query {
            userC: User
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        id: ID!
        name: String!
          @join__field(external: true, graph: A)
          @join__field(graph: B, override: "c")
        aName: String! @join__field(graph: A, requires: "name")
        cName: String! @join__field(graph: C)
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@requires", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
            aName: String! @requires(fields: "name")
          }

          type Query {
            userA: User
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@override"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @override(from: "c")
          }

          type Query {
            userB: User
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@requires", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
            cName: String! @requires(fields: "name")
          }

          type Query {
            userC: User
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        id: ID!
        name: String!
          @join__field(external: true, graph: A)
          @join__field(external: true, graph: C)
          @join__field(graph: B, override: "c")
        aName: String! @join__field(graph: A, requires: "name")
        cName: String! @join__field(graph: C, requires: "name")
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@requires", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
            aName: String! @requires(fields: "name")
          }

          type Query {
            userA: User
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@override"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String! @override(from: "c")
          }

          type Query {
            userB: User
          }
        "#)),
            service("c", graphql!(r#"
          type User @key(fields: "id") {
            id: ID!

            name: String! @external
          }

          type Foo @key(fields: "id") {
            id: ID!
            foo: String
          }

          scalar _Any
          union _Entity = Foo | User

          type _Service {
            sdl: String!
          }

          type Query {
            _entities(representations: [_Any!]!): [_Entity]!
            _service: _Service
            c: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        id: ID!
        name: String!
          @join__field(external: true, graph: A)
          @join__field(graph: B, override: "c")
        aName: String! @join__field(graph: A, requires: "name")
      }
    "#);
    }

    fn override_a_field_with_requires(api) {
        let result = api.compose_services(&[
            service("core", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable"]
            )

          type Query {
            product(id: ID!): Product
          }

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @shareable
          }
        "#)),
            service("original", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external", "@requires", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @external
            paymentReceipt: String!
              @shareable
              @requires(fields: "creditCardNumber")
          }
        "#)),
            service("new", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@override", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @shareable
            paymentReceipt: String! @shareable @override(from: "original")
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Product
        @join__type(graph: CORE, key: "id")
        @join__type(graph: NEW, key: "id")
        @join__type(graph: ORIGINAL, key: "id") {
        id: ID!
        creditCardNumber: String!
          @join__field(graph: CORE)
          @join__field(graph: NEW)
        paymentReceipt: String! @join__field(graph: NEW, override: "original")
      }
    "#);
    }

    fn print_join_field_on_shareable_field_defined_twice_once_with_non_effective_override(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          type Mutation {
            a: String!
            b: String!
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@shareable", "@override"]
            )

          type Mutation {
            a: String! @shareable @override(from: "non-existing")
            b: String! @shareable @override(from: "non-existing")
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key"]
            )

          type Mutation {
            c: String!
          }

          type Query {
            c: String
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Mutation
        @join__type(graph: A)
        @join__type(graph: B)
        @join__type(graph: C) {
        a: String!
          @join__field(graph: A)
          @join__field(graph: B, override: "non-existing")
        b: String!
          @join__field(graph: A)
          @join__field(graph: B, override: "non-existing")
        c: String! @join__field(graph: C)
      }
    "#);
    }
}

mod shareable {
    use super::*;

    test_versions! {
        fn merge_two_exact_same_types(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type User @shareable {
                name: String
              }

              type Query {
                user: User
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type User @shareable {
                name: String
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type User @join__type(graph: A) @join__type(graph: B) {
            name: String
          }
        "#);
        }

        fn merge_same_type_with_same_fields_by_of_different_return_types_nullable_vs_non_nullable(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type User @shareable {
                name: String
              }

              type Query {
                user: User
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type User @shareable {
                name: String!
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type User @join__type(graph: A) @join__type(graph: B) {
            name: String
              @join__field(graph: B, type: "String!")
              @join__field(graph: A, type: "String")
          }
        "#);
        }

        fn merge_same_types_but_one_has_an_extension(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type User @shareable {
                name: String
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type User @shareable {
                name: String
              }

              extend type User {
                email: String! @shareable
              }

              type Query {
                user: User
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type User @join__type(graph: A) @join__type(graph: B) {
            name: String
            email: String! @join__field(graph: B)
          }
        "#);
        }

        fn merge_same_types_but_one_has_an_extra_field(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Food @shareable {
                name: String!
                price: Int!
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Food @shareable {
                name: String!
                price: Int!
                inStock: Boolean! # Not in A
              }

              type Query {
                food: Food
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Food @join__type(graph: A) @join__type(graph: B) {
            name: String!
            inStock: Boolean! @join__field(graph: B)
            price: Int!
          }
        "#);
        }

        fn merge_extension_and_definition_of_object_type_and_print_extension_true(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

                type User @key(fields: "id") @shareable {
                  id: ID!
                  name: String
                }

                type Query {
                  user: User @shareable
                }
              "#, version)),
                service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

                type User @key(fields: "id") @shareable {
                  id: ID!
                  name: String
                }

                extend type User {
                  email: String! @shareable
                }

                type Query {
                  user: User @shareable
                }
              "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type User
            @join__type(graph: A, key: "id")
            @join__type(graph: B, key: "id") {
            email: String! @join__field(graph: B)
            id: ID!
            name: String
          }
        "#);
        }

        fn merge_same_types_but_one_has_an_extra_field_inaccessible(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Food @shareable {
                name: String!
                price: Int!
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@shareable", "@inaccessible"]
                )

              type Food @shareable {
                name: String!
                price: Int!
                inStock: Boolean! @inaccessible
              }

              type Query {
                food: Food
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Food @join__type(graph: A) @join__type(graph: B) {
            name: String!
            price: Int!
            inStock: Boolean! @inaccessible @join__field(graph: B)
          }
        "#);

            assert_not_to_match_string(result.public_sdl(), "@inaccessible");
        }

        fn type_annotated_with_inaccessible_should_not_result_in_inaccessible_on_its_fields(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@inaccessible"])

              type Food @inaccessible {
                name: String!
                price: Int!
              }

              type Drink @inaccessible {
                name: String!
                price: Int! @inaccessible
              }

              type Query {
                words: [String!]!
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Food @join__type(graph: A) @inaccessible {
            name: String!
            price: Int!
          }
        "#);
            to_contain_graphql(result.supergraph_sdl(), r#"
          type Drink @join__type(graph: A) @inaccessible {
            name: String!
            price: Int! @inaccessible
          }
        "#);
        }

        fn merge_an_argument_nullable_vs_missing(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Building @shareable {
                # Argument is optional
                height(units: String): Int!
              }

              type Query {
                building: Building
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Building @shareable {
                # Argument is missing
                height: Int!
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Building @join__type(graph: B) @join__type(graph: A) {
            height: Int!
          }
        "#);
        }

        fn merge_an_argument_non_nullable_vs_missing(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Building @shareable {
                # Argument is required
                height(units: String!): Int!
              }

              type Query {
                building: Building
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Building @shareable {
                # Argument is missing
                height: Int!
              }
            "#, version)),
            ]);

            assert_composition_failure(&result);
        }

        fn merge_an_argument_nullable_vs_non_nullable(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Building @shareable {
                # Argument is required
                height(units: String!): Int!
              }

              type Query {
                building: Building
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              type Building @shareable {
                # Argument can be optional
                height(units: String): Int!
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Building @join__type(graph: A) @join__type(graph: B) {
            height(units: String!): Int!
          }
        "#);
        }
    }
}

test_versions! {
    fn merge_object_types_with_different_fields(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "id") {
              id: ID!
              name: String!
              email: String!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "id") {
              id: ID!
              age: Int!
            }

            type Query {
              user: User
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: B, key: "id")
          @join__type(graph: A, key: "id") {
          id: ID!
          email: String! @join__field(graph: A)
          name: String! @join__field(graph: A)
          age: Int! @join__field(graph: B)
        }
      "#);
    }

    fn merge_object_extension_type_and_non_resolvable_object_type(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

              extend type User @key(fields: "id") {
                id: ID @external
              }

              type Query {
                user: User
              }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              users: [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id", extension: true)
          @join__type(graph: B, key: "id") {
          id: ID
            @join__field(graph: A, type: "ID")
            @join__field(graph: B, type: "ID!")
        }
      "#);
    }

    fn fed_v1_merge_object_extension_type_and_non_resolvable_object_type(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend type User @key(fields: "id") {
              id: ID @external
            }

            type Query {
              user: User
            }
          "#)),
            service("b", graphql!(r#"
            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              users: [User]
            }
          "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID
            @join__field(graph: A, type: "ID")
            @join__field(graph: B, type: "ID!")
        }
      "#);
    }

    fn merge_interface_types_with_different_fields(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "id") {
              id: ID!
              name: String!
              email: String!
            }

            interface Details {
              name: ID!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "id") {
              id: ID!
              age: Int!
            }

            interface Details {
              email: String
            }

            type Query {
              user: User
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        interface Details @join__type(graph: A) @join__type(graph: B) {
          name: ID! @join__field(graph: A)
          email: String @join__field(graph: B)
        }
      "#);
    }

    fn merge_union_types_with_different_members_some_are_overlapping(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type User @key(fields: "id") {
              id: ID!
              name: String!
              email: String!
            }

            union Media = Book

            type Book @shareable {
              title: String!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type User @key(fields: "id") {
              id: ID!
              age: Int!
            }

            union Media = Movie | Book

            type Movie {
              title: String!
            }

            type Book @shareable {
              title: String!
            }

            type Query {
              user: User
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        union Media
          @join__type(graph: A)
          @join__type(graph: B)
          @join__unionMember(graph: A, member: "Book")
          @join__unionMember(graph: B, member: "Movie")
          @join__unionMember(graph: B, member: "Book") =
          | Movie
          | Book
      "#);
    }

    fn merge_union_types_with_different_members(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type User @key(fields: "id") {
              id: ID!
              name: String!
              email: String!
            }

            union Media = Book

            type Book @shareable {
              title: String!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type User @key(fields: "id") {
              id: ID!
              age: Int!
            }

            union Media = Movie

            type Movie {
              title: String!
            }

            type Query {
              user: User
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        union Media
          @join__type(graph: A)
          @join__type(graph: B)
          @join__unionMember(graph: A, member: "Book")
          @join__unionMember(graph: B, member: "Movie") =
          | Book
          | Movie
      "#);
    }

    fn merge_input_types_and_field_arguments(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            input UserInput {
              name: String
              age: Int
            }

            type Library @shareable {
              book(title: String, author: String): String
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            input UserInput {
              name: String!
              email: String
            }

            type Library @shareable {
              book(title: String, section: String): String
            }

            type Query {
              library: Library
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Library @join__type(graph: A) @join__type(graph: B) {
          book(title: String): String
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        input UserInput @join__type(graph: A) @join__type(graph: B) {
          name: String!
            @join__field(graph: A, type: "String")
            @join__field(graph: B, type: "String!")
        }
      "#);
    }
}

mod default_values {
    use super::*;

    test_versions! {
        fn print(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              input LibraryFilter {
                limit: Int = 5
              }

              enum LibraryAccess {
                PUBLIC
                PRIVATE
              }

              type Query {
                books(filter: LibraryFilter, access: LibraryAccess = PUBLIC): [String!]!
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              input UserFilter {
                limit: Int = 10
              }

              enum UserAccess {
                PUBLIC
                PRIVATE
              }

              type Query {
                users(filter: UserFilter, access: UserAccess = PRIVATE): [String!]!
              }
            "#, version)),
                service("c", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

              input MediaFilter {
                limit: Int = 15
              }

              enum MediaAccess {
                PUBLIC
                PRIVATE
              }

              interface Media {
                records(filter: MediaFilter, access: MediaAccess = PRIVATE): [String!]!
              }

              type Movie implements Media {
                records(filter: MediaFilter, access: MediaAccess = PUBLIC): [String!]!
              }

              type Query {
                media(filter: MediaFilter, access: MediaAccess = PRIVATE): [Media!]!
              }
            "#, version)),
                service("d", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.5"
                  import: ["@key", "@composeDirective"]
                )
                @link(
                  url: "https://myspecs.dev/access/v1.0"
                  import: ["@access"]
                )
                @composeDirective(name: "@access")

              directive @access(scope: Scope! = PUBLIC) on FIELD_DEFINITION
              enum Scope {
                PUBLIC
                PRIVATE
              }

              type Query {
                hello: String @access
              }
            "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          input LibraryFilter @join__type(graph: A) {
            limit: Int = 5
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          input UserFilter @join__type(graph: B) {
            limit: Int = 10
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          input MediaFilter @join__type(graph: C) {
            limit: Int = 15
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          interface Media @join__type(graph: C) {
            records(
              filter: MediaFilter
              access: MediaAccess = PRIVATE
            ): [String!]!
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Movie implements Media
            @join__implements(graph: C, interface: "Media")
            @join__type(graph: C) {
            records(
              filter: MediaFilter
              access: MediaAccess = PUBLIC
            ): [String!]!
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          directive @access(scope: Scope! = PUBLIC) on FIELD_DEFINITION
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Query
            @join__type(graph: A)
            @join__type(graph: B)
            @join__type(graph: C)
            @join__type(graph: D) {
            books(
              filter: LibraryFilter
              access: LibraryAccess = PUBLIC
            ): [String!]! @join__field(graph: A)
            users(filter: UserFilter, access: UserAccess = PRIVATE): [String!]!
              @join__field(graph: B)
            media(
              filter: MediaFilter
              access: MediaAccess = PRIVATE
            ): [Media!]! @join__field(graph: C)
            hello: String @access @join__field(graph: D)
          }
        "#);
        }

        fn collision_with_missing_default_value_on_directive_argument(api, version) {
            // @composeDirective requires v2.1+
            if version == "v2.0" {
                return;
            }
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(
                  url: "https://myspecs.dev/access/v1.0"
                  import: ["@access"]
                )
                @composeDirective(name: "@access")

              directive @access(scope: Scope!) on FIELD_DEFINITION
              enum Scope {
                PUBLIC
                PRIVATE
              }

              type Query {
                a: String @access
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(
                  url: "https://myspecs.dev/access/v1.0"
                  import: ["@access"]
                )
                @composeDirective(name: "@access")

              directive @access(scope: Scope! = PUBLIC) on FIELD_DEFINITION
              enum Scope {
                PUBLIC
                PRIVATE
              }

              type Query {
                b: String @access
              }
            "#, version)),
            ]);

            assert_composition_failure(&result);
            assert_eq!(result.errors().unwrap().len(), 1);
            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message_containing(r#"Argument "@access(scope:)" of type "Scope!" is required, but it was not provided."#)]);
        }

        fn collision_with_different_default_value_on_directive_argument_for_schema_directive(api, version) {
            // @composeDirective requires v2.1+
            if version == "v2.0" {
                return;
            }

            let result = api.compose_services(&[
                service("a", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@composeDirective"]
                  )
                  @link(
                    url: "https://myspecs.dev/access/v1.0"
                    import: ["@access"]
                  )
                  @composeDirective(name: "@access")

                directive @access(scope: Scope! = PRIVATE) on FIELD_DEFINITION

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  a: String @access
                }
              "#, version)),
                service("b", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@composeDirective"]
                  )
                  @link(
                    url: "https://myspecs.dev/access/v1.0"
                    import: ["@access"]
                  )
                  @composeDirective(name: "@access")

                directive @access(scope: Scope! = PUBLIC) on FIELD_DEFINITION

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  b: String @access
                }
              "#, version)),
            ]);
            // This fails actually.
            assert_composition_success(&result);
            to_contain_graphql(result.supergraph_sdl(), r#"directive @access(scope: Scope! = PUBLIC) on FIELD_DEFINITION"#);
        }

        fn collision_with_different_default_value_on_directive_argument_for_executable_directive(api, version) {
            // @composeDirective requires v2.1+
            if version == "v2.0" {
                return;
            }

            let result = api.compose_services(&[
                service("a", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@composeDirective"]
                  )
                  @link(
                    url: "https://myspecs.dev/access/v1.0"
                    import: ["@access"]
                  )
                  @composeDirective(name: "@access")

                directive @access(scope: Scope! = PRIVATE) on QUERY

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  a: String
                }
              "#, version)),
                service("b", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@composeDirective"]
                  )
                  @link(
                    url: "https://myspecs.dev/access/v1.0"
                    import: ["@access"]
                  )
                  @composeDirective(name: "@access")

                directive @access(scope: Scope! = PUBLIC) on QUERY

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  b: String
                }
              "#, version)),
            ]);
            // This fails actually.
            assert_composition_success(&result);
            to_contain_graphql(result.supergraph_sdl(), r#"directive @access(scope: Scope! = PUBLIC) on QUERY"#);
        }

        fn collision_with_missing_default_value_on_directive_argument_for_executable_directive(api, version) {
            // @composeDirective requires v2.1+
            if version == "v2.0" {
                return;
            }

            let result = api.compose_services(&[
                service("a", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@composeDirective"]
                  )
                  @link(
                    url: "https://myspecs.dev/access/v1.0"
                    import: ["@access"]
                  )
                  @composeDirective(name: "@access")

                directive @access(scope: Scope!) on QUERY

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  a: String
                }
              "#, version)),
                service("b", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@composeDirective"]
                  )
                  @link(
                    url: "https://myspecs.dev/access/v1.0"
                    import: ["@access"]
                  )
                  @composeDirective(name: "@access")

                directive @access(scope: Scope! = PUBLIC) on QUERY

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  b: String
                }
              "#, version)),
            ]);
            // This fails actually.
            assert_composition_success(&result);
            to_contain_graphql(result.supergraph_sdl(), r#"directive @access(scope: Scope! = PUBLIC) on QUERY"#);
        }

        fn collision_with_different_default_value_on_argument_on_field_argument(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}")

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  hello(scope: Scope! = PUBLIC): String
                }
              "#, version)),
                service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}")

                enum Scope {
                  PUBLIC
                  PRIVATE
                }

                type Query {
                  hello(scope: Scope! = PRIVATE): String
                }
              "#, version)),
            ]);

            assert_composition_failure(&result);
            assert_eq!(result.errors().unwrap().len(), 2);
            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message_containing(r#""Query.hello(scope:)" has incompatible default values across subgraphs:"#)]);
            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message_containing(r#"Non-shareable field "Query.hello" is resolved from multiple subgraphs"#)]);
        }
    }
}

test_versions! {
    fn merge_enum_type_types_when_used_as_the_return_type_for_at_least_one_object_or_interface_field(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String!
              type: UserType
            }

            enum UserType {
              ADMIN
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String!
              type: UserType
            }

            enum UserType {
              REGULAR
            }

            type Query {
              users: [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @join__type(graph: B) @join__type(graph: A) {
          REGULAR @join__enumValue(graph: B)
          ADMIN @join__enumValue(graph: A)
        }
      "#);
    }

    fn merge_enum_type_types_when_used_as_the_type_for_at_least_one_field_argument_or_input_type_field(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String!
            }

            enum UserType {
              ADMIN
              REGULAR
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String!
            }

            enum UserType {
              REGULAR
              ANONYMOUS
            }

            type Query {
              users(type: UserType): [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @join__type(graph: A) @join__type(graph: B) {
          REGULAR @join__enumValue(graph: A) @join__enumValue(graph: B)
        }
      "#);
    }

    fn merge_enum_type_types_when_used_as_the_type_for_at_least_one_field_argument_or_input_type_field_and_as_the_return_type_for_at_least_one_object_or_interface_field(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String!
              type: UserType
            }

            enum UserType {
              ADMIN
              REGULAR
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String!
              type: UserType
            }

            enum UserType {
              REGULAR
              ADMIN
            }

            type Query {
              users(type: UserType): [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @join__type(graph: A) @join__type(graph: B) {
          ADMIN @join__enumValue(graph: A) @join__enumValue(graph: B)
          REGULAR @join__enumValue(graph: A) @join__enumValue(graph: B)
        }
      "#);
    }

    fn ignore_directive_if_not_defined_by_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on FIELD_DEFINITION

            type User @shareable {
              name: String! @lowercase
            }

            type Query {
              users: [User]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @uppercase on FIELD_DEFINITION

            type Review @shareable {
              name: String! @uppercase
            }

            type Query {
              reviews: [Review]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        assert_not_to_match_string(result.supergraph_sdl(), "lowercase");
        assert_not_to_match_string(result.supergraph_sdl(), "uppercase");
        assert_not_to_match_string(result.public_sdl(), "lowercase");
    }

    fn ignore_directive_if_identically_defined_in_all_subgraphs(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on FIELD_DEFINITION

            type User @shareable {
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on FIELD_DEFINITION

            type Review @shareable {
              name: String! @lowercase
            }

            type Query {
              reviews: [Review]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        assert_not_to_match_string(result.supergraph_sdl(), "lowercase");
        assert_not_to_match_string(result.public_sdl(), "lowercase");
    }

    fn ignore_directive_if_not_defined_in_all_subgraphs(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on FIELD_DEFINITION

            type User @shareable {
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String!
            }

            type Query {
              users: [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        assert_not_to_match_string(result.supergraph_sdl(), "lowercase");
        assert_not_to_match_string(result.public_sdl(), "lowercase");
    }

    fn ignore_directive_if_defined_differently_across_subgraphs(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on FIELD_DEFINITION

            type User @shareable {
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on OBJECT

            type User @shareable @lowercase {
              name: String!
            }

            type Query {
              users: [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        assert_not_to_match_string(result.supergraph_sdl(), "lowercase");
        assert_not_to_match_string(result.public_sdl(), "lowercase");
    }

    fn ignore_directive_if_it_has_different_locations_across_subgraphs(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on FIELD_DEFINITION

            type User @shareable {
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            directive @lowercase on OBJECT | FIELD_DEFINITION

            type User @shareable @lowercase {
              name: String!
            }

            type Query {
              users: [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        assert_not_to_match_string(result.supergraph_sdl(), "lowercase");
        assert_not_to_match_string(result.public_sdl(), "lowercase");
    }

    fn support_compose_directive_when_using_multiple_schema_extensions(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
              extend schema
                @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
              extend schema
                @composeDirective(name: "@lowercase")

              directive @lowercase on FIELD_DEFINITION

              type User @key(fields: "id") {
                id: ID!
                name: String! @lowercase
              }

              type Query {
                users: [User]
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type Review @key(fields: "id") {
                id: ID!
                name: String!
              }

              extend type User @key(fields: "id") {
                id: ID!
                reviews: [Review]
              }

              type Query {
                review: [Review]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @lowercase on FIELD_DEFINITION
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @lowercase on FIELD_DEFINITION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true) {
          id: ID!
          name: String! @lowercase @join__field(graph: A)
          reviews: [Review] @join__field(graph: B)
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User {
          id: ID!
          name: String! @lowercase
          reviews: [Review]
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION)
          @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"]) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn preserve_directive_if_not_defined_in_all_subgraphs_but_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
                @composeDirective(name: "@lowercase")

              directive @lowercase on FIELD_DEFINITION

              type User @key(fields: "id") {
                id: ID!
                name: String! @lowercase
              }

              type Query {
                users: [User]
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type Review @key(fields: "id") {
                id: ID!
                name: String!
              }

              extend type User @key(fields: "id") {
                id: ID!
                reviews: [Review]
              }

              type Query {
                review: [Review]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @lowercase on FIELD_DEFINITION
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @lowercase on FIELD_DEFINITION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true) {
          id: ID!
          name: String! @lowercase @join__field(graph: A)
          reviews: [Review] @join__field(graph: B)
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User {
          id: ID!
          name: String! @lowercase
          reviews: [Review]
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION)
          @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"]) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn preserve_directive_if_not_applied_on_a_field_in_all_subgraphs_but_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("b", graphql!(r#"
              directive @lowercase on FIELD_DEFINITION

              extend type Query {
                " users "
                users: [String] @lowercase @override(from: "a")
              }

              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective", "@override"]
                )
                @link(url: "https://specs.community.graphql.org/lowercase/v1.0", import: ["@lowercase"])
                @composeDirective(name: "@lowercase")
            "#, version)),
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.0"
                import: ["@key"]
              )

            type Query {
              users: [String]
            }
          "#)),
            service("c", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.0"
                import: ["@key"]
              )

            extend type Query {
              comments: [String]
            }
          "#)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @lowercase on FIELD_DEFINITION
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
          directive @lowercase on FIELD_DEFINITION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Query
          @join__type(graph: A)
          @join__type(graph: B)
          @join__type(graph: C) {
          comments: [String] @join__field(graph: C)
          """
          users
          """
          users: [String] @join__field(graph: B, override: "a") @lowercase
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Query {
          comments: [String]
          """
          users
          """
          users: [String] @lowercase
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION)
          @link(url: "https://specs.community.graphql.org/lowercase/v1.0", import: ["@lowercase"]) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn ignore_directive_if_defined_identically_in_all_subgraphs_and_not_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

              directive @lowercase on FIELD_DEFINITION

              type User @key(fields: "id") {
                id: ID!
                name: String! @lowercase
              }

              type Query {
                users: [User]
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

              directive @lowercase on FIELD_DEFINITION

              type Comment @key(fields: "id") {
                id: ID!
                name: String! @lowercase
              }

              type Query {
                comments: [Comment]
              }
            "#, version)),
        ]);

        assert_composition_success(&result);
        assert_not_to_match_string(result.supergraph_sdl(), "lowercase");
        assert_not_to_match_string(result.public_sdl(), "lowercase");
    }

    fn requires_on_a_union_with_typename_fails(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type Query {
              product(id: ID!): Product
            }

            type Product @key(fields: "id") {
              id: ID!
              description: Description!
            }

            union Description = DescriptionLong | DescriptionShort

            type DescriptionLong {
              value: String!
            }

            type DescriptionShort {
              value: String!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external", "@requires"]
              )

            type Product @key(fields: "id") {
              id: ID!
              description: Description! @external
              calculatedField: String!
                @requires(
                  fields: "description { __typename ... on DescriptionLong { value } ... on DescriptionShort {value}} "
                )
            }

            union Description = DescriptionLong | DescriptionShort

            type DescriptionLong {
              value: String! @external
            }

            type DescriptionShort {
              value: String! @external
            }
          "#, version)),
        ]);
        assert_composition_success(&result);
    }

    fn preserve_directive_from_one_subgraph_if_defined_differently_across_subgraphs_but_one_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
                @composeDirective(name: "@lowercase")

              directive @lowercase on FIELD_DEFINITION

              type User @key(fields: "id") {
                id: ID!
                name: String! @lowercase
              }

              type Query {
                users: [User]
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              directive @lowercase on OBJECT

              extend type User @key(fields: "id") @lowercase {
                id: ID!
                comments: [Comment]
              }

              type Comment @lowercase {
                id: ID!
                text: String!
              }

              type Query {
                comments: [Comment]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @lowercase on FIELD_DEFINITION
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @lowercase on FIELD_DEFINITION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true) {
          id: ID!
          name: String! @join__field(graph: A) @lowercase
          comments: [Comment] @join__field(graph: B)
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User {
          id: ID!
          name: String! @lowercase
          comments: [Comment]
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Comment @join__type(graph: B) {
          id: ID!
          text: String!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Comment {
          id: ID!
          text: String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION)
          @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"]) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn preserve_directive_if_defined_with_overlapping_locations_across_subgraphs_and_all_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
                @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
                @composeDirective(name: "@lowercase")

              directive @lowercase on FIELD_DEFINITION

              type User @key(fields: "id") {
                id: ID!
                name: String! @lowercase
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
                @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
                @composeDirective(name: "@lowercase")

              directive @lowercase on OBJECT | FIELD_DEFINITION

              extend type User @key(fields: "id") @lowercase {
                id: ID!
                comments: [Comment]
              }

              type Comment @lowercase {
                id: ID!
                text: String!
              }

              type Query {
                comments: [Comment]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @lowercase on OBJECT | FIELD_DEFINITION
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @lowercase on OBJECT | FIELD_DEFINITION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true)
          @lowercase {
          id: ID!
          name: String! @join__field(graph: A) @lowercase
          comments: [Comment] @join__field(graph: B)
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User @lowercase {
          id: ID!
          name: String! @lowercase
          comments: [Comment]
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Comment @join__type(graph: B) @lowercase {
          id: ID!
          text: String!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Comment @lowercase {
          id: ID!
          text: String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION)
          @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"]) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn preserve_directive_on_scalars_if_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(url: "https://myspecs.dev/whatever/v1.0", import: ["@whatever"])
                @composeDirective(name: "@whatever")

              directive @whatever on SCALAR

              scalar DateTime @whatever

              type User @key(fields: "id") {
                id: ID!
                name: String!
                createdAt: DateTime!
              }

              type Query {
                users: [User]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @whatever on SCALAR
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @whatever on SCALAR
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User @join__type(graph: A, key: "id") {
          id: ID!
          name: String!
          createdAt: DateTime!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User {
          id: ID!
          name: String!
          createdAt: DateTime!
        }
      "#);
    }

    fn preserve_directive_on_enums_if_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(url: "https://myspecs.dev/whatever/v1.0", import: ["@whatever"])
                @composeDirective(name: "@whatever")

              directive @whatever on ENUM

              enum UserType @whatever {
                ADMIN
                REGULAR
              }

              type User @key(fields: "id") {
                id: ID!
                name: String!
                type: UserType!
              }

              type Query {
                users: [User]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @whatever on ENUM
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @whatever on ENUM
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @whatever @join__type(graph: A) {
          ADMIN @join__enumValue(graph: A)
          REGULAR @join__enumValue(graph: A)
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        enum UserType
        @whatever
        {
          ADMIN
          REGULAR
        }
      "#);
    }

    fn preserve_directive_on_enum_values_if_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(url: "https://myspecs.dev/whatever/v1.0", import: ["@whatever"])
                @composeDirective(name: "@whatever")

              directive @whatever on ENUM_VALUE

              enum UserType {
                ADMIN @whatever
                REGULAR
              }

              type User @key(fields: "id") {
                id: ID!
                name: String!
                type: UserType!
              }

              type Query {
                users: [User]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @whatever on ENUM_VALUE
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @whatever on ENUM_VALUE
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @join__type(graph: A) {
          ADMIN @join__enumValue(graph: A) @whatever
          REGULAR @join__enumValue(graph: A)
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        enum UserType {
          ADMIN @whatever
          REGULAR
        }
      "#);
    }

    fn preserve_directive_on_union_types_if_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(url: "https://myspecs.dev/whatever/v1.0", import: ["@whatever"])
                @composeDirective(name: "@whatever")

              directive @whatever on UNION

              type User @key(fields: "id") {
                id: ID!
                name: String!
              }

              type Admin @key(fields: "id") {
                id: ID!
                name: String!
              }

              union Whoever @whatever = User | Admin

              type Query {
                whoever: [Whoever]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @whatever on UNION
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @whatever on UNION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        union Whoever
          @join__type(graph: A)
          @join__unionMember(graph: A, member: "Admin")
          @join__unionMember(graph: A, member: "User")
          @whatever =
          | Admin
          | User
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        union Whoever @whatever =
          | Admin
          | User
      "#);
    }

    fn preserve_directive_on_interface_its_field_and_argument_if_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@composeDirective"]
                )
                @link(url: "https://myspecs.dev/whatever/v1.0", import: ["@whatever"])
                @composeDirective(name: "@whatever")

              directive @whatever on FIELD_DEFINITION | INTERFACE | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

              type AuthenticatedUser implements User @key(fields: "id") {
                id: ID!
                name: String!
                createdAt: String!
                tags(limit: Int): [String!]!
              }

              type AnonymousUser implements User @key(fields: "id") {
                id: ID!
                tags(limit: Int): [String!]!
              }

              interface User @whatever {
                id: ID! @whatever
                tags(limit: Int @whatever): [String!]!
              }

              type Query {
                users: [User]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @whatever on FIELD_DEFINITION | INTERFACE | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @whatever on FIELD_DEFINITION | INTERFACE | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        interface User @join__type(graph: A) @whatever {
          id: ID! @whatever
          tags(limit: Int @whatever): [String!]!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        interface User @whatever {
          id: ID! @whatever
          tags(limit: Int @whatever): [String!]!
        }
      "#);
    }

    fn preserve_directive_on_directive_argument_if_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@composeDirective"]
                )
                @link(url: "https://myspecs.dev/whatever/v1.0", import: ["@whatever", "@whenever"])
                @composeDirective(name: "@whatever")
                @composeDirective(name: "@whenever")

              directive @whatever(when: String @whenever) on FIELD_DEFINITION
              directive @whenever on ARGUMENT_DEFINITION

              type User {
                id: ID! @whatever(when: "now")
              }

              type Query {
                users: [User]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @whatever(when: String @whenever) on FIELD_DEFINITION
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @whenever on ARGUMENT_DEFINITION
      "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @whatever(when: String @whenever) on FIELD_DEFINITION
        "#);

        to_contain_graphql(result.public_sdl(), r#"
          directive @whenever on ARGUMENT_DEFINITION
        "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User @join__type(graph: A) {
          id: ID! @whatever(when: "now")
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User {
          id: ID! @whatever(when: "now")
        }
      "#);
    }

    fn use_highest_version_of_a_spec_that_imports_a_custom_directive_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
                @link(url: "https://myspecs.dev/lowercase/v1.1", import: ["@lowercase"])
                @composeDirective(name: "@lowercase")

              directive @lowercase on FIELD_DEFINITION

              type User @key(fields: "id") {
                id: ID!
                name: String! @lowercase
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
                @link(url: "https://myspecs.dev/lowercase/v1.3", import: ["@lowercase"])
                @composeDirective(name: "@lowercase")

              directive @lowercase on OBJECT | FIELD_DEFINITION

              extend type User @key(fields: "id") @lowercase {
                id: ID!
                comments: [Comment]
              }

              type Comment @lowercase {
                id: ID!
                text: String!
              }

              type Query {
                comments: [Comment]
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @lowercase on OBJECT | FIELD_DEFINITION
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true)
          @lowercase {
          id: ID!
          name: String! @join__field(graph: A) @lowercase
          comments: [Comment] @join__field(graph: B)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Comment @join__type(graph: B) @lowercase {
          id: ID!
          text: String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION)
          @link(url: "https://myspecs.dev/lowercase/v1.3", import: ["@lowercase"]) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn two_different_major_versions_of_the_same_spec_were_used_to_import_a_scalar_of_the_same_name(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
                @link(url: "https://myspecs.dev/casing/v1.1", import: ["LowercaseString"])

              scalar LowercaseString

              type User @key(fields: "id") {
                id: ID!
                name: LowercaseString!
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
                @link(url: "https://myspecs.dev/casing/v2.3", import: ["LowercaseString"])

              scalar LowercaseString

              extend type User @key(fields: "id") {
                id: ID!
                comments: [Comment]
              }

              type Comment {
                id: ID!
                text: LowercaseString!
              }

              type Query {
                comments: [Comment]
              }
            "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        scalar LowercaseString @join__type(graph: A) @join__type(graph: B)
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        scalar LowercaseString
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true) {
          id: ID!
          name: LowercaseString! @join__field(graph: A)
          comments: [Comment] @join__field(graph: B)
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User {
          id: ID!
          name: LowercaseString!
          comments: [Comment]
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Comment @join__type(graph: B) {
          id: ID!
          text: LowercaseString!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Comment {
          id: ID!
          text: LowercaseString!
        }
      "#);
        // No @link directive for the scalar - not sure why though...
        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn allow_to_compose_a_built_in_directive_one_of(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
                @link(url: "https://spec.graphql.org/specified/v1.6", import: ["@oneOf"])
                @composeDirective(name: "@oneOf")

              directive @oneOf on INPUT_OBJECT

              input HelloInput @oneOf {
                world: String
                me: String
              }

              type Query {
                hello(input: HelloInput): String
              }
            "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @oneOf on INPUT_OBJECT
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        input HelloInput @oneOf @join__type(graph: A) {
          world: String
          me: String
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        input HelloInput
        @oneOf
        {
          world: String
          me: String
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://spec.graphql.org/specified/v1.6", import: ["@oneOf"])
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn two_different_specs_were_used_to_import_a_scalar_of_the_same_name(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
                @link(url: "https://myspecs.dev/casing/v1.1", import: ["LowercaseString"])

              scalar LowercaseString

              type User @key(fields: "id") {
                id: ID!
                name: LowercaseString!
              }
            "#, version)),
            service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
                @link(url: "https://yourspecs.dev/casing/v2.3", import: ["LowercaseString"])

              scalar LowercaseString

              extend type User @key(fields: "id") {
                id: ID!
                comments: [Comment]
              }

              type Comment {
                id: ID!
                text: LowercaseString!
              }

              type Query {
                comments: [Comment]
              }
            "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        scalar LowercaseString @join__type(graph: A) @join__type(graph: B)
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true) {
          id: ID!
          name: LowercaseString! @join__field(graph: A)
          comments: [Comment] @join__field(graph: B)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Comment @join__type(graph: B) {
          id: ID!
          text: LowercaseString!
        }
      "#);
        // No @link directive for the scalar - not sure why though...
        to_contain_graphql(result.supergraph_sdl(), tpl!(r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/${join}", for: EXECUTION) {
          query: Query
        }
      "#, join = federation_version_to_join_spec_version(version)));
    }

    fn import_type(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])
              @link(url: "https://myspecs.dev/json/v1.0", import: ["JSON"])

            scalar JSON

            type User @shareable {
              name: JSON
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])
              @link(url: "https://myspecs.dev/json/v1.0", import: [{ name: "JSON2", as: "JSON" }])

            scalar JSON

            type User @shareable {
              name: JSON
            }

            type Query {
              users: [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);
        // KNOW: if directives are defined and registered at least once by @composeDirective
        to_contain_graphql(result.supergraph_sdl(), r#"
        scalar JSON @join__type(graph: A) @join__type(graph: B)
      "#);
    }

    fn every_entity_must_be_annotated_with_the_join_type_directive(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "email") {
              email: ID!
              name: String
              totalProductsCreated: Int
              role: Role!
            }

            enum Role {
              ADMIN
              USER
            }
          "#, version)),
            service("pandas", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              allPandas: [Panda]
              panda(name: ID!): Panda
            }

            type Panda {
              name: ID!
              favoriteFood: String
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"type Panda @join__type(graph: PANDAS)"#);
        to_contain_graphql(result.supergraph_sdl(), r#"type User @join__type(graph: USERS, key: "email")"#);
        to_contain_graphql(result.supergraph_sdl(), r#"
        enum Role @join__type(graph: USERS) {
          ADMIN @join__enumValue(graph: USERS)
          USER @join__enumValue(graph: USERS)
        }
      "#);
    }

    fn renaming_directives(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: [{ name: "@key", as: "@primaryKey" }]
              )

            type User @primaryKey(fields: "email") {
              email: ID!
              name: String
              totalProductsCreated: Int
              role: Role!
            }

            enum Role {
              ADMIN
              USER
            }
          "#, version)),
            service("pandas", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              allPandas: [Panda]
              panda(name: ID!): Panda
            }

            type Panda @key(fields: "name") {
              name: ID!
              favoriteFood: String
            }
          "#, version)),
        ]);
        // KNOW: if directive has an alias
        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"type Panda @join__type(graph: PANDAS, key: "name")"#);
        to_contain_graphql(result.supergraph_sdl(), r#"type User @join__type(graph: USERS, key: "email")"#);
        to_contain_graphql(result.supergraph_sdl(), r#"
        enum Role @join__type(graph: USERS) {
          ADMIN @join__enumValue(graph: USERS)
          USER @join__enumValue(graph: USERS)
        }
      "#);
    }

    fn namespaced_directives(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "email") {
              email: ID!
              name: String
              totalProductsCreated: Int
              role: Role!
            }

            type Details @federation__shareable {
              createdAt: String
            }

            enum Role {
              ADMIN
              USER
            }
          "#, version)),
            service("pandas", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: [])

            type Query {
              allPandas: [Panda]
              panda(name: ID!): Panda
            }

            type Panda @federation__key(fields: "name") {
              name: ID!
              favoriteFood: String
            }

            type Details @federation__shareable {
              createdAt: String
            }

            enum Role {
              ADMIN
              USER
            }
          "#, version)),
        ]);
        // KNOW: extract `federation` from `https://specs.apollo.dev/federation/v2.0`
        // KNOW: if the namespace is `federation`, make all its directives available with the `federation__` prefix
        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User @join__type(graph: USERS, key: "email") {
          email: ID!
          name: String
          totalProductsCreated: Int
          role: Role!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Panda @join__type(graph: PANDAS, key: "name") {
          name: ID!
          favoriteFood: String
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Details @join__type(graph: PANDAS) @join__type(graph: USERS) {
          createdAt: String
        }
      "#);
    }

    fn repeatable_key(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "email") @key(fields: "name") {
              email: ID!
              name: String
              totalProductsCreated: Int
            }

            type Query {
              allUsers: [User]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: USERS, key: "email")
          @join__type(graph: USERS, key: "name") {
          email: ID!
          name: String
          totalProductsCreated: Int
        }
      "#);
    }

    fn interface_object(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@interfaceObject"]
              )

            interface Media @key(fields: "id") {
              id: ID!
              title: String!
            }

            type Book implements Media @key(fields: "id") {
              id: ID!
              title: String!
            }

            type Query {
              books: [Book]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@interfaceObject"]
              )

            type Media @key(fields: "id") @interfaceObject {
              id: ID!
              reviews: [Review!]!
            }

            type Review {
              score: Int!
            }

            type Query {
              topRatedMedia: [Media!]!
            }
          "#, version)),
        ]);

        if satisfies_version_range("< v2.3", version) {
            assert_composition_failure(&result);
            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Cannot import unknown element "@interfaceObject"."#)
                .code("INVALID_LINK_DIRECTIVE_USAGE")]);
            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[b] Cannot import unknown element "@interfaceObject"."#)
                .code("INVALID_LINK_DIRECTIVE_USAGE")]);

            return;
        }

        assert_composition_success(&result);
        // KNOW: which interface should be extended by @interfaceObject
        // KNOW: which fields were added by @interfaceObject and by which graph
        to_contain_graphql(result.supergraph_sdl(), r#"
        interface Media
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", isInterfaceObject: true) {
          id: ID!
          title: String! @join__field(graph: A)
          reviews: [Review!]! @join__field(graph: B)
        }
      "#);
        to_contain_graphql(result.supergraph_sdl(), r#"
        type Book implements Media
          @join__implements(graph: A, interface: "Media")
          @join__type(graph: A, key: "id") {
          id: ID!
          title: String!
          reviews: [Review!]! @join__field
        }
      "#);
    }

    fn extends(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Book @key(fields: "id") {
              id: ID!
              title: String!
            }

            type Query {
              books: [Book]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@extends"])

            type Book @key(fields: "id") @extends {
              id: ID!
              review: Review!
            }

            type Review {
              score: Int!
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Book
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", extension: true) {
          id: ID!
          title: String! @join__field(graph: A)
          review: Review! @join__field(graph: B)
        }
      "#);
    }

    fn r#override(api, version) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              inStock: Boolean!
            }

            type Query {
              products: [Product]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override"])

            type Product @key(fields: "id") {
              id: ID!
              inStock: Boolean! @override(from: "a")
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID!
          inStock: Boolean! @join__field(graph: B, override: "a")
        }
      "#);

        result = api.compose_services(&[
            service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@shareable", "@override"]
              )

            type User @key(fields: "id") @shareable {
              id: ID!
              name: String
            }

            type Query {
              user: User
            }
          "#, version)),
            service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@shareable", "@override"]
              )

            type User @key(fields: "id") @shareable {
              id: ID!
              name: String @override(from: "main")
            }
          "#, version)),
            service("main", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@shareable", "@override"]
              )

            type User @shareable {
              name: String
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: FOO, key: "id")
          @join__type(graph: MAIN)
          @join__type(graph: BAR, key: "id") {
          id: ID! @join__field(graph: BAR) @join__field(graph: FOO)
          name: String
            @join__field(graph: FOO)
            @join__field(graph: BAR, override: "main")
        }
      "#);
    }

    fn remove_provides_fields_when_all_fields_are_in_common_with_key_fields_and_provides_refers_to_a_type_extension(api, version) {
        let result = api.compose_services(&[
            service("foo", graphql!(r#"
            extend type User @key(fields: "id") {
              id: String! @external
            }

            type Group @key(fields: "id") {
              id: ID
              title: String
              users: [User] @provides(fields: "id")
            }
          "#)),
            service("bar", graphql!(r#"
            extend type Group @key(fields: "id") {
              id: ID @external
              topic: Topic
            }

            type Topic @key(fields: "id") {
              id: ID!
              title: String!
            }

            type User @key(fields: "id") {
              id: String!
              name: String
            }
          "#)),
            service("baz", graphql!(r#"
            type Theme @key(fields: "id") {
              id: ID!
              primaryColor: String
            }

            type Query {
              themes: [Theme]
            }
          "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Group
          @join__type(graph: FOO, key: "id")
          @join__type(graph: BAR, key: "id") {
          id: ID
          title: String @join__field(graph: FOO)
          users: [User] @join__field(graph: FOO)
          topic: Topic @join__field(graph: BAR)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Topic @join__type(graph: BAR, key: "id") {
          id: ID!
          title: String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: FOO, key: "id")
          @join__type(graph: BAR, key: "id") {
          id: String!
          name: String @join__field(graph: BAR)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Theme @join__type(graph: BAZ, key: "id") {
          id: ID!
          primaryColor: String
        }
      "#);
    }

    fn preserve_provides_fields_when_all_fields_are_in_common_with_key_fields_and_provides_refers_to_type_definition(api, version) {
        let result = api.compose_services(&[
            service("foo", graphql!(r#"
            type User @key(fields: "id") {
              id: String! @external
            }

            type Group @key(fields: "id") {
              id: ID
              title: String
              users: [User] @provides(fields: "id")
            }
          "#)),
            service("bar", graphql!(r#"
            extend type Group @key(fields: "id") {
              id: ID @external
              topic: Topic
            }

            type Topic @key(fields: "id") {
              id: ID!
              title: String!
            }

            type User @key(fields: "id") {
              id: String!
              name: String
            }
          "#)),
            service("baz", graphql!(r#"
            type Theme @key(fields: "id") {
              id: ID!
              primaryColor: String
            }

            type Query {
              themes: [Theme]
            }
          "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Group
          @join__type(graph: FOO, key: "id")
          @join__type(graph: BAR, key: "id") {
          id: ID
          title: String @join__field(graph: FOO)
          users: [User] @join__field(graph: FOO, provides: "id")
          topic: Topic @join__field(graph: BAR)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Topic @join__type(graph: BAR, key: "id") {
          id: ID!
          title: String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: BAR, key: "id")
          @join__type(graph: FOO, key: "id") {
          id: String!
            @join__field(graph: BAR)
            @join__field(graph: FOO, external: true)
          name: String @join__field(graph: BAR)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Theme @join__type(graph: BAZ, key: "id") {
          id: ID!
          primaryColor: String
        }
      "#);
    }

    fn provides_itself_circular_reference(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@external", "@provides"]
              )

            type Query {
              product: Product
            }

            interface Product {
              samePriceProduct: Product
            }

            type Book implements Product @key(fields: "id") {
              id: ID!
              samePriceProduct: Book @provides(fields: "price")
              price: Float @external
            }
          "#)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@shareable"]
              )

            type Book @key(fields: "id") {
              id: ID!
              price: Float @shareable
            }
          "#)),
        ]);

        assert_composition_success(&result);
    }

    fn external_provides(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Product @key(fields: "id") {
              id: ID!
              name: String! @shareable
              inStock: Int!
            }

            type Query {
              products: [Product]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external", "@provides"]
              )

            type Product @key(fields: "id") {
              id: ID!
              name: String! @external
            }

            type Query {
              outOfStockProducts: [Product!]! @provides(fields: "name")
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID!
          name: String!
            @join__field(graph: A)
            @join__field(graph: B, external: true)
          inStock: Int! @join__field(graph: A)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: A) @join__type(graph: B) {
          products: [Product] @join__field(graph: A)
          outOfStockProducts: [Product!]!
            @join__field(graph: B, provides: "name")
        }
      "#);
    }

    fn external_requires(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              name: String!
              inStock: Int!
            }

            type Query {
              products: [Product]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external", "@requires"]
              )

            type Product @key(fields: "id") {
              id: ID!
              inStock: Int! @external
              isAvailable: String! @requires(fields: "inStock")
            }
          "#, version)),
        ]);

        assert_composition_success(&result);
        // KNOW: which fields are external
        // KNOW: which fields are required
        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID!
          name: String! @join__field(graph: A)
          inStock: Int!
            @join__field(graph: A)
            @join__field(graph: B, external: true)
          isAvailable: String! @join__field(graph: B, requires: "inStock")
        }
      "#);
    }

    fn external_and_not_a_key_in_its_graph_but_is_key_in_other_graph(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Product @key(fields: "id") @shareable{
              id: ID!
              productId: String!
            }

            type Query {
              products: [Product]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id", resolvable: false) {
              id: ID!
            }
          "#, version)),
            service("d", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable", "@external", "@key", "@requires"])

            extend type Product @shareable @key(fields: "id productId") {
              id: ID! @external
              productId: String! @external
              bField: Boolean
            }
          "#, version)),
            service("e", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@shareable", "@external", "@requires"]
              )

            extend type Product @shareable @key(fields: "id") {
              id: ID! @external
              productId: String! @external
              bField: Boolean @requires(fields: "productId")
            }
          "#)),
        ]);

        assert_composition_success_with_message(
            &result,
            Some(&format!(
                "Composition failures:\n{}",
                result
                    .errors()
                    .map(|errors| errors.iter().map(|e| format!("- {}", e.message)).collect::<Vec<_>>().join("\n"))
                    .unwrap_or_else(|| "undefined".to_string()),
            )),
        );

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", resolvable: false)
          @join__type(graph: D, key: "id productId", extension: true)
          @join__type(graph: E, key: "id", extension: true) {
          id: ID!
          productId: String!
            @join__field(external: true, graph: E)
            @join__field(graph: A)
            @join__field(graph: D)
          bField: Boolean
            @join__field(graph: D)
            @join__field(graph: E, requires: "productId")
        }
      "#);
    }

    fn external_tag(api, version) {
        let subgraphs = vec![
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@tag"])

            type Product @key(fields: "id") {
              id: ID! @tag(name: "public")
              name: String! @tag(name: "public")
              inStock: Int! @tag(name: "public")
            }

            type Query {
              products: [Product] @tag(name: "public")
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external", "@requires", "@tag"]
              )

            type Product @key(fields: "id") {
              id: ID! @tag(name: "public")
              inStock: Int! @external
              isAvailable: String! @requires(fields: "inStock") @tag(name: "public")
            }
          "#, version)),
        ];

        let result = api.compose_services(&subgraphs);
        assert!(result.errors().is_none());
        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID! @tag(name: "public")
          name: String! @join__field(graph: A) @tag(name: "public")
          inStock: Int!
            @join__field(graph: A)
            @join__field(graph: B, external: true)
            @tag(name: "public")
          isAvailable: String!
            @join__field(graph: B, requires: "inStock")
            @tag(name: "public")
        }
      "#);
    }

    fn tag(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@tag"])

            type Query {
              customer(id: String!): Customer @tag(name: "team-customers")
              employee(id: String! @tag(name: "team-admin")): Employee @tag(name: "team-admin")
              employees(filter: EmployeesFilter!): [Employee!] @tag(name: "team-admin")
            }

            scalar DateTime @tag(name: "team-admin")

            interface User @tag(name: "team-accounts") {
              id: String!
              name(lowercase: Boolean @tag(name: "team-accounts")): String!
            }

            type Customer implements User @tag(name: "team-customers") {
              id: String!
              name(lowercase: Boolean @tag(name: "team-accounts")): String!
            }

            type Employee implements User @tag(name: "team-admin") {
              id: String!
              name(lowercase: Boolean @tag(name: "team-accounts")): String!
              ssn: String!
              createdAt: DateTime!
            }

            input EmployeesFilter {
              id: String @tag (name: "team-admin")
              name: String
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: A) {
          customer(id: String!): Customer @tag(name: "team-customers")
          employee(id: String! @tag(name: "team-admin")): Employee
            @tag(name: "team-admin")
          employees(filter: EmployeesFilter!): [Employee!]
            @tag(name: "team-admin")
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        interface User @join__type(graph: A) @tag(name: "team-accounts") {
          id: String!
          name(lowercase: Boolean @tag(name: "team-accounts")): String!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        interface User {
          id: String!
          name(lowercase: Boolean): String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Employee implements User
          @join__implements(graph: A, interface: "User")
          @join__type(graph: A)
          @tag(name: "team-admin") {
          id: String!
          name(lowercase: Boolean @tag(name: "team-accounts")): String!
          ssn: String!
          createdAt: DateTime!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Employee implements User {
          id: String!
          name(lowercase: Boolean): String!
          ssn: String!
          createdAt: DateTime!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Customer implements User
          @join__implements(graph: A, interface: "User")
          @join__type(graph: A)
          @tag(name: "team-customers") {
          id: String!
          name(lowercase: Boolean @tag(name: "team-accounts")): String!
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Customer implements User {
          id: String!
          name(lowercase: Boolean): String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        scalar DateTime @join__type(graph: A) @tag(name: "team-admin")
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        scalar DateTime
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        input EmployeesFilter @join__type(graph: A) {
          id: String @tag(name: "team-admin")
          name: String
        }
      "#);

        to_contain_graphql(result.public_sdl(), r#"
        input EmployeesFilter {
          id: String
          name: String
        }
      "#);
    }

    // Skipping because this test is not valid for earlier versions
    fn tag_on_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@tag", "@composeDirective"])
            @link(url: "https://myspecs.dev/access/v1.0", import: ["@access"])
                @composeDirective(name: "@access")

            directive @access(scope: String @tag(name: "team-admin")) on FIELD_DEFINITION

            type Query {
              employee(id: String!): Employee @tag(name: "team-admin") @access(scope: "admin")
            }

            type Employee {
              id: String!
              name: String!
              ssn: String!
            }
          "#, version)),
        ]);

        if satisfies_version_range("< v2.3", version) {
            if version == "v2.0" {
                assert_composition_failure(&result);
            } else {
                assert_composition_success(&result);
            }
            return;
        }

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: A) {
          employee(id: String!): Employee
            @tag(name: "team-admin")
            @access(scope: "admin")
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Employee @join__type(graph: A) {
          id: String!
          name: String!
          ssn: String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        directive @access(
          scope: String @tag(name: "team-admin")
        ) on FIELD_DEFINITION
      "#);
    }

    fn entity_annotated_with_key_directive_must_have_join_type_with_key_argument(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "email") {
              email: ID!
              name: String
              totalProductsCreated: Int
              role: Role!
            }

            enum Role {
              ADMIN
              USER
            }
          "#, version)),
            service("pandas", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              allPandas: [Panda]
              panda(name: ID!): Panda
            }

            type Panda {
              name: ID!
              favoriteFood: String
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"type Panda @join__type(graph: PANDAS)"#);
        to_contain_graphql(result.supergraph_sdl(), r#"type User @join__type(graph: USERS, key: "email")"#);
        to_contain_graphql(result.supergraph_sdl(), r#"
        enum Role @join__type(graph: USERS) {
          ADMIN @join__enumValue(graph: USERS)
          USER @join__enumValue(graph: USERS)
        }
      "#);
    }

    fn fields_referenced_by_key_directive_must_not_be_annotated_with_join_field(api, version) {
        let result = api.compose_services(&[
            service("reviews", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

            extend type Product @key(fields: "id") {
              id: ID! @external
              reviews: [Review]
            }

            type Review @key(fields: "id") {
              id: ID!
              rating: Float
              content: String
            }
          "#, version)),
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              title: String
            }

            extend type Query {
              product(id: ID!): Product
            }
          "#, version)),
        ]);
        // KNOW: if field is the primary key, to not annotate it with @join__field
        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: PRODUCTS, key: "id")
          @join__type(graph: REVIEWS, key: "id", extension: true) {
          id: ID!
          title: String @join__field(graph: PRODUCTS)
          reviews: [Review] @join__field(graph: REVIEWS)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Review @join__type(graph: REVIEWS, key: "id") {
          id: ID!
          rating: Float
          content: String
        }
      "#);
    }

    fn entity_used_by_more_than_one_service_must_be_annotated_with_join_type_of_these_service(api, version) {
        let result = api.compose_services(&[
            service("reviews", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

            extend type Product @key(fields: "id") {
              id: ID! @external
              reviews: [Review]
            }

            type Review @key(fields: "id") {
              id: ID!
              rating: Float
              content: String
            }
          "#, version)),
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              title: String
            }

            extend type Query {
              product(id: ID!): Product
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"type Product @join__type(graph: PRODUCTS, key: "id") @join__type(graph: REVIEWS, key: "id", extension: true)"#);
    }

    fn entity_directly_used_by_more_than_one_service_must_have_all_fields_annotated_with_join_field_directive_where_graph_argument_points_to_the_owning_service(api, version) {
        let result = api.compose_services(&[
            service("reviews", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

            extend type Product @key(fields: "id") {
              id: ID! @external
              reviews: [Review]
            }

            type Review @key(fields: "id") {
              id: ID!
              rating: Float
              content: String
            }
          "#, version)),
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              title: String
              price: Price!
            }

            type Price {
              amount: Float
            }

            extend type Query {
              product(id: ID!): Product
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: PRODUCTS, key: "id")
          @join__type(graph: REVIEWS, key: "id", extension: true) {
          id: ID!
          title: String @join__field(graph: PRODUCTS)
          price: Price! @join__field(graph: PRODUCTS)
          reviews: [Review] @join__field(graph: REVIEWS)
        }
      "#);
    }

    fn a_field_of_an_entity_with_different_return_type_nullability_must_be_annotated_with_multiple_join_field_directives_each_with_original_return_type(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])
            type Position @shareable {
              x: Int!
              y: Int!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])
            type Position @shareable {
              x: Int
              y: Int!
            }

            type Query {
              position: Position
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Position @join__type(graph: A) @join__type(graph: B) {
          x: Int
            @join__field(graph: A, type: "Int!")
            @join__field(graph: B, type: "Int")
          y: Int!
        }
      "#);
    }

    fn a_field_of_an_entity_with_same_type_but_external_true_in_one_or_more_must_be_annotated_with_multiple_join_field_directives(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id name") {
              id: ID!
              name: String!
              discount: Int!
            }

            type Query {
              product(id: ID!): Product
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

            type Product @key(fields: "id name") {
              id: ID!
              name: String! @external
              reviews: [String]
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: A, key: "id name")
          @join__type(graph: B, key: "id name") {
          id: ID!
          name: String!
            @join__field(graph: A)
            @join__field(graph: B, external: true)
          discount: Int! @join__field(graph: A)
          reviews: [String] @join__field(graph: B)
        }
      "#);
    }

    fn enum_indirectly_used_by_more_than_one_service_must_have_all_values_annotated_with_join_field_directive_where_graph_argument_points_to_the_owning_service(api, version) {
        let result = api.compose_services(&[
            service("reviews", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

            extend type Product @key(fields: "id") {
              id: ID! @external
              reviews: [Review]
            }

            type Review @key(fields: "id") {
              id: ID!
              rating: Float
              content: String
            }
          "#, version)),
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              title: String
              price: Price!
              category: ProductCategory!
            }

            type Price {
              amount: Float
            }

            enum ProductCategory {
              ALL
              ELECTRONICS
            }

            extend type Query {
              product(id: ID!): Product
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: PRODUCTS, key: "id")
          @join__type(graph: REVIEWS, key: "id", extension: true) {
          id: ID!
          title: String @join__field(graph: PRODUCTS)
          price: Price! @join__field(graph: PRODUCTS)
          category: ProductCategory! @join__field(graph: PRODUCTS)
          reviews: [Review] @join__field(graph: REVIEWS)
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        enum ProductCategory @join__type(graph: PRODUCTS) {
          ALL @join__enumValue(graph: PRODUCTS)
          ELECTRONICS @join__enumValue(graph: PRODUCTS)
        }
      "#);
    }

    fn query_must_be_annotated_with_join_type_directive_of_every_service(api, version) {
        let result = api.compose_services(&[
            service("reviews", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Review @key(fields: "id") {
              id: ID!
              rating: Float
              content: String
            }
          "#, version)),
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              title: String
              price: Price!
            }

            type Price {
              amount: Float
            }

            extend type Query {
              product(id: ID!): Product
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"type Query @join__type(graph: PRODUCTS) @join__type(graph: REVIEWS)"#);
    }

    fn all_fields_of_query_object_must_be_annotated_with_join_field_pointing_to_the_owning_service(api, version) {
        let result = api.compose_services(&[
            service("reviews", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@shareable"])

            extend type Query {
              review(id: ID!): Review
              product(id: ID!): Product @shareable
            }

            extend type Product @key(fields: "id") {
              id: ID! @external
              reviews: [Review]
            }

            type Review @key(fields: "id") {
              id: ID!
              rating: Float
              content: String
            }
          "#, version)),
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Product @key(fields: "id") {
              id: ID!
              title: String
              price: Price!
            }

            type Price {
              amount: Float
            }

            extend type Query {
              product(id: ID!): Product @shareable
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: PRODUCTS) @join__type(graph: REVIEWS) {
          product(id: ID!): Product
          review(id: ID!): Review @join__field(graph: REVIEWS)
        }
      "#);
    }

    fn v1_all_fields_of_query_object_must_be_annotated_with_join_field_pointing_to_the_owning_service(api, version) {
        let result = api.compose_services(&[
            service("reviews", graphql!(r#"
            extend type Query {
              review(id: ID!): Review
              product(id: ID!): Product
            }

            extend type Product @key(fields: "id") {
              id: ID! @external
              reviews: [Review]
            }

            type Review @key(fields: "id") {
              id: ID!
              rating: Float
              content: String
            }
          "#)),
            service("products", graphql!(r#"
            type Product @key(fields: "id") {
              id: ID!
              title: String
              price: Price!
            }

            type Price {
              amount: Float
            }

            extend type Query {
              product(id: ID!): Product
            }
          "#)),
        ]);

        assert_composition_success(&result);
        // Query.product is defined in all subgraphs, so no need to annotate it (that's not true in Fed v2 though)
        to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: PRODUCTS) @join__type(graph: REVIEWS) {
          product(id: ID!): Product
          review(id: ID!): Review @join__field(graph: REVIEWS)
        }
      "#);
    }

    fn key_resolvable_false_should_be_passed_to_join_type_resolvable_false(api, version) {
        let result = api.compose_services(&[
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "id", resolvable: false) {
              id: ID!
              title: String
            }

            type Query {
              product(id: ID!): Product
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product
          @join__type(graph: PRODUCTS, key: "id", resolvable: false) {
          id: ID!
          title: String
        }
      "#);
    }

    fn join_implements_on_interface_type(api, version) {
        let result = api.compose_services(&[
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product implements Node & SellingItem @key(fields: "id")  {
              id: ID!
              name: String!
              price: Price!
            }

            interface Node {
              id: ID!
            }

            interface SellingItem implements Node {
              id: ID!
              price: Price!
            }

            type Price {
              amount: Float
            }

            type Query {
              product(id: ID!): Product
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Product implements Node & SellingItem
          @join__implements(graph: PRODUCTS, interface: "Node")
          @join__implements(graph: PRODUCTS, interface: "SellingItem")
          @join__type(graph: PRODUCTS, key: "id") {
          id: ID!
          name: String!
          price: Price!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        interface SellingItem implements Node
          @join__implements(graph: PRODUCTS, interface: "Node")
          @join__type(graph: PRODUCTS) {
          id: ID!
          price: Price!
        }
      "#);
    }

    fn fed_v1_no_join_type_key_on_interface_type(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product {
              id: ID!
              name: String!
            }
          "#, version)),
            service("b", graphql!(r#"
            type User implements Node @key(fields: "id") {
              id: ID!
              age: Int!
            }

            interface Node @key(fields: "id") {
              id: ID!
            }

            type Query {
              user: User
            }
          "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        interface Node @join__type(graph: B) {
          id: ID!
        }
      "#);
    }
}

mod descriptions {
    use super::*;

    test_versions! {
        fn on_scalars(api, version) {
            let result = api.compose_services(&[
                service("reviews", graphql!(r#"
              extend type Query {
                dateTime: DateTime
              }

              """
              reviews
              """
              scalar DateTime
            "#)),
                service("products", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              """
              products
              """
              scalar DateTime

              type Product @key(fields: "id") {
                id: ID!
                title: String
                createdAt: DateTime
              }

              extend type Query {
                product(
                  id: ID!
                ): Product
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          products
          """
          scalar DateTime
        "#);
        }

        fn on_object_types(api, version) {
            let mut result = api.compose_services(&[
                service("a", graphql!(r#"
              """
              a
              """
              type Query {
                a: String
              }
            "#)),
                service("b", graphql!(r#"
              """
              b
              """
              type Query {
                b: String
              }
            "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          a
          """
          type Query @join__type(graph: A) @join__type(graph: B) {
            a: String @join__field(graph: A)
            b: String @join__field(graph: B)
          }
        "#);

            result = api.compose_services(&[
                service("reviews", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

              """reviews"""
              type Query {
                """
                reviews
                """
                review(
                  """
                  reviews
                  """
                  id: ID!
                ): Review
              }

              extend type Product @key(fields: "id") {
                """
                it will be removed
                """
                id: ID! @external
                """
                reviews
                """
                reviews: [Review]
              }

              """
              reviews
              """
              type Review @key(fields: "id") {
                """
                reviews
                """
                id: ID!
                """
                reviews
                """
                rating: Float
                """
                reviews
                """
                content: String
              }
            "#, version)),
                service("pricing", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@extends"])

              type Product @key(fields: "id") @extends {
                """
                pricing
                """
                id: ID! @external
                promoCodes: [PromoCode]
              }

              type PromoCode {
                id: ID!
                code: String!
              }
            "#, version)),
                service("products", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              """
              products
              """
              type Product @key(fields: "id") {
                """
                products
                """
                id: ID!
                """
                products
                """
                title: String
                """
                products
                """
                price(currency: String = "USD"): Price!
              }

              """
              products
              """
              type Price {
                """
                products
                """
                amount: Float
              }

              type Query {
                """
                products
                """
                product(
                  """
                  products
                  """
                  id: ID!
                ): Product
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          products
          """
          type Price @join__type(graph: PRODUCTS) {
            """
            products
            """
            amount: Float
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          products
          """
          type Product
            @join__type(graph: PRICING, key: "id", extension: true)
            @join__type(graph: PRODUCTS, key: "id")
            @join__type(graph: REVIEWS, key: "id", extension: true) {
            """
            pricing
            """
            id: ID!
            promoCodes: [PromoCode] @join__field(graph: PRICING)
            """
            products
            """
            title: String @join__field(graph: PRODUCTS)
            """
            products
            """
            price(currency: String = "USD"): Price!
              @join__field(graph: PRODUCTS)
            """
            reviews
            """
            reviews: [Review] @join__field(graph: REVIEWS)
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          reviews
          """
          type Review @join__type(graph: REVIEWS, key: "id") {
            """
            reviews
            """
            id: ID!

            """
            reviews
            """
            rating: Float

            """
            reviews
            """
            content: String
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          reviews
          """
          type Query
            @join__type(graph: PRODUCTS)
            @join__type(graph: REVIEWS)
            @join__type(graph: PRICING) {
            """
            products
            """
            product(
              """
              products
              """
              id: ID!
            ): Product @join__field(graph: PRODUCTS)
            """
            reviews
            """
            review(
              """
              reviews
              """
              id: ID!
            ): Review @join__field(graph: REVIEWS)
          }
        "#);
        }

        fn on_fields_with_override_entity_types(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@requires", "@external"]
                )

              type User @key(fields: "id") {
                id: ID!
                name: String! @external
                aName: String! @requires(fields: "name")
              }

              type Query {
                userA: User
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override"])

              type User @key(fields: "id") {
                id: ID!
                name: String! @override(from: "c")
              }

              type Query {
                userB: User
              }
            "#, version)),
                service("c", graphql!(r#"
              type User @key(fields: "id") {
                id: ID!
                "it should not be in supergraph"
                name: String! @external
              }

              type Foo @key(fields: "id") {
                id: ID!
                foo: String
              }

              scalar _Any
              union _Entity = Foo | User

              type _Service {
                sdl: String!
              }

              type Query {
                _entities(representations: [_Any!]!): [_Entity]!
                _service: _Service
                c: String!
              }
            "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type User
            @join__type(graph: A, key: "id")
            @join__type(graph: B, key: "id")
            @join__type(graph: C, key: "id") {
            id: ID!
            name: String!
              @join__field(external: true, graph: A)
              @join__field(graph: B, override: "c")
            aName: String! @join__field(graph: A, requires: "name")
          }
        "#);
        }

        fn on_fields_with_override_not_entity_types(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@shareable", "@override", "@key"]
                )

              type User @shareable {
                id: ID!
                name: String! @override(from: "b")
              }

              type Organization @key(fields: "id") {
                id: ID!
                owner: User @shareable
              }

              type Query {
                organizationA: Organization
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable", "@key"])

              type User @shareable {
                id: ID! @shareable
                """should be preserved"""
                name: String! @shareable
              }

              type Organization @key(fields: "id") {
                id: ID!
                owner: User @shareable
              }

              type Query {
                organizationB: Organization
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type User @join__type(graph: A) @join__type(graph: B) {
            id: ID!
            """
            should be preserved
            """
            name: String! @join__field(graph: A, override: "b")
          }
        "#);
        }

        fn on_object_type_definition_with_extends(api, version) {
            let result = api.compose_services(&[
                service("reviews", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@extends"])

              type Query {
                review(
                  id: ID!
                ): Review
              }

              """
              reviews
              """
              type Product @key(fields: "id") @extends {
                """
                reviews
                """
                id: ID! @external
                """
                reviews
                """
                reviews: [Review]
              }

              type Review {
                id: ID!
                rating: Float
                content: String
              }
            "#, version)),
                service("products", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type Product @key(fields: "id") {
                """
                products
                """
                id: ID!
                """
                products
                """
                title: String
              }

              type Query {
                product(
                  id: ID!
                ): Product
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          reviews
          """
          type Product
            @join__type(graph: PRODUCTS, key: "id")
            @join__type(graph: REVIEWS, key: "id", extension: true) {
            """
            products
            """
            id: ID!
            """
            products
            """
            title: String @join__field(graph: PRODUCTS)
            """
            reviews
            """
            reviews: [Review] @join__field(graph: REVIEWS)
          }
        "#);
        }

        fn on_object_types_with_arguments(api, version) {
            let result = api.compose_services(&[
                service("reviews", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@shareable"])

              """reviews"""
              type Query {
                """
                reviews
                """
                review(
                  """
                  reviews
                  """
                  id: ID!
                ): Review
              }

              extend type Product @key(fields: "id") {
                """
                it will be removed
                """
                id: ID! @external
                """
                reviews
                """
                reviews(
                  """
                  reviews
                  """
                  limit: Int
                ): [Review]
                """
                reviews
                """
                price(
                  """
                  reviews
                  """
                  currency: String = "USD"
                ): Price! @shareable
              }

              """
              reviews
              """
              type Review @key(fields: "id") {
                """
                reviews
                """
                id: ID!
                """
                reviews
                """
                rating: Float
                """
                reviews
                """
                content: String
              }

              """
              reviews
              """
              type Price {
                """
                reviews
                """
                amount: Float @shareable
              }
            "#, version)),
                service("pricing", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@extends", "@shareable"])

              type Product @key(fields: "id") @extends {
                """
                pricing
                """
                id: ID! @external
                promoCodes: [PromoCode]
                """
                pricing
                """
                price(
                  """
                  pricing
                  """
                  currency: String = "USD"
                ): Price! @shareable
              }

              type PromoCode {
                id: ID!
                code: String!
              }

              """
              pricing
              """
              type Price {
                """
                pricing
                """
                amount: Float @shareable
              }
            "#, version)),
                service("products", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

              """
              products
              """
              type Product @key(fields: "id") {
                """
                products
                """
                id: ID!
                """
                products
                """
                title: String
                """
                products
                """
                price(
                  """
                  products
                  """
                  currency: String = "USD"
                ): Price! @shareable
              }

              """
              products
              """
              type Price {
                """
                products
                """
                amount: Float @shareable
              }

              type Query {
                """
                products
                """
                product(
                  """
                  products
                  """
                  id: ID!
                ): Product
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          pricing
          """
          type Price
            @join__type(graph: PRICING)
            @join__type(graph: PRODUCTS)
            @join__type(graph: REVIEWS) {
            """
            pricing
            """
            amount: Float
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          products
          """
          type Product
            @join__type(extension: true, graph: PRICING, key: "id")
            @join__type(extension: true, graph: REVIEWS, key: "id")
            @join__type(graph: PRODUCTS, key: "id") {
            """
            pricing
            """
            id: ID!
            """
            pricing
            """
            price(
              """
              pricing
              """
              currency: String = "USD"
            ): Price!
            promoCodes: [PromoCode] @join__field(graph: PRICING)
            """
            reviews
            """
            reviews(
              """
              reviews
              """
              limit: Int
            ): [Review] @join__field(graph: REVIEWS)
            """
            products
            """
            title: String @join__field(graph: PRODUCTS)
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          reviews
          """
          type Review @join__type(graph: REVIEWS, key: "id") {
            """
            reviews
            """
            id: ID!

            """
            reviews
            """
            rating: Float

            """
            reviews
            """
            content: String
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          reviews
          """
          type Query
            @join__type(graph: PRODUCTS)
            @join__type(graph: REVIEWS)
            @join__type(graph: PRICING) {
            """
            products
            """
            product(
              """
              products
              """
              id: ID!
            ): Product @join__field(graph: PRODUCTS)
            """
            reviews
            """
            review(
              """
              reviews
              """
              id: ID!
            ): Review @join__field(graph: REVIEWS)
          }
        "#);
        }

        fn on_input_object_types(api, version) {
            // @apollo/composition sorts services by name A->Z
            // and the first detected description wins.
            // That's why descriptions from `a` service are available on `UserInput`.
            // In case `UserInput` has no description in `a` service, the description from `b` service is NOT used.
            // It's weird but let's follow their logic for consistency.
            let result = api.compose_services(&[
                service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                """
                b
                """
                input UserInput {
                  """
                  b
                  """
                  name: String!
                  """
                  b
                  """
                  email: String
                }

                type Library @shareable {
                  book(title: String, section: String): String
                }

                type Query {
                  library: Library
                }
              "#, version)),
                service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                input UserInput {
                  """
                  a
                  """
                  name: String!
                  """
                  a
                  """
                  age: Int
                }

                type Library @shareable {
                  book(title: String, author: String): String
                }
              "#, version)),
            ]);
            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          b
          """
          input UserInput @join__type(graph: A) @join__type(graph: B) {
            """
            a
            """
            name: String!
          }
        "#);
        }
    }

    mod on_enum_types {
        use super::*;

        test_versions! {
            fn when_used_as_the_return_type_for_at_least_one_object_or_interface_field(api, version) {
                let result = api.compose_services(&[
                    service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                """
                a
                """
                enum UserType {
                  """
                  a
                  """
                  ADMIN
                }
              "#, version)),
                    service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                """
                b
                """
                enum UserType {
                  """
                  b
                  """
                  REGULAR
                }

                type Query {
                  users: [User]
                }
              "#, version)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
            """
            a
            """
            enum UserType @join__type(graph: A) @join__type(graph: B) {
              """
              a
              """
              ADMIN @join__enumValue(graph: A)

              """
              b
              """
              REGULAR @join__enumValue(graph: B)
            }
          "#);
            }

            fn when_used_as_the_type_for_at_least_one_field_argument_or_input_type_field(api, version) {
                let result = api.compose_services(&[
                    service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                }

                """
                a
                """
                enum UserType {
                  """
                  a
                  """
                  ADMIN
                  """
                  a
                  """
                  REGULAR
                }
              "#, version)),
                    service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                }

                """
                b
                """
                enum UserType {
                  """
                  b
                  """
                  REGULAR
                  """
                  b
                  """
                  ANONYMOUS
                }

                type Query {
                  users(type: UserType): [User]
                }
              "#, version)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
            """
            a
            """
            enum UserType @join__type(graph: A) @join__type(graph: B) {
              """
              a
              """
              REGULAR @join__enumValue(graph: A) @join__enumValue(graph: B)
            }
          "#);
            }

            fn when_used_as_the_type_for_at_least_one_field_argument_or_input_type_field_and_as_the_return_type_for_at_least_one_object_or_interface_field(api, version) {
                let result = api.compose_services(&[
                    service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                """
                b
                """
                enum UserType {
                  """
                  b
                  """
                  REGULAR
                  """
                  b
                  """
                  ADMIN
                }

                type Query {
                  users(type: UserType): [User]
                }
              "#, version)),
                    service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                """
                a
                """
                enum UserType {
                  """
                  a
                  """
                  ADMIN
                  REGULAR
                }
              "#, version)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
            """
            a
            """
            enum UserType @join__type(graph: A) @join__type(graph: B) {
              """
              a
              """
              ADMIN @join__enumValue(graph: A) @join__enumValue(graph: B)

              """
              b
              """
              REGULAR @join__enumValue(graph: A) @join__enumValue(graph: B)
            }
          "#);
            }
        }
    }

    test_versions! {
        fn on_union_types(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

                union Media = Movie

                type Movie {
                  id: ID!
                  title: String!
                }

                type Admin @key(fields: "id") {
                  id: ID!
                }
            "#, version)),
                service("c", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

                """
                c
                """
                union Media = Song

                type Song {
                  id: ID!
                  title: String!
                }

                type Anonymous @key(fields: "id") {
                  id: ID!
                }
            "#, version)),
                service("b", graphql!(r#"
              extend schema
                @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

                """
                b
                """
                union Media = Book

                type Book {
                  id: ID!
                  title: String!
                }

                type User @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  media(id: ID!): Media
                }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          b
          """
          union Media
            @join__type(graph: A)
            @join__type(graph: B)
            @join__type(graph: C)
            @join__unionMember(graph: A, member: "Movie")
            @join__unionMember(graph: B, member: "Book")
            @join__unionMember(graph: C, member: "Song") =
            | Movie
            | Book
            | Song
        "#);
        }

        fn on_interface_types(api, version) {
            let result = api.compose_services(&[
                service("c", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type User @key(fields: "id") {
                id: ID!
                age: Int!
              }

              """
              c
              """
              interface Details {
                """
                c
                """
                age: Int
              }
            "#, version)),
                service("a", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type User @key(fields: "id") {
                id: ID!
                name: String!
              }

              interface Details {
                """
                a
                """
                name: ID!
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type User @key(fields: "id") {
                id: ID!
                email: String!
              }

              """
              b
              """
              interface Details {
                """
                b
                """
                email: String
                """
                b
                """
                age: Int
              }

              type Query {
                user: User
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          """
          b
          """
          interface Details
            @join__type(graph: A)
            @join__type(graph: B)
            @join__type(graph: C) {
            """
            a
            """
            name: ID! @join__field(graph: A)
            """
            b
            """
            email: String @join__field(graph: B)
            """
            b
            """
            age: Int @join__field(graph: B) @join__field(graph: C)
          }
        "#);
        }
    }
}

mod deprecated {
    use super::*;

    test_versions! {
        fn on_object_types(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

              extend type Query {
                reviews(
                  limit: Int @deprecated
                ): [Review] @deprecated
              }

              extend type Product @key(fields: "id") {
                id: ID! @external @deprecated(reason: "a")
                reviews: [Review] @deprecated
              }

              type Review @key(fields: "id") {
                id: ID! @deprecated
                rating: Float @deprecated
                content: String @deprecated
              }
            "#, version)),
                service("b", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type Product @key(fields: "id") {
                id: ID! @deprecated(reason: "b")
                title: String @deprecated
                price: Price! @deprecated
              }

              type Price {
                amount: Float @deprecated
              }

              extend type Query {
                product(
                  id: ID!
                ): Product @deprecated
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Price @join__type(graph: B) {
            amount: Float @deprecated
          }
        "#);
            // Seems like first @deprecated wins (see the `id` field - `a` won, because it was first, even though the field is defined in `b`)
            to_contain_graphql(result.supergraph_sdl(), r#"
          type Product
            @join__type(graph: B, key: "id")
            @join__type(graph: A, key: "id", extension: true) {
            id: ID! @deprecated(reason: "a")
            title: String @join__field(graph: B) @deprecated
            price: Price! @join__field(graph: B) @deprecated
            reviews: [Review] @join__field(graph: A) @deprecated
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Review @join__type(graph: A, key: "id") {
            id: ID! @deprecated
            rating: Float @deprecated
            content: String @deprecated
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          type Query @join__type(graph: B) @join__type(graph: A) {
            product(id: ID!): Product @join__field(graph: B) @deprecated
            reviews(limit: Int @deprecated): [Review]
              @join__field(graph: A)
              @deprecated
          }
        "#);
        }

        fn on_input_object_types(api, version) {
            let result = api.compose_services(&[
                service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                input UserInput {
                  name: String @deprecated(reason: "b")
                  email: String @deprecated(reason: "b")
                }

                type Library @shareable {
                  book(title: String, section: String): String
                }

                type Query {
                  library: Library
                }
              "#, version)),
                service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                input UserInput {
                  name: String
                  email: String @deprecated(reason: "a")
                  age: Int @deprecated(reason: "a")
                }

                type Library @shareable {
                  book(title: String, author: String): String
                }
              "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          input UserInput @join__type(graph: A) @join__type(graph: B) {
            name: String @deprecated(reason: "b")
            email: String @deprecated(reason: "a")
          }
        "#);
        }
    }

    mod on_enum_types {
        use super::*;

        test_versions! {
            fn when_used_as_the_return_type_for_at_least_one_object_or_interface_field(api, version) {
                let result = api.compose_services(&[
                    service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                enum UserType {
                  ADMIN @deprecated(reason: "a")
                }
              "#, version)),
                    service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                enum UserType {
                  REGULAR @deprecated(reason: "b")
                }

                type Query {
                  users: [User]
                }
              "#, version)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
            enum UserType @join__type(graph: A) @join__type(graph: B) {
              ADMIN @join__enumValue(graph: A) @deprecated(reason: "a")
              REGULAR @join__enumValue(graph: B) @deprecated(reason: "b")
            }
          "#);
            }

            fn when_used_as_the_type_for_at_least_one_field_argument_or_input_type_field(api, version) {
                let result = api.compose_services(&[
                    service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                }

                enum UserType {
                  ADMIN @deprecated(reason: "a")
                  REGULAR
                }
              "#, version)),
                    service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                }

                enum UserType {
                  REGULAR @deprecated(reason: "b")
                  ANONYMOUS @deprecated(reason: "b")
                }

                type Query {
                  users(type: UserType): [User]
                }
              "#, version)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
            enum UserType @join__type(graph: A) @join__type(graph: B) {
              REGULAR
                @join__enumValue(graph: A)
                @join__enumValue(graph: B)
                @deprecated(reason: "b")
            }
          "#);
            }

            fn when_used_as_the_type_for_at_least_one_field_argument_or_input_type_field_and_as_the_return_type_for_at_least_one_object_or_interface_field(api, version) {
                let result = api.compose_services(&[
                    service("b", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                enum UserType {
                  REGULAR @deprecated(reason: "b")
                  ADMIN
                }

                type Query {
                  users(type: UserType): [User]
                }
              "#, version)),
                    service("a", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

                type User @shareable {
                  name: String!
                  type: UserType
                }

                enum UserType {
                  ADMIN
                  REGULAR @deprecated(reason: "a")
                }
              "#, version)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
            enum UserType @join__type(graph: A) @join__type(graph: B) {
              ADMIN @join__enumValue(graph: A) @join__enumValue(graph: B)
              REGULAR
                @join__enumValue(graph: A)
                @join__enumValue(graph: B)
                @deprecated(reason: "a")
            }
          "#);
            }
        }
    }
}

test_versions! {
    fn enum_join_graph_must_contain_all_subgraphs_with_their_urls(api, version) {
        let result = api.compose_services(&[
            service_with_url("users", "https://users.com", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "email") {
              email: ID!
              name: String
              totalProductsCreated: Int
            }
          "#, version)),
            service("pandas", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              allPandas: [Panda]
              panda(name: ID!): Panda
            }

            type Panda {
              name: ID!
              favoriteFood: String
            }
          "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        enum join__Graph {
          PANDAS @join__graph(name: "pandas", url: "")
          USERS @join__graph(name: "users", url: "https://users.com")
        }
      "#);
    }
}

mod federation_directives_and_types {
    use super::*;

    test_versions! {
        fn link_and_join_spec(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String
            }

            type Query {
              user: User
            }
          "#, version)),
                service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User @shareable {
              name: String
            }
          "#, version)),
            ]);

            assert_composition_success(&result);

            let definitions = parse(link::SDL).unwrap().definitions.into_iter().chain(parse(&join::sdl(FederationVersion::from_str(version).unwrap()))
                .unwrap()
                .definitions);
            for def in definitions {
                to_contain_graphql(result.supergraph_sdl(), print(&Document::new(vec![def.clone()])));
            }
        }

        fn v1_link_and_join_spec(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              type User {
                name: String
              }

              type Query {
                user: User
              }
            "#)),
                service("b", graphql!(r#"
              type Dog {
                name: String
              }

              type Query {
                dog: Dog
              }
            "#)),
            ]);

            assert_composition_success(&result);

            let definitions = parse(link::SDL).unwrap().definitions.into_iter().chain(parse(&join::sdl(FederationVersion::from_str("v1.0").unwrap()))
                .unwrap()
                .definitions);
            for def in definitions {
                to_contain_graphql(result.supergraph_sdl(), print(&Document::new(vec![def.clone()])));
            }
        }
    }
}

mod federation_v1_only {
    use super::*;

    test_implementations! {
        fn delete_subgraph_spec(api) {
            let mut result = api.compose_services(&[
                service("users", graphql!(r#"
            type User {
              name: String!
              blocked: Boolean!
            }

            type Query {
              _entities(representations: [_Any!]!): [_Entity]!
            }

            extend type Query {
              users: [User]!
            }

            extend type Query {
              _service: _Service!
            }

            scalar _FieldSet
            scalar _Any

            union _Entity = User

            type _Service {
              sdl: String
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: USERS) {
          users: [User]!
        }
      "#);

            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("type _Service")));
            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("scalar _Any")));
            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("scalar _FieldSet")));
            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("union _Entity")));

            result = api.compose_services(&[
                service("users", graphql!(r#"
            type User {
              name: String!
              blocked: Boolean!
            }

            type RootQuery {
              _entities(representations: [_Any!]!): [_Entity]!
            }

            schema {
              query: RootQuery
            }

            extend type RootQuery {
              users: [User]!
            }

            extend type RootQuery {
              _service: _Service!
            }

            scalar _FieldSet
            scalar _Any

            union _Entity = User

            type _Service {
              sdl: String
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: USERS) {
          users: [User]!
        }
      "#);

            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("type _Service")));
            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("scalar _Any")));
            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("scalar _FieldSet")));
            assert!(!result.supergraph_sdl().is_some_and(|s| s.contains("union _Entity")));
        }
    }

    mod tag {
        use super::*;

        test_implementations! {
            fn without_definition(api) {
                let result = api.compose_services(&[
                    service("users", graphql!(r#"
              type User @key(fields: "id") {
                id: ID!
                name: String! @tag(name: "private")
                blocked: Boolean!
              }

              type Query {
                users: [User]!
              }
            "#)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
          type User @join__type(graph: USERS, key: "id") {
            id: ID!
            name: String! @tag(name: "private")
            blocked: Boolean!
          }
        "#);

                to_contain_graphql(result.public_sdl(), r#"
          type User {
            id: ID!
            name: String!
            blocked: Boolean!
          }
        "#);

                to_contain_graphql(result.supergraph_sdl(), Definition::DirectiveDefinition(TAG_DIRECTIVE.clone()));
                assert_not_to_match_string(result.public_sdl(), "@tag");

                to_contain_graphql(result.supergraph_sdl(), r#"
          schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION)
            @link(url: "https://specs.apollo.dev/tag/v0.3") {
            query: Query
          }
        "#);
            }

            fn with_definition(api) {
                let result = api.compose_services(&[
                    service("users", graphql!(r#"
              directive @tag(name: String!) repeatable on FIELD_DEFINITION

              type User @key(fields: "id") {
                id: ID!
                name: String! @tag(name: "private")
                blocked: Boolean!
              }

              type Query {
                users: [User]!
              }
            "#)),
                ]);

                assert_composition_success(&result);

                to_contain_graphql(result.supergraph_sdl(), r#"
          type User @join__type(graph: USERS, key: "id") {
            id: ID!
            name: String! @tag(name: "private")
            blocked: Boolean!
          }
        "#);

                to_contain_graphql(result.public_sdl(), r#"
          type User {
            id: ID!
            name: String!
            blocked: Boolean!
          }
        "#);
                // The original definition should be ignored
                to_contain_graphql(result.supergraph_sdl(), Definition::DirectiveDefinition(TAG_DIRECTIVE.clone()));

                to_contain_graphql(result.supergraph_sdl(), r#"
          schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION)
            @link(url: "https://specs.apollo.dev/tag/v0.3") {
            query: Query
          }
        "#);
            }
        }
    }
}

test_implementations! {
    fn external_on_an_unused_field_in_an_extension_type(api) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
          type User @extends @key(fields: "id") {
            a: String
            id: ID! @external
            uuid: ID! @external
          }

          type Query {
            a: String
          }
        "#)),
            service("b", graphql!(r#"
          type User @extends @key(fields: "id") {
            b: String @requires(fields: "uuid")
            id: ID! @external
            uuid: ID! @external
          }

          type Query {
            b: String
          }
        "#)),
            service("c", graphql!(r#"
          type User @key(fields: "id") {
            c: String
            id: ID!
            uuid: ID!
          }

          type Query {
            c: User
          }
        "#)),
            service("extra", graphql!(r#"
          type Query {
            extra: String
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(extension: true, graph: A, key: "id")
        @join__type(extension: true, graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        a: String @join__field(graph: A)
        b: String @join__field(graph: B, requires: "uuid")
        c: String @join__field(graph: C)
        uuid: ID! @join__field(external: true, graph: B) @join__field(graph: C)
        id: ID!
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          type User @extends @key(fields: "id") {
            a: String
            id: ID! @external
            uuid: ID! @external
          }

          type Query {
            a: String
          }
        "#)),
            service("b", graphql!(r#"
          type User @extends @key(fields: "id") {
            b: String @requires(fields: "uuid")
            id: ID! @external
            uuid: ID! @external
          }

          type Query {
            b: String
          }
        "#)),
            service("c", graphql!(r#"
          type User @key(fields: "id") {
            c: String
            id: ID!
            uuid: ID!
          }

          type Query {
            c: User
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(extension: true, graph: A, key: "id")
        @join__type(extension: true, graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        a: String @join__field(graph: A)
        b: String @join__field(graph: B, requires: "uuid")
        c: String @join__field(graph: C)
        uuid: ID! @join__field(external: true, graph: B) @join__field(graph: C)
        id: ID!
      }
    "#);
    }

    fn fed_v1_external_on_a_provided_field_in_an_extension_type(api) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
          type User @key(fields: "id") @key(fields: "uuid") {
            id: ID!
            uuid: String!
            name: String!
          }

          type Query {
            a: User
          }
        "#)),
            service("b", graphql!(r#"
          extend type User @key(fields: "uuid") {
            uuid: String! @external
          }

          type B {
            id: ID!
            user: User!
          }
        "#)),
            service("c", graphql!(r#"
          type User @key(fields: "id") @key(fields: "uuid") {
            id: ID!
            uuid: String!
            commentCount: Int!
          }
        "#)),
            service("d", graphql!(r#"
          extend type User @key(fields: "uuid") {
            uuid: String! @external
            id: ID! @external
          }

          type D {
            id: ID!
            user: User! @provides(fields: "id")
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: A, key: "uuid")
        @join__type(graph: B, key: "uuid")
        @join__type(graph: C, key: "id")
        @join__type(graph: C, key: "uuid")
        @join__type(graph: D, key: "uuid") {
        id: ID!
          @join__field(external: true, graph: D)
          @join__field(graph: A)
          @join__field(graph: C)
        uuid: String!
        name: String! @join__field(graph: A)
        commentCount: Int! @join__field(graph: C)
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          type User @key(fields: "id") @key(fields: "uuid") {
            id: ID!
            uuid: String!
            name: String!
          }

          type Query {
            a: User
          }
        "#)),
            service("b", graphql!(r#"
          extend type User @key(fields: "uuid") {
            uuid: String! @external
          }

          type B {
            id: ID!
            user: User!
          }
        "#)),
            service("c", graphql!(r#"
          type User @key(fields: "id") @key(fields: "uuid") {
            id: ID!
            uuid: String!
            commentCount: Int!
          }
        "#)),
            service("d", graphql!(r#"
          extend type User @key(fields: "uuid") {
            uuid: String! @external
            id: ID! @external
          }

          type D {
            id: ID!
            user: User! @provides(fields: "id")
          }
        "#)),
            service("extra", graphql!(r#"
          type Query {
            extra: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: A, key: "uuid")
        @join__type(graph: B, key: "uuid")
        @join__type(graph: C, key: "id")
        @join__type(graph: C, key: "uuid")
        @join__type(graph: D, key: "uuid") {
        id: ID!
          @join__field(external: true, graph: D)
          @join__field(graph: A)
          @join__field(graph: C)
        uuid: String!
        name: String! @join__field(graph: A)
        commentCount: Int! @join__field(graph: C)
      }
    "#);
    }

    fn unused_external_on_key_field_of_an_entity_type(api) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
          type Site @key(fields: "header") @key(fields: "footer") {
            header: String @external
            footer: String @external
            headerTitle: String
            footerTitle: String
          }

          type Query {
            a: String
          }
        "#)),
            service("b", graphql!(r#"
          type Site @key(fields: "id") {
            id: ID!
            header: String
            footer: String
            title: String
          }

          type Query {
            site: Site
          }
        "#)),
            service("c", graphql!(r#"
          type Site @key(fields: "id") {
            id: ID!
            tag: String
          }

          type Query {
            c: Site
          }
        "#)),
            service("extra", graphql!(r#"
          type Query {
            extra: String
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Site
        @join__type(graph: A, key: "header")
        @join__type(graph: A, key: "footer")
        @join__type(graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        header: String
          @join__field(external: true, graph: A)
          @join__field(graph: B)
        headerTitle: String @join__field(graph: A)
        id: ID! @join__field(graph: B) @join__field(graph: C)
        footer: String
          @join__field(external: true, graph: A)
          @join__field(graph: B)
        footerTitle: String @join__field(graph: A)
        title: String @join__field(graph: B)
        tag: String @join__field(graph: C)
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          type Site @key(fields: "header") @key(fields: "footer") {
            header: String @external
            footer: String @external
            headerTitle: String
            footerTitle: String
          }

          type Query {
            a: String
          }
        "#)),
            service("b", graphql!(r#"
          type Site @key(fields: "id") {
            id: ID!
            header: String
            footer: String
            title: String
          }

          type Query {
            site: Site
          }
        "#)),
            service("c", graphql!(r#"
          type Site @key(fields: "id") {
            id: ID!
            tag: String
          }

          type Query {
            c: Site
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Site
        @join__type(graph: A, key: "header")
        @join__type(graph: A, key: "footer")
        @join__type(graph: B, key: "id")
        @join__type(graph: C, key: "id") {
        header: String
          @join__field(external: true, graph: A)
          @join__field(graph: B)
        headerTitle: String @join__field(graph: A)
        id: ID! @join__field(graph: B) @join__field(graph: C)
        footer: String
          @join__field(external: true, graph: A)
          @join__field(graph: B)
        footerTitle: String @join__field(graph: A)
        title: String @join__field(graph: B)
        tag: String @join__field(graph: C)
      }
    "#);
    }

    fn external_on_non_key_field_of_an_entity_type(api) {
        // "apollo" errors on this schema definition
        {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/v2.8"
                import: ["@key", "@external", "@requires", "@tag"]
              )

            type User @key(fields: "id") {
              id: ID! @tag(name: "public")
              creditScore: Int @requires(fields: "ssn") @tag(name: "public")
            }

            extend type User @external {
              ssn: String
            }
          "#)),
                service("b", graphql!(r#"
            schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/v2.8"
                import: ["@key", "@tag"]
              ) {
              query: Query
            }

            type Query {
              user: User @tag(name: "public")
            }

            type User @key(fields: "id") {
              id: ID! @tag(name: "public")
              ssn: String
            }
          "#)),
            ]);

            assert!(result.errors().is_none());
            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: A) @join__type(graph: B) {
          user: User @join__field(graph: B) @tag(name: "public")
        }

        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID! @tag(name: "public")
          creditScore: Int
            @join__field(graph: A, requires: "ssn")
            @tag(name: "public")
          ssn: String
            @join__field(external: true, graph: A)
            @join__field(graph: B)
        }
      "#);
        }

        let mut result = api.compose_services(&[
            service("foo", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external", "@provides", "@shareable"]
            )
          type Note @key(fields: "id") @shareable {
            id: ID!
            name: String @external
            author: User @external
          }
          type User @key(fields: "id", resolvable: false) {
            id: ID!
          }
          type PrivateNote @key(fields: "id") @shareable {
            id: ID!
            note: Note @provides(fields: "name author { id }")
          }
          type Query {
            note: Note @shareable
            privateNote: PrivateNote @shareable
          }
        "#)),
            service("bar", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable"]
            )
          type Note @key(fields: "id") @shareable {
            id: ID!
            name: String
            author: User
          }
          type User @key(fields: "id") {
            id: ID!
            name: String
          }
          type PrivateNote @key(fields: "id") @shareable {
            id: ID!
            note: Note
          }
          type Query {
            note: Note @shareable
            privateNote: PrivateNote @shareable
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Note
        @join__type(graph: FOO, key: "id")
        @join__type(graph: BAR, key: "id") {
        id: ID!
        name: String
          @join__field(external: true, graph: FOO)
          @join__field(graph: BAR)
        author: User
          @join__field(external: true, graph: FOO)
          @join__field(graph: BAR)
      }
    "#);

        result = api.compose_services(&[
            service("foo", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external", "@provides", "@shareable"]
            )
          type Note @key(fields: "id") @shareable {
            id: ID!
            name: String @external
            author: User @external
          }
          type User @key(fields: "id", resolvable: false) {
            id: ID!
          }
          type PrivateNote @key(fields: "id") @shareable {
            id: ID!
            note: Note @provides(fields: "name author { id }")
          }
          type Query {
            note: Note @shareable
            privateNote: PrivateNote @shareable
          }
        "#)),
            service("bar", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable"]
            )
          type Note @key(fields: "id") @shareable {
            id: ID!
            name: String
            author: User
          }
          type User @key(fields: "id") {
            id: ID!
            name: String
          }
          type PrivateNote @key(fields: "id") @shareable {
            id: ID!
            note: Note
          }
          type Query {
            note: Note @shareable
            privateNote: PrivateNote @shareable
          }
        "#)),
            service("baz", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external", "@provides", "@shareable"]
            )
          type Query {
            hello: String
          }
          type Note @key(fields: "id") @shareable {
            id: ID!
            tag: String
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Note
        @join__type(graph: FOO, key: "id")
        @join__type(graph: BAR, key: "id")
        @join__type(graph: BAZ, key: "id") {
        id: ID!
        name: String
          @join__field(external: true, graph: FOO)
          @join__field(graph: BAR)
        author: User
          @join__field(external: true, graph: FOO)
          @join__field(graph: BAR)
        tag: String @join__field(graph: BAZ)
      }
    "#);
    }

    fn print_join_field_external_the_field_is_required_in_a_deeply_nested_selection_set(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          type Query {
            a: String
          }

          type User {
            id: ID!
            age: Int!
            name: String!
          }
        "#)),
            service("b", graphql!(r#"
          type Query {
            b: String
          }

          type Book {
            author: User @requires(fields: "author { name }")
          }

          extend type User {
            name: String! @external
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String!
          @join__field(external: true, graph: B)
          @join__field(graph: A)
        id: ID! @join__field(graph: A)
        age: Int! @join__field(graph: A)
      }
    "#);
    }

    fn query_field_with_override_that_points_to_non_existing_subgraph(api) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@override"]
            )

          type Query {
            a: String @override(from: "non-existing")
          }
        "#)),
        ]);
        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query @join__type(graph: A) {
        a: String
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@override"]
            )

          type Query {
            a: String @override(from: "non-existing")
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@override"]
            )

          type Query {
            b: String
          }
        "#)),
        ]);
        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query @join__type(graph: A) @join__type(graph: B) {
        a: String @join__field(graph: A, override: "non-existing")
        b: String @join__field(graph: B)
      }
    "#);
    }

    fn drop_unused_external_fields_from_federation_v1_subgraphs(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          type User @key(fields: "id") {
            id: ID!
            name: String @external
            age: Int!
          }

          type Query {
            a: String
          }
        "#)),
            service("b", graphql!(r#"
          type User @key(fields: "id") {
            id: ID!
            age: Int! @external
            birthday: String @requires(fields: "age")
          }

          type Query {
            b: String
          }
        "#)),
        ]);

        assert_composition_success(&result);
        // No User.name, it's dropped
        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id") {
        id: ID!
        age: Int! @join__field(external: true, graph: B) @join__field(graph: A)
        birthday: String @join__field(graph: B, requires: "age")
      }
    "#);
    }

    fn join_field_used_overridden_true_on_a_field_that_is_a_key_field_but_not_external(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@override"]
            )

          type User @key(fields: "userId") {
            userId: ID! @override(from: "b")
            age: Int! @override(from: "b")
          }

          type Query {
            a: String
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@override"]
            )

          type User @key(fields: "userId") {
            userId: ID!
            age: Int!
          }

          type Query {
            b: String
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "userId")
        @join__type(graph: B, key: "userId") {
        age: Int! @join__field(graph: A, override: "b")
        userId: ID!
          @join__field(graph: A, override: "b")
          @join__field(graph: B, usedOverridden: true)
      }
    "#);
    }

    fn deduplicates_directives(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable", "@composeDirective"]
            )
            @link(
              url: "https://myspecs.dev/lowercase/v1.0"
              import: ["@lowercase"]
            )
            @composeDirective(name: "@lowercase")

          directive @lowercase on FIELD_DEFINITION | ENUM

          type User @key(fields: "id") {
            id: ID! @lowercase
            age: Int!
            type: UserType! @shareable
          }

          enum UserType @lowercase {
            REGULAR
            ADMIN
          }

          type Query {
            a: String
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: [
                "@key"
                "@external"
                "@requires"
                "@shareable"
                "@composeDirective"
              ]
            )
            @link(
              url: "https://myspecs.dev/lowercase/v1.0"
              import: ["@lowercase"]
            )
            @composeDirective(name: "@lowercase")

          directive @lowercase on FIELD_DEFINITION | ENUM

          type User @key(fields: "id") {
            id: ID! @lowercase
            age: Int! @external
            birthday: String @requires(fields: "age")
            type: UserType! @lowercase @shareable
          }

          enum UserType @lowercase {
            REGULAR
            ADMIN
          }

          type Query {
            b: String
          }
        "#)),
        ]);

        assert_composition_success(&result);
        // No User.name, it's dropped
        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id") {
        id: ID! @lowercase
        age: Int! @join__field(external: true, graph: B) @join__field(graph: A)
        birthday: String @join__field(graph: B, requires: "age")
        type: UserType! @lowercase
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      enum UserType @join__type(graph: A) @join__type(graph: B) @lowercase {
        ADMIN @join__enumValue(graph: A) @join__enumValue(graph: B)
        REGULAR @join__enumValue(graph: A) @join__enumValue(graph: B)
      }
    "#);
    }

    fn reproduction_gw_164_empty_list_as_a_non_nullable_enum_value_argument_of_a_directive(api) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.7"
              import: ["@key", "@composeDirective"]
            )
            @link(
              url: "https://myspecs.dev/myDirective/v1.0"
              import: ["@myDirective"]
            )
            @composeDirective(name: "@myDirective")

          directive @myDirective(myarg: [MyEnum!]!) on OBJECT
          enum MyEnum {
            MY_ENUM_VALUE
          }
          type Query {
            myRootField: MyObject
          }

          type MyObject @myDirective(myarg: []) {
            myField: String
          }
        "#)),
        ]);
        assert_composition_success(&result);
        to_contain_graphql(result.supergraph_sdl(), r#"
      type MyObject @join__type(graph: A) @myDirective(myarg: []) {
        myField: String
      }
    "#);
    }

    fn tag_usage_without_name_argument_raises_no_exception(api) {
        let result = api.compose_services(&[
            service("a service", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@tag"]
            )

          type Query {
            ping: String
            pong: String
            foo: User @tag
          }

          type User {
            id: ID!
          }
        "#)),
        ]);
        assert_eq!(result.errors().unwrap().len(), 1);
        assert!(result.errors()
            .and_then(|errors| errors.first())
            .map(|e| e.message.as_str())
            .unwrap()
            .contains(r#"Argument "@tag(name:)" of type "String!" is required, but it was not provided."#));
    }

    fn tag_usage_with_wrong_name_argument_type_raises_no_exception(api) {
        let result = api.compose_services(&[
            service("a service", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@tag"]
            )

          type Query {
            ping: String
            pong: String
            foo: User @tag(name: {})
          }

          type User {
            id: ID!
          }
        "#)),
        ]);
        assert_eq!(result.errors().unwrap().len(), 1);
        assert!(result.errors()
            .and_then(|errors| errors.first())
            .map(|e| e.message.as_str())
            .unwrap()
            .contains(r#"Invalid value for "@tag(name:)" of type "String!" in application of "@tag" to "Query.foo"."#));
    }

    fn inaccessible_interface_implemented_by_accessible_types(api) {
        let sdl = tpl!(r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@inaccessible"]
        ) {
        query: Query
      }

      type Query {
        public: PublicAccessTokenConnection!
        private: PrivateAccessTokenConnection! @inaccessible
      }

      interface AccessToken @inaccessible {
        id: ID!
      }

      interface AccessTokenEdge @inaccessible {
        node: AccessToken!
      }

      interface AccessTokenConnection @inaccessible {
        edges: [AccessTokenEdge!]!
      }

      type PrivateAccessToken implements AccessToken @inaccessible {
        id: ID!
      }

      type PrivateAccessTokenEdge implements AccessTokenEdge @inaccessible {
        node: PrivateAccessToken!
      }

      type PrivateAccessTokenConnection implements AccessTokenConnection
        @inaccessible {
        edges: [PrivateAccessTokenEdge!]!
      }

      type PublicAccessToken implements AccessToken {
        id: ID!
      }

      type PublicAccessTokenEdge implements AccessTokenEdge {
        node: PublicAccessToken!
      }

      type PublicAccessTokenConnection implements AccessTokenConnection {
        edges: [PublicAccessTokenEdge!]!
      }
    "#);

        let result = api.compose_services(&[
            service_with_url("FOO_GRAPHQL", "https://lol.de", graphql(&sdl)),
        ]);

        assert!(result.errors().is_none());
        to_contain_graphql(result.supergraph_sdl(), r#"
      type PrivateAccessToken implements AccessToken
        @join__implements(graph: FOO_GRAPHQL, interface: "AccessToken")
        @join__type(graph: FOO_GRAPHQL)
        @inaccessible {
        id: ID!
      }

      type PrivateAccessTokenConnection implements AccessTokenConnection
        @join__implements(
          graph: FOO_GRAPHQL
          interface: "AccessTokenConnection"
        )
        @join__type(graph: FOO_GRAPHQL)
        @inaccessible {
        edges: [PrivateAccessTokenEdge!]!
      }

      type PrivateAccessTokenEdge implements AccessTokenEdge
        @join__implements(graph: FOO_GRAPHQL, interface: "AccessTokenEdge")
        @join__type(graph: FOO_GRAPHQL)
        @inaccessible {
        node: PrivateAccessToken!
      }

      type PublicAccessToken implements AccessToken
        @join__implements(graph: FOO_GRAPHQL, interface: "AccessToken")
        @join__type(graph: FOO_GRAPHQL) {
        id: ID!
      }

      type PublicAccessTokenConnection implements AccessTokenConnection
        @join__implements(
          graph: FOO_GRAPHQL
          interface: "AccessTokenConnection"
        )
        @join__type(graph: FOO_GRAPHQL) {
        edges: [PublicAccessTokenEdge!]!
      }

      type PublicAccessTokenEdge implements AccessTokenEdge
        @join__implements(graph: FOO_GRAPHQL, interface: "AccessTokenEdge")
        @join__type(graph: FOO_GRAPHQL) {
        node: PublicAccessToken!
      }
    "#);
        assert_composition_success(&result);
        to_contain_graphql(result.public_sdl(), r#"
      type Query {
        public: PublicAccessTokenConnection!
      }

      type PublicAccessToken {
        id: ID!
      }

      type PublicAccessTokenEdge {
        node: PublicAccessToken!
      }

      type PublicAccessTokenConnection {
        edges: [PublicAccessTokenEdge!]!
      }
    "#);
    }

    fn field_set_does_not_leak_as_external_type_onto_supergraph_if_defined_in_subgraph(api) {
        let result = api.compose_services(&[
            service_with_url("a", "a", graphql!(r#"
          schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.5"
              import: ["@key", "FieldSet"]
            ) {
            query: Query
            mutation: Mutation
          }

          interface Node {
            id: ID!
          }

          type Mutation {
            _: String
          }

          type Query {
            node(id: ID!): Node
            _service: _Service!
            _entities(representations: [_Any!]!): [_Entity]!
          }

          type User implements Node
            @key(fields: "id")
            @key(fields: "username") {
            id: ID!
            username: String
          }

          type _Service {
            sdl: String!
          }

          union _Entity = User

          directive @key(
            fields: FieldSet!
            resolvable: Boolean = true
          ) repeatable on OBJECT | INTERFACE

          directive @link(url: String!, import: [String!]) repeatable on SCHEMA

          scalar FieldSet

          scalar Long

          scalar _Any
        "#)),
        ]);

        assert_composition_success(&result);
        assert!(!result.supergraph_sdl().unwrap().contains("scalar FieldSet"));
    }

    fn keeps_join_field_for_object_type_extension_with_key_being_external(api) {
        let result = api.compose_services(&[
            service("owner-service", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external"]
            )

          extend type Repository @key(fields: "id name") {
            id: ID! @external
            name: String! @external
          }
        "#)),
            service("team-service", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external"]
            )

          extend type Repository @key(fields: "id") {
            id: ID! @external
          }
        "#)),
            service("repo-service", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable"]
            )

          type Query {
            repo: Repository
          }

          type Repository @key(fields: "id") {
            id: ID!
            name: String! @shareable
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Repository
        @join__type(extension: true, graph: OWNER_SERVICE, key: "id name")
        @join__type(extension: true, graph: TEAM_SERVICE, key: "id")
        @join__type(graph: REPO_SERVICE, key: "id") {
        id: ID!
        name: String!
          @join__field(graph: OWNER_SERVICE)
          @join__field(graph: REPO_SERVICE)
      }
    "#);
    }

    fn keep_join_field_for_object_type_extension_where_key_is_external_and_have_other_external_fields_used_with_requires(api) {
        let result = api.compose_services(&[
            service("developer-service", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable"]
            )

          type Query {
            me: Developer
          }

          type Developer @key(fields: "id") @key(fields: "guestGuid") {
            id: ID!
            guestGuid: ID! @shareable
            isMe: Boolean! @shareable
            myFollowingStatus: String! @shareable
            userVisibility: String! @shareable
          }
        "#)),
            service("team-service", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external", "@requires"]
            )

          extend type Developer @key(fields: "guestGuid") {
            guestGuid: ID! @external
            myFollowingStatus: String! @external
            userVisibility: String! @external
            guestSavedLists: [ID!]!
              @requires(fields: "myFollowingStatus userVisibility")
          }
        "#)),
            service("commit-service", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external", "@requires"]
            )

          extend type Developer @key(fields: "id") {
            id: ID! @external
            isMe: Boolean! @external
            userVisibility: String! @external
            commits: [ID!]! @requires(fields: "isMe userVisibility")
          }
        "#)),
            service("reaction-service", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external"]
            )

          extend type Developer @key(fields: "id") {
            id: ID! @external
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Developer
        @join__type(extension: true, graph: TEAM_SERVICE, key: "guestGuid")
        @join__type(extension: true, graph: REACTION_SERVICE, key: "id")
        @join__type(extension: true, graph: COMMIT_SERVICE, key: "id")
        @join__type(graph: DEVELOPER_SERVICE, key: "guestGuid")
        @join__type(graph: DEVELOPER_SERVICE, key: "id") {
        guestGuid: ID!
          @join__field(graph: TEAM_SERVICE)
          @join__field(graph: DEVELOPER_SERVICE)
        guestSavedLists: [ID!]!
          @join__field(
            graph: TEAM_SERVICE
            requires: "myFollowingStatus userVisibility"
          )
        id: ID!
          @join__field(graph: REACTION_SERVICE)
          @join__field(graph: COMMIT_SERVICE)
          @join__field(graph: DEVELOPER_SERVICE)
        isMe: Boolean!
          @join__field(external: true, graph: COMMIT_SERVICE)
          @join__field(graph: DEVELOPER_SERVICE)
        myFollowingStatus: String!
          @join__field(external: true, graph: TEAM_SERVICE)
          @join__field(graph: DEVELOPER_SERVICE)
        commits: [ID!]!
          @join__field(graph: COMMIT_SERVICE, requires: "isMe userVisibility")
        userVisibility: String!
          @join__field(external: true, graph: TEAM_SERVICE)
          @join__field(external: true, graph: COMMIT_SERVICE)
          @join__field(graph: DEVELOPER_SERVICE)
      }
    "#);
    }

    fn external_on_non_key_field_in_federation_v1(api) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          type Query {
            users: [User]
          }

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
          }
        "#)),
            service("extra", graphql!(r#"
          type User @key(fields: "id") {
            id: ID!
            name: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);
        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: EXTRA, key: "id")
        @join__type(graph: USERS, key: "id") {
        id: ID!
        name: String! @join__field(graph: EXTRA)
      }
    "#);
    }

    fn external_on_non_key_field_in_federation_v1_2(api) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          type Query {
            users: [User]
          }

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
          }
        "#)),
            service("extra", graphql!(r#"
          type User @key(fields: "id") {
            id: ID!
            name: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);
        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: EXTRA, key: "id")
        @join__type(graph: USERS, key: "id") {
        id: ID!
        name: String! @join__field(graph: EXTRA)
      }
    "#);
    }

    fn external_on_non_key_field_in_federation_v2(api) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external"]
            )

          type Query {
            users: [User]
          }

          type User @key(fields: "id") {
            id: ID!
            name: String! @external
          }
        "#)),
            service("extra", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external"]
            )

          type User @key(fields: "id") {
            id: ID!
            name: String!
          }
        "#)),
        ]);

        assert_composition_failure(&result);
    }

    fn external_object_type_field_implementing_interface_field(api) {
        let result = api.compose_services(&[
            service("github_core", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key"]
            )

          type Query {
            checks: QueuedCheckRunInfo
          }

          interface CheckRunInfo {
            id: ID!
            status: String!
          }

          type QueuedCheckRunInfo implements CheckRunInfo @key(fields: "id") {
            id: ID!
            status: String!
            commitOid: String!
          }
        "#)),
            service("github_action", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@external", "@requires"]
            )

          extend type QueuedCheckRunInfo implements CheckRunInfo
            @key(fields: "id") {
            id: ID! @external
            status: String! @external
            commitOid: String! @external
            workflowRunId: ID! @requires(fields: "commitOid")
          }

          interface CheckRunInfo {
            id: ID!
            status: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      interface CheckRunInfo
        @join__type(graph: GITHUB_ACTION)
        @join__type(graph: GITHUB_CORE) {
        id: ID!
        status: String!
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type QueuedCheckRunInfo implements CheckRunInfo
        @join__implements(graph: GITHUB_ACTION, interface: "CheckRunInfo")
        @join__implements(graph: GITHUB_CORE, interface: "CheckRunInfo")
        @join__type(graph: GITHUB_CORE, key: "id")
        @join__type(graph: GITHUB_ACTION, key: "id", extension: true) {
        id: ID!
        status: String!
          @join__field(graph: GITHUB_CORE)
          @join__field(graph: GITHUB_ACTION, external: true)
        commitOid: String!
          @join__field(graph: GITHUB_CORE)
          @join__field(graph: GITHUB_ACTION, external: true)
        workflowRunId: ID!
          @join__field(graph: GITHUB_ACTION, requires: "commitOid")
      }
    "#);
    }
}

mod guild_composition_one_of {
    use super::*;

    test_implementations! {
        fn supports_composing_built_in_directive(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.0"
                  import: ["@key"]
                )

              input HelloInput @oneOf {
                world: String
                me: String
              }

              type Query {
                hello(input: HelloInput): String
              }
            "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          input HelloInput @oneOf @join__type(graph: A) {
            world: String
            me: String
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          directive @oneOf on INPUT_OBJECT
        "#);

            to_contain_graphql(result.public_sdl(), r#"
          directive @oneOf on INPUT_OBJECT
        "#);

            to_contain_graphql(result.public_sdl(), r#"
          input HelloInput @oneOf {
            world: String
            me: String
          }
        "#);
        }

        fn supports_composing_built_in_directive_with_matching_definition(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.0"
                  import: ["@key"]
                )

              directive @oneOf on INPUT_OBJECT

              input HelloInput @oneOf {
                world: String
                me: String
              }

              type Query {
                hello(input: HelloInput): String
              }
            "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
          input HelloInput @oneOf @join__type(graph: A) {
            world: String
            me: String
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
          directive @oneOf on INPUT_OBJECT
        "#);

            to_contain_graphql(result.public_sdl(), r#"
          input HelloInput @oneOf {
            world: String
            me: String
          }
        "#);
        }

        fn merges_built_in_one_of_with_an_explicitly_linked_and_composed_one_of_definition(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.0"
                  import: ["@key"]
                )

              input HelloInput @oneOf {
                world: String
                me: String
              }

              type Query {
                hello(input: HelloInput): String
              }
            "#)),
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key", "@composeDirective"]
                )
                @link(
                  url: "https://spec.graphql.org/specified/v1.6"
                  import: ["@oneOf"]
                )
                @composeDirective(name: "@oneOf")

              directive @oneOf on INPUT_OBJECT

              type Hello {
                world: String
              }

              type Query {
                greetings(input: GreetingsInput!): [Hello]
              }

              input GreetingsInput @oneOf {
                hello: String
                world: String
              }
            "#)),
            ]);

            assert_composition_success_with_message(
                &result,
                Some(&format!("Composition failed: {}", json_stringify_errors(result.errors()))),
            );
            assert!(result.errors().is_none());
            assert!(result.supergraph_sdl().is_some());
            if result.supergraph_sdl().is_some() {
                assert_build_schema_does_not_throw(result.supergraph_sdl().unwrap());
                to_contain_graphql(result.supergraph_sdl(), r#"directive @oneOf on INPUT_OBJECT"#);
                to_contain_graphql(result.supergraph_sdl(), r#"input GreetingsInput @join__type(graph: B) @oneOf"#);
                to_contain_graphql(result.supergraph_sdl(), r#"input HelloInput @join__type(graph: A) @oneOf"#);
                assert_build_schema_does_not_throw(result.public_sdl().unwrap());
                to_contain_graphql(result.public_sdl(), r#"input HelloInput @oneOf"#);
                to_contain_graphql(result.public_sdl(), r#"input GreetingsInput @oneOf"#);
            }
        }

        fn throws_during_composition_when_a_manual_one_of_definition_adds_a_conflicting_required_argument(api) {
            assert_throws(
                api.try_compose_services(&[
                    service("a", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.0"
                    import: ["@key"]
                  )

                input HelloInput @oneOf {
                  world: String
                  me: String
                }

                type Query {
                  hello(input: HelloInput): String
                }
              "#)),
                    service("b", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@composeDirective"]
                  )
                  @link(
                    url: "https://spec.graphql.org/specified/v1.6"
                    import: ["@oneOf"]
                  )
                  @composeDirective(name: "@oneOf")

                directive @oneOf(arg: String!) on INPUT_OBJECT | OBJECT

                type Hello @oneOf(arg: "Test") {
                  world: String
                }

                type Query {
                  greetings: [Hello]
                }
              "#)),
                ]),
                Some(
                    "The schema is not a valid GraphQL schema.. Caused by:\nArgument \"@oneOf(arg:)\" of type \"String!\" is required, but it was not provided.",
                ),
            );
        }

        fn composes_successfully_when_a_composed_manual_definition_widens_specified_directive_locations(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key"]
                )

              input HelloInput @oneOf {
                world: String
                me: String
              }

              directive @oneOf on INPUT_OBJECT

              type Query {
                hello(input: HelloInput): String
              }
            "#)),
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key", "@composeDirective"]
                )
                @link(
                  url: "https://spec.graphql.org/specified/v1.6"
                  import: ["@oneOf"]
                )
                @composeDirective(name: "@oneOf")

              # This redefines @oneOf but since it's a widening of the definition, it's allowed
              directive @oneOf on INPUT_OBJECT | OBJECT

              type Hello @oneOf {
                world: String
              }

              type Query {
                greetings: [Hello]
              }
            "#)),
            ]);

            assert!(result.errors().is_none());
            assert!(result.supergraph_sdl().is_some());

            if result.supergraph_sdl().is_some() {
                to_contain_graphql(result.supergraph_sdl(), r#"directive @oneOf on INPUT_OBJECT | OBJECT"#);
                assert_build_schema_does_not_throw(result.supergraph_sdl().unwrap());

                to_contain_graphql(result.public_sdl(), r#"directive @oneOf on INPUT_OBJECT | OBJECT"#);
                assert_build_schema_does_not_throw(result.public_sdl().unwrap());
            }
        }

        fn composition_fails_when_a_composed_directive_definitions_are_not_compatible_with_specified_one_of(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key"]
                )

              # This subgraph implicitly defined the default @oneOf directive definition

              input HelloInput @oneOf {
                world: String
                me: String
              }

              type Query {
                hello(input: HelloInput): String
              }
            "#)),
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key", "@composeDirective"]
                )
                @link(
                  url: "https://spec.graphql.org/specified/v1.6"
                  import: ["@oneOf"]
                )
                @composeDirective(name: "@oneOf")

              # This redefines @oneOf but since it's a widening of the definition, it's allowed
              directive @oneOf on OBJECT

              type Hello @oneOf {
                world: String
              }

              type Query {
                greetings: [Hello]
              }
            "#)),
            ]);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message_containing(r#"Directive "@oneOf" has no shared locations between subgraphs"#)]);
            assert!(result.supergraph_sdl().is_none());
        }

        fn composes_successfully_when_a_single_subgraph_widens_a_specified_directive_location(api) {
            let result = api.compose_services(&[
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key", "@composeDirective"]
                )
                @link(
                  url: "https://spec.graphql.org/specified/v1.6"
                  import: ["@oneOf"]
                )
                @composeDirective(name: "@oneOf")

              # This expands the @oneOf locations, but since no other subgraph is defined that uses it, this is allowed.
              directive @oneOf on INPUT_OBJECT | OBJECT

              type Hello @oneOf {
                world: String
              }

              type Query {
                greetings: [Hello]
              }
            "#)),
            ]);

            assert!(result.errors().is_none());
            assert!(result.supergraph_sdl().is_some());

            if result.supergraph_sdl().is_some() {
                // The composition engine's standard internal definition overrides the public schema output
                to_contain_graphql(result.supergraph_sdl(), r#"directive @oneOf on INPUT_OBJECT | OBJECT"#);
                to_contain_graphql(result.public_sdl(), r#"directive @oneOf on INPUT_OBJECT | OBJECT"#);

                assert_build_schema_does_not_throw(result.supergraph_sdl().unwrap());
                assert_build_schema_does_not_throw(result.public_sdl().unwrap());
            }
        }

        fn strips_custom_one_of_execution_locations_from_the_composed_sdl_if_it_is_not_explicitly_composed(api) {
            let result = api.compose_services(&[
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key"]
                )

              directive @oneOf on INPUT_OBJECT | OBJECT

              type Hello @oneOf {
                world: String
              }

              type Query {
                greetings: [Hello]
              }
            "#)),
            ]);

            assert!(result.errors().is_none());
            assert!(result.supergraph_sdl().is_some());
            if result.supergraph_sdl().is_some() {
                not_to_contain_graphql(result.supergraph_sdl(), r#"directive @oneOf on INPUT_OBJECT | OBJECT"#);
                not_to_contain_graphql(result.supergraph_sdl(), r#"type Hello @oneOf"#);
                assert_build_schema_does_not_throw(result.public_sdl().unwrap());
                // Ensure internal directive use on the OBJECT type doesn't leak to public graph
                not_to_contain_graphql(result.public_sdl(), r#"type Hello @oneOf"#);
            }
        }

        fn allows_an_uncomposed_one_of_matching_the_spec_to_surface_implicitly_on_public_input_objects(api) {
            let result = api.compose_services(&[
                service("b", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/v2.3"
                  import: ["@key"]
                )

              directive @oneOf on INPUT_OBJECT

              input GreetingsInput @oneOf {
                hello: String
                world: String
              }

              type Query {
                greetings(input: GreetingsInput!): String
              }
            "#)),
            ]);

            assert_composition_success(&result);
            if result.supergraph_sdl().is_some() {
                to_contain_graphql(result.supergraph_sdl(), r#"directive @oneOf on INPUT_OBJECT"#);
                to_contain_graphql(result.supergraph_sdl(), r#"input GreetingsInput @join__type(graph: B) @oneOf"#);
                to_contain_graphql(result.public_sdl(), r#"input GreetingsInput @oneOf"#);
            }
        }
    }
}
