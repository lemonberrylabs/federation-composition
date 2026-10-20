//! Port of `__tests__/supergraph/errors/INTERFACE_OBJECT_USAGE_ERROR.spec.ts`.

use crate::shared::*;

test_versions! {
    fn interface_object_usage_error(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("book", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            interface Media @key(fields: "id") {
              id: ID!
              title: String!
            }

            type Book implements Media @key(fields: "id") {
              id: ID!
              title: String!
            }
          "#, version)),
                service("review", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@interfaceObject"]
              )

            type Media @interfaceObject {
              id: ID!
              reviews: [Review!]!
            }

            type Review {
              score: Int!
            }

            type Query {
              topRatedMedia: [Media!]!
            }
          "#, version)),
            ]),
            &[
                if satisfies_version_range(">= v2.3", version) {
                    ErrorMatcher::new()
                        .message_containing(r#"The @interfaceObject directive can only be applied to entity types but type "Media" has no @key in this subgraph."#)
                        .code("INTERFACE_OBJECT_USAGE_ERROR")
                } else {
                    ErrorMatcher::new()
                        .message_containing(r#"[review] Cannot import unknown element "@interfaceObject"."#)
                        .code("INVALID_LINK_DIRECTIVE_USAGE")
                },
            ],
        );
    }
}
