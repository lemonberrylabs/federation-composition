//! Port of `__tests__/supergraph/errors/REQUIRED_INACCESSIBLE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn required_inaccessible(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )
            type Query {
              a(id: ID! @inaccessible): Int!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message(r#"Argument "Query.a(id:)" is @inaccessible but is a required argument of its field."#)
                    .code("REQUIRED_INACCESSIBLE"),
            ],
        );

        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )

            input A {
              id: ID! @inaccessible
              b: Int
            }
            type Query {
              a(a: A): Int!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message(r#"Input field "A.id" is @inaccessible but is a required input field of its type."#)
                    .code("REQUIRED_INACCESSIBLE"),
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

          type Query {
            a(id: ID! @inaccessible): Int!@inaccessible
            b: Int!
          }
        "#, version)),
            ])
                .errors()
                .is_none()
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@inaccessible"]
            )

          input A @inaccessible {
            a: Int! @inaccessible
          }

          type Query {
            a(id: A! @inaccessible): Int! @inaccessible
            b: Int!
          }
        "#, version)),
            ])
                .errors()
                .is_none()
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@inaccessible"]
            )

          type Query {
            a(id: String! = "brrry" @inaccessible): Int!
          }
        "#, version)),
            ])
                .errors()
                .is_none()
        );

        assert!(
            api.compose_services(&[
                service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@inaccessible"]
            )

          input Foo {
            a: String
            b: String! = "he he he" @inaccessible
          }

          type Query {
            a(id: Foo!): Int!
            b: Int!
          }
        "#, version)),
            ])
                .errors()
                .is_none()
        );
    }
}
