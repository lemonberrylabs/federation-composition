//! Port of `__tests__/subgraph/errors/KEY_INVALID_FIELDS.spec.ts`.

use crate::shared::*;

test_versions! {
    fn key_invalid_fields_syntax_error(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("serviceA", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "featuredItem { id") {
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
                    r#"[serviceA] On type "Product", for @key(fields: "featuredItem { id"): Syntax Error: Expected Name, found <EOF>."#,
                )
                .code("KEY_INVALID_FIELDS")],
        );
    }

    fn key_invalid_fields_unknown_directive(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "id name @lowercase ") {
              id: ID!
              name: String
            }

            extend type Query {
              user(id: ID!): User
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(r#"[users] On type "User", for @key(fields: "id name @lowercase "): Unknown directive "@lowercase""#)
                .code("KEY_INVALID_FIELDS")],
        );
    }

    fn key_invalid_fields_unknown_field(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("serviceA", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Product @key(fields: "skucha") {
              sku: String!
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[serviceA] On type "Product", for @key(fields: "skucha"): Cannot query field "skucha" on type "Product" (the field should either be added to this subgraph or, if it should not be resolved by this subgraph, you need to add it to this subgraph with @external)."#,
                )
                .code("KEY_INVALID_FIELDS")],
        );
    }
}
