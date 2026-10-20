//! Port of `__tests__/subgraph/errors/REQUIRES_DIRECTIVE_IN_FIELDS_ARG.spec.ts`.

use crate::shared::*;

test_versions! {
    fn requires_directive_in_fields_arg(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@composeDirective", "@requires"]
              )
              @link(url: "https://myspecs.dev/testkit/v1.0", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            type User @key(fields: "id name") {
              id: ID!
              name: String
              profile: Profile @requires(fields: "name @lowercase")
            }

            type Profile {
              id: ID!
              name: String
            }

            extend type Query {
              user(id: ID!): User
            }
          "#, version))]),
            &[if version == "v2.0" {
                ErrorMatcher::new()
                    .message(r#"[users] Cannot import unknown element "@composeDirective"."#)
                    .code("INVALID_LINK_DIRECTIVE_USAGE")
            } else {
                ErrorMatcher::new()
                    .message(
                        r#"[users] On field "User.profile", for @requires(fields: "name @lowercase"): cannot have directive applications in the @requires(fields:) argument but found @lowercase."#,
                    )
                    .code("REQUIRES_DIRECTIVE_IN_FIELDS_ARG")
            }],
        );
    }
}
