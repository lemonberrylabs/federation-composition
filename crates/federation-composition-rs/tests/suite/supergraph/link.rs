//! Port of `__tests__/supergraph/link.spec.ts`.

use crate::shared::*;

/// `expect.objectContaining({ message: '[<subgraph>] Cannot import unknown element "@composeDirective".',
/// extensions: expect.objectContaining({ code: "INVALID_LINK_DIRECTIVE_USAGE" }) })`.
fn unknown_compose_directive(subgraph: &str) -> ErrorMatcher {
    ErrorMatcher::new()
        .message(format!(r#"[{subgraph}] Cannot import unknown element "@composeDirective"."#))
        .code("INVALID_LINK_DIRECTIVE_USAGE")
}

test_versions! {
    fn error_if_directive_is_defined_without_overlapping_locations_across_subgraphs_and_all_included_in_compose_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            type User @key(fields: "id") {
              id: ID!
              name: String! @lowercase
            }

            type Query {
              users: [User]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on OBJECT

            extend type User @key(fields: "id") {
              id: ID!
              comments: [Comment]
            }

            type Comment @lowercase {
              id: ID!
              text: String!
            }

            type Query {
              comments: [Comment]
            }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        if version == "v2.0" {
            assert_errors_contain(result.errors(), &[unknown_compose_directive("a")]);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("b")]);
        } else {
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new().message(r#"Directive "@lowercase" has no shared locations between subgraphs"#)],
            );
        }
    }

    fn error_if_two_different_major_versions_of_the_same_spec_were_used_to_import_the_same_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/lowercase/v1.1", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            type User @key(fields: "id") {
              id: ID!
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/lowercase/v2.3", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on OBJECT | FIELD_DEFINITION

            extend type User @key(fields: "id") @lowercase {
              id: ID!
              comments: [Comment]
            }

            type Comment @lowercase {
              id: ID!
              text: String!
            }

            type Query {
              comments: [Comment]
            }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        if version == "v2.0" {
            assert_errors_contain(result.errors(), &[unknown_compose_directive("a")]);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("b")]);
        } else {
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"Core feature "https://myspecs.dev/lowercase" requested to be merged has major version mismatch across subgraphs"#)
                    .code("DIRECTIVE_COMPOSITION_ERROR")],
            );
        }
    }

    fn directive_composition_error_the_same_composed_directive_used_under_different_names_alias(api, version) {
        let compose = || {
            api.compose_services(&[
                service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
            @link(url: "https://myspecs.dev/lowercase/v1.0", import: ["@lowercase"])
            @composeDirective(name: "@lowercase")

          directive @lowercase on FIELD_DEFINITION

          type User @key(fields: "id") {
            id: ID!
            name: String! @lowercase
          }
        "#, version)),
                service("b", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
            @link(url: "https://myspecs.dev/lowercase/v1.0", import: [{name: "@lowercase", as: "@lower"}])
            @composeDirective(name: "@lower")

          directive @lower on OBJECT | FIELD_DEFINITION

          extend type User @key(fields: "id") {
            id: ID!
            comments: [Comment]
          }

          type Comment @lower {
            id: ID!
            text: String!
          }

          type Query {
            comments: [Comment]
          }
        "#, version)),
                service("c", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
            @link(url: "https://myspecs.dev/lowercase/v1.0", import: [{name: "@lowercase", as: "@lower_case"}])
            @composeDirective(name: "@lower_case")

          directive @lower_case on FIELD_DEFINITION

          extend type User @key(fields: "id") {
            id: ID!
            reviews: [Review]
          }

          type Review {
            id: ID!
            text: String! @lower_case
          }

          type Query {
            reviews: [Review]
          }
        "#, version)),
            ])
        };

        if version == "v2.0" {
            let result = compose();
            assert_composition_failure(&result);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("a")]);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("b")]);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("c")]);
        } else {
            let result = compose();
            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"Composed directive "@lowercase" has incompatible name across subgraphs: it has name "@lowercase" in subgraph "a" but name "@lower" in subgraph "b" and name "@lower_case" in subgraph "c". Composed directive must have the same name across all subgraphs."#)
                    .code("DIRECTIVE_COMPOSITION_ERROR")],
            );
        }
    }

    fn error_if_two_different_major_versions_of_the_same_spec_were_used_to_import_two_different_directives(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/casing/v1.1", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            type User @key(fields: "id") {
              id: ID!
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/casing/v2.3", import: ["@uppercase"])
              @composeDirective(name: "@uppercase")

            directive @uppercase on FIELD_DEFINITION

            extend type User @key(fields: "id") {
              id: ID!
              comments: [Comment]
            }

            type Comment {
              id: ID!
              text: String! @uppercase
            }

            type Query {
              comments: [Comment]
            }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        if version == "v2.0" {
            assert_errors_contain(result.errors(), &[unknown_compose_directive("a")]);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("b")]);
        } else {
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"Core feature "https://myspecs.dev/casing" requested to be merged has major version mismatch across subgraphs"#)
                    .code("DIRECTIVE_COMPOSITION_ERROR")],
            );
        }
    }

    fn error_if_two_different_major_versions_of_the_same_spec_were_used_to_import_two_different_kind_of_definitions(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/casing/v1.1", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            type User @key(fields: "id") {
              id: ID!
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])
              @link(url: "https://myspecs.dev/casing/v2.3", import: ["UppercaseString"])

            scalar UppercaseString

            extend type User @key(fields: "id") {
              id: ID!
              comments: [Comment]
            }

            type Comment {
              id: ID!
              text: UppercaseString!
            }

            type Query {
              comments: [Comment]
            }
          "#, version)),
        ]);

        if version == "v2.0" {
            assert_composition_failure(&result);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("a")]);
        } else {
            assert_composition_success(&result);
        }
    }

    fn error_if_two_different_specs_were_used_to_import_the_same_directive(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/casing/v1.1", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            type User @key(fields: "id") {
              id: ID!
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/lower/v2.3", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            extend type User @key(fields: "id") {
              id: ID!
              comments: [Comment]
            }

            type Comment {
              id: ID!
              text: String! @lowercase
            }

            type Query {
              comments: [Comment]
            }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        if version == "v2.0" {
            assert_errors_contain(result.errors(), &[unknown_compose_directive("a")]);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("b")]);
        } else {
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"Composed directive "@lowercase" is not linked by the same core feature in every subgraph"#)
                    .code("DIRECTIVE_COMPOSITION_ERROR")],
            );
        }
    }

    fn error_if_two_different_specs_were_used_to_import_the_same_directive_alias(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/casing/v1.1", import: ["@lowercase"])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            type User @key(fields: "id") {
              id: ID!
              name: String! @lowercase
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@composeDirective"])
              @link(url: "https://myspecs.dev/lower/v2.3", import: [{name: "@lower", as: "@lowercase"}])
              @composeDirective(name: "@lowercase")

            directive @lowercase on FIELD_DEFINITION

            extend type User @key(fields: "id") {
              id: ID!
              comments: [Comment]
            }

            type Comment {
              id: ID!
              text: String! @lowercase
            }

            type Query {
              comments: [Comment]
            }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        if version == "v2.0" {
            assert_errors_contain(result.errors(), &[unknown_compose_directive("a")]);
            assert_errors_contain(result.errors(), &[unknown_compose_directive("b")]);
        } else {
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"Composed directive "@lowercase" is not linked by the same core feature in every subgraph"#)
                    .code("DIRECTIVE_COMPOSITION_ERROR")],
            );
        }
    }
}
