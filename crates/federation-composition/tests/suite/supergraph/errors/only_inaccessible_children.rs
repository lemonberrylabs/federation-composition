//! Port of `__tests__/supergraph/errors/ONLY_INACCESSIBLE_CHILDREN.spec.ts`.

use crate::shared::*;

test_versions! {
    fn only_inaccessible_children(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible", "@requires"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id internalId") {
              id: ID!
              internalId: ID! @inaccessible
            }

            type Profile {
              name: String @inaccessible
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type "Profile" is in the API schema but all of its fields are @inaccessible."#)
                    .code("ONLY_INACCESSIBLE_CHILDREN"),
            ],
        );

        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              profile: Profile! @inaccessible
            }

            type Profile @inaccessible {
              name: String @inaccessible
              details: Details
              address: Address @inaccessible
            }

            type Details @inaccessible {
              age: Int
            }

            """
            Address has all its fields inaccessible, but it's also referenced by a field that is not inaccessible as well.
            It is never in the API schema, except as part of introspection, but it's still reported as ONLY_INACCESSIBLE_CHILDREN error...
            """
            type Address {
              street: String @inaccessible
              postalCode: String @inaccessible
            }
          "#, version)),
                service("products", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type Query {
              products: [Product]
            }

            type Product @key(fields: "id") {
              id: ID!
              name: String!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type "Address" is in the API schema but all of its fields are @inaccessible."#)
                    .code("ONLY_INACCESSIBLE_CHILDREN"),
            ],
        );

        assert_composition_success(&api.compose_services(&[
            service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible", "@requires"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id internalId") {
              id: ID!
              internalId: ID! @inaccessible
            }

            type Profile @inaccessible {
              name: String
            }
          "#, version)),
        ]));

        assert_composition_success(&api.compose_services(&[
            service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              profile: Profile! @inaccessible
            }

            type Profile @inaccessible {
              name: String @inaccessible
              details: Details
              address: Address @inaccessible
            }

            type Details @inaccessible {
              age: Int
            }

            type Address @inaccessible {
              street: String @inaccessible
              postalCode: String @inaccessible
            }
          "#, version)),
            service("products", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type Query {
              products: [Product]
            }

            type Product @key(fields: "id") {
              id: ID!
              name: String!
            }
          "#, version)),
        ]));

        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@inaccessible"]
              )

            type Query {
              users: [User]
            }

            interface Node @key(fields: "id") {
              id: ID @inaccessible
            }

            type User implements Node @key(fields: "id") {
              id: ID!
            }
          "#)),
                service("products", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type Node @key(fields: "id", resolvable: false) @interfaceObject {
              id: ID
            }
          "#)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type "Node" is in the API schema but all of its fields are @inaccessible."#)
                    .code("ONLY_INACCESSIBLE_CHILDREN"),
            ],
        );
    }
}
