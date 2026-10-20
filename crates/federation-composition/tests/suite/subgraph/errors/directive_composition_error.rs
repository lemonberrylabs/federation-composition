//! Port of `__tests__/subgraph/errors/DIRECTIVE_COMPOSITION_ERROR.spec.ts`.

use crate::shared::*;

test_versions! {
    fn directive_composition_error(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@composeDirective"]
              )
              @link(url: "https://myspecs.dev/hello/v1.0", import: ["@hello"])
              @composeDirective(name: "@hello")

            type Query {
              words: [String!]!
            }
          "#, version))]),
            &[if version == "v2.0" {
                // We don't want to follow Apollo's example here as returning the INVALID_LINK_DIRECTIVE_USAGE error makes more sense.
                ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot import unknown element "@composeDirective"."#)
                    .code("INVALID_LINK_DIRECTIVE_USAGE")
            } else {
                ErrorMatcher::new()
                    .message_containing(
                        r#"Could not find matching directive definition for argument to @composeDirective "@hello" in subgraph "users"."#,
                    )
                    .code("DIRECTIVE_COMPOSITION_ERROR")
            }],
        );

        assert_errors_contain(
            api.compose_services(&[service("foo", graphql!(r#"
          directive @composeDirective(name: String!) repeatable on SCHEMA
          directive @block on FIELD_DEFINITION
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@composeDirective"]) @composeDirective(name: "@block")

         type Query {
          foo: String @block
         }
        "#, version))]),
            &[if version == "v2.0" {
                // `expect.any(Object)`: any error.
                ErrorMatcher::new()
            } else {
                ErrorMatcher::new()
                    .message_containing(
                        r#"Directive "@block" in subgraph "foo" cannot be composed because it is not a member of a core feature"#,
                    )
                    .code("DIRECTIVE_COMPOSITION_ERROR")
            }],
        );
    }
}
