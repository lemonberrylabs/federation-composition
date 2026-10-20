//! Port of `__tests__/subgraph/errors/REQUIRES_UNSUPPORTED_ON_INTERFACE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn requires_unsupported_on_interface(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@requires"]
              )

            type Query {
              users: [User]
            }

            type RegisteredUser implements User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }

            interface User @key(fields: "id") {
              id: ID!
              name: String!
              email: String @requires(fields: "name")
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] Cannot use @requires on field "User.email" of parent type "User": @requires is not yet supported within interfaces"#,
                )
                .code("REQUIRES_UNSUPPORTED_ON_INTERFACE")],
        );
    }
}
