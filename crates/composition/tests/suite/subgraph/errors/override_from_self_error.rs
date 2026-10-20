//! Port of `__tests__/subgraph/errors/OVERRIDE_FROM_SELF_ERROR.spec.ts`.

use crate::shared::*;

test_versions! {
    fn override_from_self_error(api, version) {
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

            type Bill @key(fields: "id") {
              id: ID!
              amount: Int! @override(from: "billing")
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(r#"Source and destination subgraphs "billing" are the same for overridden field "Bill.amount""#)
                .code("OVERRIDE_FROM_SELF_ERROR")],
        );
    }
}
