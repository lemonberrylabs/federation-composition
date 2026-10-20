//! Port of `__tests__/subgraph/errors/KEY_UNSUPPORTED_ON_INTERFACE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn key_unsupported_on_interface(api, version) {
        let result = api.compose_services(&[service("users", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

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
                email: String
              }
            "#, version))]);
        if satisfies_version_range(">= v2.3", version) {
            // `expect.objectContaining({ supergraphSdl: expect.any(String) })`.
            assert!(result.supergraph_sdl().is_some(), "expected a supergraphSdl, received errors: {:?}", result.errors());
        } else {
            assert_errors_contain(
                &result,
                &[ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot use @key on interface "User": @key is not yet supported on interfaces"#)
                    .code("KEY_UNSUPPORTED_ON_INTERFACE")],
            );
        }
    }
}
