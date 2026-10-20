//! Port of `__tests__/subgraph/errors/INTERFACE_KEY_NOT_ON_IMPLEMENTATION.spec.ts`.

use crate::shared::*;

test_versions! {
    fn interface_key_not_on_implementation(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User]
            }

            type RegisteredUser implements User {
              id: ID!
              name: String!
              email: String
            }

            interface User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }
          "#, version))]),
            &[if satisfies_version_range(">= v2.3", version) {
                ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Key @key(fields: "id") on interface type "User" is missing on implementation type "RegisteredUser"."#,
                    )
                    .code("INTERFACE_KEY_NOT_ON_IMPLEMENTATION")
            } else {
                ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot use @key on interface "User": @key is not yet supported on interfaces"#)
                    .code("KEY_UNSUPPORTED_ON_INTERFACE")
            }],
        );
    }

    fn no_interface_key_not_on_implementation_on_interfaces(api, version) {
        if satisfies_version_range("< v2.3", version) {
            return;
        }

        let result = api.compose_services(&[service("users", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type Query {
            users: [User]
          }

          type RegisteredUser implements User & Node @key(fields: "id") {
            id: ID!
            name: String!
            email: String
          }

          interface User implements Node {
            id: ID!
            name: String!
            email: String
          }

          interface Node @key(fields: "id") {
            id: ID!
          }
        "#, version))]);

        assert_composition_success(&result);
    }
}
