//! Port of `__tests__/supergraph/errors/REFERENCED_INACCESSIBLE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn referenced_inaccessible(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )

            type Query {
              users(filter: Filter, access: Access): [User]
            }

            enum Access @inaccessible {
              PUBLIC
              PRIVATE
            }

            type User @inaccessible {
              id: ID!
            }

            input Filter @inaccessible {
              limit: Int
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type "User" is @inaccessible but is referenced by "Query.users", which is in the API schema."#)
                    .code("REFERENCED_INACCESSIBLE"),
                ErrorMatcher::new()
                    .message_containing(r#"Type "Filter" is @inaccessible but is referenced by "Query.users(filter:)", which is in the API schema."#)
                    .code("REFERENCED_INACCESSIBLE"),
                ErrorMatcher::new()
                    .message_containing(r#"Type "Access" is @inaccessible but is referenced by "Query.users(access:)", which is in the API schema."#)
                    .code("REFERENCED_INACCESSIBLE"),
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

            type Profile {
              name: String
              address: Address
            }

            type Address @inaccessible {
              street: String
              postalCode: String
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
                    .message_containing(r#"Type "Address" is @inaccessible but is referenced by "Profile.address", which is in the API schema."#)
                    .code("REFERENCED_INACCESSIBLE"),
            ],
        );

        assert_errors_contain(
            api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: [ "@inaccessible"]
              )

            type Query {
              a: Foo
            }

            scalar Foo
          "#, version)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: [ "@inaccessible"]
              )

            type Query {
              b: String
            }

            scalar Foo @inaccessible
          "#, version)),
            ]).errors(),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"Type "Foo" is @inaccessible but is referenced by "Query.a", which is in the API schema."#)
                    .code("REFERENCED_INACCESSIBLE"),
            ],
        );
    }
}
