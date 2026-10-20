//! Port of `__tests__/subgraph/errors/KEY_FIELDS_SELECT_INVALID_TYPE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn key_fields_select_invalid_type(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("serviceA", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "featuredItem { id }") {
              featuredItem: Node!
              sku: String!
            }

            interface Node {
              id: ID!
            }
          "#, version)),
                service("serviceB", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@external", "@requires"])

            extend type Product {
              sku: String! @external
              price: Int! @requires(fields: "sku")
            }
          "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[serviceA] On type "Product", for @key(fields: "featuredItem { id }"): field "Product.featuredItem" is a Interface type which is not allowed in @key"#,
                )
                .code("KEY_FIELDS_SELECT_INVALID_TYPE")],
        );
    }
}
