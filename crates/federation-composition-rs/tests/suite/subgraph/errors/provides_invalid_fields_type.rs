//! Port of `__tests__/subgraph/errors/PROVIDES_INVALID_FIELDS_TYPE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn provides_invalid_fields_type_boolean(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("billing", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@provides"]
              )

            type Payment @key(fields: "id") {
              id: ID!
              amount: Int!
            }

            type Invoice @key(fields: "id") {
              id: ID!
              amount: Int!
              payment: Payment @provides(fields: true)
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[billing] On field "Invoice.payment", for @provides(fields: true): Invalid value for argument "fields": must be a string."#,
                )
                .code("PROVIDES_INVALID_FIELDS_TYPE")],
        );
    }
}
