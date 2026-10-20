//! Port of `__tests__/supergraph/errors/IMPLEMENTED_BY_INACCESSIBLE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn implemented_by_inaccessible(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@inaccessible"]
              )

            interface Shared {
              sharedField: String!
            }

            type A implements Shared {
              sharedField: String! @inaccessible
              b: String!
            }

            type B implements Shared {
              sharedField: String!
              b: String!
            }

            type Query {
              a: Shared
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Field "A.sharedField" is @inaccessible but implements the interface field "Shared.sharedField", which is in the API schema."#)
                    .code("IMPLEMENTED_BY_INACCESSIBLE"),
            ],
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@inaccessible"]
              )

            interface Shared {
              sharedField: String!
              inaccessibleField: String! @inaccessible
            }

            type A implements Shared {
              sharedField: String!
              inaccessibleField: String! @inaccessible
            }

            type B implements Shared {
              sharedField: String!
              inaccessibleField: String! @inaccessible
            }

            type Query {
              a: Shared
            }
          "#, version)),
            ])
                .supergraph_sdl()
                .is_some()
        );

        assert!(
            api.compose_services(&[
                service("node", graphql!(r#"
            schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@tag"]
              ) {
              query: Query
            }

            type Query {
              b(id: ID! @federation__inaccessible): B @federation__inaccessible
              a(id: ID!): A
            }

            type B implements Node @federation__inaccessible {
              id: ID! @federation__inaccessible
            }

            type A implements Node {
              id: ID!
            }

            interface Node {
              id: ID!
            }
          "#)),
            ])
                .supergraph_sdl()
                .is_some()
        );
    }
}
