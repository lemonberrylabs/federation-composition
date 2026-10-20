//! Port of `__tests__/subgraph/federation-directives.spec.ts`.

use crate::shared::*;

test_versions! {
    fn federation_v1_directives_should_be_available_when_schema_link_federation_is_not_provided(api, version) {
        let result = api.compose_services(&[service("a", graphql!(r#"
          type User @key(fields: "name") {
            name: String
          }

          type Query {
            user: User
          }
        "#))]);

        assert_composition_success(&result);
    }

    fn federation_v1_directives_prefixed_with_federation_should_not_be_available_when_schema_link_federation_is_not_provided(api, version) {
        let result = api.compose_services(&[service("a", graphql!(r#"
          type User @federation__key(fields: "name") {
            name: String
          }

          type Query {
            user: User
          }
        "#))]);

        assert_composition_failure(&result);

        assert_errors_contain(
            &result,
            &[ErrorMatcher::new().message_containing(r#"Unknown directive "@federation__key""#)],
        );
    }

    fn federation_directives_should_not_be_available_when_not_imported_but_schema_link_federation_is_provided(api, version) {
        let result = api.compose_services(&[service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

          type User @key(fields: "name") {
            name: String
          }

          type Query {
            user: User
          }
        "#, version))]);

        assert_errors_contain(
            &result,
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[a] Unknown directive "@key". If you meant the "@key" federation directive, you should use fully-qualified name "@federation__key" or add "@key" to the `import` argument of the @link to the federation specification."#,
                )
                .code("INVALID_GRAPHQL")],
        );
    }

    fn federation_directives_prefixed_with_federation_should_be_available_when_not_imported_but_schema_link_federation_is_provided(api, version) {
        let result = api.compose_services(&[service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

          type User @federation__key(fields: "name") {
            name: String
          }

          type Query {
            user: User
          }
        "#, version))]);
        assert_composition_success(&result);
    }

    fn support_basic_aliasing(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: [{ name: "@key", as: "@myKey" }]
            )

          type Query {
            users: [User]
          }

          type User @myKey(fields: "id") {
            id: ID!
            name: String!
            age: String!
          }
        "#, version)),
            service("reviews", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", { name: "@external", as: "@myExternal" }]
            )

          type Query {
            reviews: [Review]
          }

          type Review @key(fields: "id") {
            id: ID!
            title: String!
          }

          extend type User @key(fields: "id") {
            id: ID! @myExternal
            reviews: [Review!]!
          }
        "#, version)),
        ]);

        assert_composition_success(&result);
    }
}
