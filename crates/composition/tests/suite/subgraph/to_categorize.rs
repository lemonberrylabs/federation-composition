//! Port of `__tests__/subgraph/to-categorize.spec.ts`.

use crate::shared::*;

test_versions! {
    fn valid_composed_directive_gets_an_empty_object_instead_of_name(api, version) {
        let result = api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@composeDirective"]
              )
              @link(url: "https://myspecs.dev/hello/v1.0", import: ["@hello", "Person"])
              @composeDirective(name: "@hello")

            directive @hello(person: Person) on FIELD_DEFINITION

            input Person {
              name: String!
            }

            type Query {
              words: [String!]! @hello(person: {})
            }
          "#, version))]);

        if version == "v2.0" {
            assert_errors_contain(
                &result,
                // We don't want to follow Apollo's example here as returning the INVALID_LINK_DIRECTIVE_USAGE error makes more sense.
                &[ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot import unknown element "@composeDirective"."#)
                    .code("INVALID_LINK_DIRECTIVE_USAGE")],
            );
        } else {
            assert_composition_success(&result);
        }
    }
}
