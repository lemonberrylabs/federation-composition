//! Port of `__tests__/supergraph/errors/OVERRIDE_SOURCE_HAS_OVERRIDE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn override_source_has_override(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("billing", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@override"]
                  )
                type Query {
                  bills: [Bill]
                }
                type Bill @key(fields: "id") {
                  id: ID!
                  amount: Int! @override(from: "payments")
                }
              "#, version)),
                service("payments", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@override"]
                  )
                type Query {
                  payments: [Bill]
                }
                type Bill @key(fields: "id") {
                  id: ID!
                  amount: Int @override(from: "billing")
                }
              "#, version)),
                service("invoices", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@override"]
                  )
                type Query {
                  invoices: [Bill]
                }
                type Bill @key(fields: "id") {
                  id: ID!
                  amount: Int @override(from: "billing")
                }
              "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Field "Bill.amount" on subgraph "billing" is also marked with directive @override in subgraph "payments". Only one @override directive is allowed per field."#)
                    .code("OVERRIDE_SOURCE_HAS_OVERRIDE"),
                ErrorMatcher::new()
                    .message_containing(r#"Field "Bill.amount" on subgraph "invoices" is also marked with directive @override in subgraph "billing". Only one @override directive is allowed per field."#)
                    .code("OVERRIDE_SOURCE_HAS_OVERRIDE"),
                ErrorMatcher::new()
                    .message_containing(r#"Field "Bill.amount" on subgraph "payments" is also marked with directive @override in subgraph "billing". Only one @override directive is allowed per field."#)
                    .code("OVERRIDE_SOURCE_HAS_OVERRIDE"),
            ],
        );

        assert_composition_success(&api.compose_services(&[
            service("bar", graphql!(r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/${version}"
            import: ["@shareable", "@override"]
          )

          type Query {
            user: String @shareable @override(from: "mono")
          }
      "#, version)),
            service("foo", graphql!(r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/${version}"
            import: ["@shareable", "@override"]
          )

        type Query {
          user: String @shareable @override(from: "mono")
        }
      "#, version)),
            service("main", graphql!(r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/${version}"
            import: ["@shareable"]
          )

          type Query {
            user: String @shareable
          }
      "#, version)),
        ]));
    }
}
