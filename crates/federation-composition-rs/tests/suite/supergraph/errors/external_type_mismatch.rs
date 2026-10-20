//! Port of `__tests__/supergraph/errors/EXTERNAL_TYPE_MISMATCH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn external_type_mismatch(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              tags: [String!]
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external", "@provides"]
              )

            type Query {
              feed: [Article]
            }

            type Article @key(fields: "id") {
              id: ID!
              author: User! @provides(fields: "tags")
            }

            extend type User @key(fields: "id") {
              id: ID!
              tags: [String]! @external
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type of field "User.tags" is incompatible across subgraphs (where marked @external): it has type "[String!]" in subgraph "users" but type "[String]!" in subgraph "feed""#)
                    .code("EXTERNAL_TYPE_MISMATCH"),
            ],
        );

        assert_errors_contain(
            &api.compose_services(&[
                service("scores", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@requires"])

              type User @key(fields: "id") {
                id: ID!
                name: String! @external
                score: Int! @requires(fields: "name")
              }
          "#, version)),
                service("users", graphql!(r#"
              extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

              type User @key(fields: "id") {
                id: ID!
                name: String!
              }

              type Query {
                users: [User]
              }
          "#, version)),
                service("comments", graphql!(r#"
              extend schema
              @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@external", "@requires"]
                )
              
              type User @key(fields: "id") {
                id: ID!
                name: String @external
                comments: [String!]! @requires(fields: "name")
              }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type of field "User.name" is incompatible across subgraphs (where marked @external): it has type "String!" in subgraphs "scores" and "users" but type "String" in subgraph "comments""#)
                    .code("EXTERNAL_TYPE_MISMATCH"),
            ],
        );

        to_contain_graphql(
            ensure_composition_success(api.compose_services(&[
                service("b", graphql!(r#"
              type User @key(fields: "id") {
                id: ID!
              }

              extend type Component @key(fields: "context { id }") {
                context: Context! @external
              }

              type Context {
                id: ID!
              }

              type Query {
                users: [User]
              }
            "#)),
                service("a", graphql!(r#"
              extend type User @key(fields: "id") {
                id: ID @external
                age: Int
              }

              type Context {
                id: ID!
              }

              type Component @key(fields: "context { id }") {
                context: Context
                id: ID!
              }
            "#)),
                service("c", graphql!(r#"
              extend type User @key(fields: "id") {
                id: ID! @external
                name: String
              }
            "#)),
            ])).supergraph_sdl(),
            r#"
      type Component
        @join__type(graph: A, key: "context { id }")
        @join__type(graph: B, key: "context { id }") {
        context: Context
          @join__field(graph: A, type: "Context")
          @join__field(graph: B, type: "Context!")
        id: ID! @join__field(graph: A)
      }
    "#,
        );
    }

    fn external_type_mismatch_on_a_field_selected_by_requires(api, version) {
        // The @requires selection set is written against "pricing"'s VariantV2,
        // but the merged Product.variant points at VariantV1, which has no "sku".
        // Composition has to report the mismatch, not blow up while resolving the selection set.
        assert_errors_contain(
            &api.compose_services(&[
                service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              product: Product
            }

            type Product @key(fields: "id") {
              id: ID!
              variant: VariantV1!
            }

            type VariantV1 @key(fields: "id") {
              id: ID!
            }
          "#, version)),
                service("pricing", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external", "@requires"])

            type Product @key(fields: "id") {
              id: ID!
              variant: VariantV2 @external
              price: Float @requires(fields: "variant { sku }")
            }

            type VariantV2 @key(fields: "id") {
              id: ID!
              sku: String!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type of field "Product.variant" is incompatible across subgraphs (where marked @external): it has type "VariantV1!" in subgraph "products" but type "VariantV2" in subgraph "pricing""#)
                    .code("EXTERNAL_TYPE_MISMATCH"),
            ],
        );
    }
}
