//! Port of `__tests__/subgraph/errors/PROVIDES_UNSUPPORTED_ON_INTERFACE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn provides_unsupported_on_interface(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@provides"]
              )

            type Query {
              users: [User]
            }

            type RegisteredUser implements User @key(fields: "id") {
              id: ID!
              name: String!
              profile: Profile
            }

            interface User @key(fields: "id") {
              id: ID!
              name: String!
              profile: Profile @provides(fields: "name")
            }

            type Profile {
              name: String
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] Cannot use @provides on field "User.profile" of parent type "User": @provides is not yet supported within interfaces"#,
                )
                .code("PROVIDES_UNSUPPORTED_ON_INTERFACE")],
        );
    }
}
