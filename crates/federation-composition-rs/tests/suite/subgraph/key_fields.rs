//! Port of `__tests__/subgraph/key-fields.spec.ts`.

use crate::shared::*;

test_versions! {
    fn typename_allowed_in_provides(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

          type Query {
            users: [User]
          }

          type User @key(fields: "id") {
            id: ID!
            profile: Profile! @shareable
          }

          type Profile {
            id: ID!  @shareable
            name: String!
          }
        "#, version)),
            service("reviews", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@provides", "@external"]
            )

          type Query {
            reviews: [Review]
          }

          type Review @key(fields: "id") {
            id: ID!
            title: String!
            author: User! @provides(fields: "profile { id __typename }")
          }

          extend type User @key(fields: "id") {
            id: ID! @external
            profile: Profile! @external
          }

          extend type Profile {
            id: ID! @external
          }
        "#, version)),
        ]);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Profile @join__type(graph: REVIEWS) @join__type(graph: USERS) {
        id: ID!
          @join__field(graph: REVIEWS, external: true)
          @join__field(graph: USERS)
        name: String! @join__field(graph: USERS)
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Review @join__type(graph: REVIEWS, key: "id") {
        id: ID!
        title: String!
        author: User!
          @join__field(graph: REVIEWS, provides: "profile { id __typename }")
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: REVIEWS, key: "id", extension: true)
        @join__type(graph: USERS, key: "id") {
        id: ID!
        profile: Profile!
          @join__field(graph: REVIEWS, external: true)
          @join__field(graph: USERS)
      }
    "#);
    }

    fn typename_allowed_in_requires(api, version) {
        assert_no_error_matches(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              profile: Profile!
            }

            type Profile {
              id: ID!
              name: String!
            }
          "#, version)),
                service("reviews", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@requires", "@external"]
              )

            type Query {
              reviews: [Review]
            }

            type Review @key(fields: "id") {
              id: ID!
              title: String!
              author: User!
            }

            extend type User @key(fields: "id") {
              id: ID! @external
              profile: Profile! @external
              reviews: [Review] @requires(fields: "profile { id __typename }")
            }

            extend type Profile {
              id: ID! @external
            }
          "#, version)),
            ]),
            &ErrorMatcher::new().message_containing("__typename"),
        );
    }

    fn fragments_in_requires(api, version) {
        let stars_stuff = create_stars_stuff();

        assert_composition_success(&api.compose_services(&[
            service("inventory", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@shareable", "@external", "@requires"]
              )

            type Product implements ProductItf @key(fields: "id") {
              id: ID!
              dimensions: ProductDimension @external
              delivery(zip: String): DeliveryEstimates
                @requires(fields: "dimensions { ... on ProductDimension { size weight } }")
            }

            type ProductDimension @shareable {
              size: String
              weight: Float
            }

            type DeliveryEstimates {
              estimatedDelivery: String
              fastestDelivery: String
            }

            interface ProductItf {
              id: ID!
              dimensions: ProductDimension
              delivery(zip: String): DeliveryEstimates
            }

            enum ShippingClass {
              STANDARD
              EXPRESS
              OVERNIGHT
            }
          "#, version)),
            stars_stuff.pandas,
            stars_stuff.products,
            stars_stuff.reviews,
            stars_stuff.users,
        ]));
    }

    fn missing_field_set_definition(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("foo", graphql!(r#"
            directive @key(fields: _FieldSet!) repeatable on OBJECT | INTERFACE

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              name: String!
            }
          "#))]),
            &[ErrorMatcher::new()
                .message_containing("[foo] Unknown type _FieldSet")
                .code("INVALID_GRAPHQL")],
        );
    }
}
