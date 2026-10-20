//! Port of `__tests__/supergraph/errors/OVERRIDE_COLLISION_WITH_ANOTHER_DIRECTIVE_PROGRESSIVE_REQUIRES.spec.ts`.

use crate::shared::*;

test_versions! {
    fn override_collision_with_another_directive_for_progressive_override_with_requires(api, version) {
        // Verifies progressive override (`@override` with `label`) is forbidden
        // when the overridden source field uses `@requires`.
        if satisfies_version_range("< v2.7", version) {
            return;
        }

        let result = api.compose_services(&[
            service("original", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@requires", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @external
            paymentReceipt: String! @shareable @requires(fields: "creditCardNumber")
          }

          type Query {
            product(id: ID!): Product
          }
        "#, version)),
            service("new", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@override", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @shareable
            paymentReceipt: String! @shareable @override(from: "original", label: "public")
          }
        "#, version)),
        ]);

        assert_errors_contain(
            &result,
            &[
                ErrorMatcher::new()
                    .message(r#"@override with label (progressive override) cannot be used on field "Product.paymentReceipt" on subgraph "new" since "Product.paymentReceipt" on "original" is marked with directive "@requires". Use non-progressive @override (without label) for this migration."#)
                    .code("OVERRIDE_COLLISION_WITH_ANOTHER_DIRECTIVE"),
            ],
        );
    }
}
