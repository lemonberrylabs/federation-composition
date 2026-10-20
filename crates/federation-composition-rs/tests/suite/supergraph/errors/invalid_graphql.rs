//! Port of `__tests__/supergraph/errors/INVALID-GRAPHQL.spec.ts`.

use crate::shared::*;

test_versions! {
    fn interface_field_object_field_subtype(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.6"
              import: ["@shareable"]
            )

          type Query {
            a: Admin
          }

          type Admin implements Node {
            id: ID! @shareable
          }

          type User implements Node {
            id: ID!
          }

          interface Node {
            id: ID!
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.6"
              import: ["@key"]
            )

          type Query {
            b: Admin
          }

          type Admin @key(fields: "id") {
            id: ID
            name: String!
          }
        "#)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new()
                .message("Interface field Node.id expects type ID! but Admin.id of type ID is not a proper subtype.")
                .code("INVALID_GRAPHQL")],
        );

        assert_composition_success(&api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.6"
                import: ["@shareable"]
              )

            type Query {
              a: Admin
            }

            type Admin implements Node {
              id: ID! @shareable
              parent: Node
            }

            type User implements Node {
              id: ID!
              parent: User
            }

            interface Node {
              id: ID!
              parent: Node
            }
          "#)),
        ]));
    }
}
