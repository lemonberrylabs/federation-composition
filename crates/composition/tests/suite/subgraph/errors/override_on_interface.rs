//! Port of `__tests__/subgraph/errors/OVERRIDE_ON_INTERFACE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn override_on_interface(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("billing", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@override"]
              )

            type Query {
              bills: [Bill]
            }

            interface Bill @key(fields: "id") {
              id: ID!
              amount: Int! @override(from: "billing")
            }
          "#, version))]),
            &[if satisfies_version_range(">= v2.3", version) {
                ErrorMatcher::new()
                    .message_containing(
                        r#"@override cannot be used on field "Bill.amount" on subgraph "billing": @override is not supported on interface type fields."#,
                    )
                    .code("OVERRIDE_ON_INTERFACE")
            } else {
                ErrorMatcher::new()
                    .message_containing(r#"[billing] Cannot use @key on interface "Bill": @key is not yet supported on interfaces"#)
                    .code("KEY_UNSUPPORTED_ON_INTERFACE")
            }],
        );
    }
}
