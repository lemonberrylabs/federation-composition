//! Port of `__tests__/subgraph/errors/INVALID_SUBGRAPH_NAME.spec.ts`.

use crate::shared::*;

test_versions! {
    fn invalid_subgraph_name(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("_", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(r#"[_] Invalid name _ for a subgraph: this name is reserved"#)
                .code("INVALID_SUBGRAPH_NAME")],
        );
    }
}
