//! Port of `__tests__/subgraph/errors/MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL.spec.ts`.

use crate::shared::*;

test_versions! {
    fn merged_directive_application_on_external(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible", "@requires", "@provides", "@external", "@tag"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              internalId: ID! @inaccessible @external @tag(name: "public")
              profile: Profile @requires(fields: "internalId")
            }

            type Profile {
              name: String
            }
          "#, version)),
                service("ext", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible", "@external"]
              )

            type Query {
              users: [User]
            }

            extend type User @key(fields: "id") {
              id: ID!
              internalId: ID! @inaccessible
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot apply merged directive @inaccessible to external field "User.internalId""#)
                    .code("MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL"),
                ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot apply merged directive @tag(name: "public") to external field "User.internalId""#)
                    .code("MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL"),
            ],
        );
    }

    fn fed_v1_has_no_merged_directive_application_on_external(api, version) {
        assert_composition_success(&api.compose_services(&[
            service("inventory", graphql!(r#"
            directive @tag(name: String!) repeatable on FIELD_DEFINITION

            extend type Product @key(fields: "id") {
              id: ID! @external @tag(name: "public")
              dimensions: ProductDimension @external
              delivery(zip: String): DeliveryEstimates
                @requires(fields: "dimensions { size weight }")
            }

            type ProductDimension {
              size: String
              weight: Float @tag(name: "public")
            }

            type DeliveryEstimates {
              estimatedDelivery: String
            }
          "#)),
            service("ext", graphql!(r#"
            directive @tag(name: String!) repeatable on FIELD_DEFINITION

            type Product @key(fields: "id") @key(fields: "sku package") {
              id: ID!
              sku: String
              package: String
              dimensions: ProductDimension
            }

            type ProductDimension {
              size: String
              weight: Float
            }

            type Query {
              allProducts: [Product]
              product(id: ID!): Product
            }
          "#)),
        ]));
    }
}
