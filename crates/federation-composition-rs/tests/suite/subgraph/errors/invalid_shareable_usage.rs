//! Port of `__tests__/subgraph/errors/INVALID_SHAREABLE_USAGE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn invalid_shareable_usage(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

            type User implements Node {
              id: ID!
              name: String
            }

            interface Node {
              id: ID! @shareable
            }

            type Query {
              user: User
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] Invalid use of @shareable on field "Node.id": only object type fields can be marked with @shareable"#,
                )
                .code("INVALID_SHAREABLE_USAGE")],
        );
    }
}
