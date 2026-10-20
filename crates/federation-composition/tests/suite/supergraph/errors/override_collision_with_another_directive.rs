//! Port of `__tests__/supergraph/errors/OVERRIDE_COLLISION_WITH_ANOTHER_DIRECTIVE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn override_collision_with_another_directive(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("billing", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@override", "@external", "@provides"]
              )

            extend type Payment @key(fields: "id") {
              id: ID!
              amount: Int! @override(from: "payments") @external
            }

            type Invoice @key(fields: "id") {
              id: ID!
              amount: Int!
              payment: Payment @provides(fields: "amount")
            }
          "#, version)),
                service("payments", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              payments: [Payment]
            }

            type Payment @key(fields: "id") {
              id: ID!
              amount: Int!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"@override cannot be used on field "Payment.amount" on subgraph "billing" since "Payment.amount" on "billing" is marked with directive "@external""#)
                    .code("OVERRIDE_COLLISION_WITH_ANOTHER_DIRECTIVE"),
            ],
        );
    }
}
