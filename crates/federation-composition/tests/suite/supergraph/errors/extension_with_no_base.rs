//! Port of `__tests__/supergraph/errors/EXTENSION_WITH_NO_BASE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn fed_v1_extension_with_no_base(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("products", graphql!(r#"
            type Product @extends @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              products: [Product]
            }
          "#)),
                service("reviews", graphql!(r#"
            type Query {
              reviews: [String]
            }
          "#)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"[products] Type "Product" is an extension type, but there is no type definition for "Product" in any subgraph."#)
                    .code("EXTENSION_WITH_NO_BASE"),
            ],
        );

        assert_errors_contain(
            api.compose_services(&[
                service("products", graphql!(r#"
            type Product @extends @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              products: [Product]
            }
          "#)),
                service("reviews", graphql!(r#"
            extend type Product @key(fields: "id") {
              id: ID!
              reviews: [String]
            }

            type Query {
              reviews: [String]
            }
          "#)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"[products] Type "Product" is an extension type, but there is no type definition for "Product" in any subgraph."#)
                    .code("EXTENSION_WITH_NO_BASE"),
            ],
        );

        assert_composition_success(&api.compose_services(&[
            service("products", graphql!(r#"
            type Product @extends @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              products: [Product]
            }
          "#)),
            service("reviews", graphql!(r#"
            type Product @extends @key(fields: "id") {
              id: ID!
              name: String!
              reviews: [String]
            }

            type Query {
              reviews: [String]
            }
          "#)),
        ]));

        assert_composition_success(&api.compose_services(&[
            service("products", graphql!(r#"
            type Product @key(fields: "id") {
              id: ID!
            }

            extend type Product {
              name: String!
            }

            extend type Query {
              products: [Product]
            }
          "#)),
            service("reviews", graphql!(r#"
            type Product @extends @key(fields: "id") {
              id: ID! @external
              reviews: [String]
            }

            type Query {
              reviews: [String]
            }
          "#)),
        ]));
    }

    #[ignore = "skipped in TypeScript: test.skipIf(api.library === \"guild\")"]
    fn fed_v1_extension_with_no_base_fails_in_our_implementation(api, version) {
        // KAMIL: I have no idea why this supposed to succeed and what's going on here...
        assert_composition_success(&api.compose_services(&[
            service("products", graphql!(r#"
              type Product @extends @key(fields: "id") {
                id: ID!
                name: String!
              }

              type Query {
                products: [Product]
              }
            "#)),
            service("reviews", graphql!(r#"
              type Product @extends @key(fields: "id") {
                id: ID!
                name: String!
                reviews: [String]
              }

              type Query {
                reviews: [String]
              }
            "#)),
            service("foo", graphql!(r#"
              extend type Product @key(fields: "id") {
                id: ID!
                name: String!
                foo: String
              }

              type Query {
                foo: Product
              }
            "#)),
        ]));
    }

    fn extension_with_no_base(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID
              profile: Profile
            }

            extend type Profile {
              name: String
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(r#"[users] Type "Profile" is an extension type, but there is no type definition for "Profile" in any subgraph."#)
                    .code("EXTENSION_WITH_NO_BASE"),
            ],
        );

        assert_composition_success(&api.compose_services(&[
            service("products", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
              
              type Product @key(fields: "id") {
                id: ID!
              }

              extend type Product {
                name: String
              }

              type Query {
                products: [Product]
              }
            "#, version)),
            service("reviews", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@extends", "@external"])
              type Product @key(fields: "id") @extends {
                id: ID! @external
                reviews: [String]
              }
            "#, version)),
        ]));

        assert_composition_success(&api.compose_services(&[
            service("products", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@extends"])
              
              type Product @extends @key(fields: "id") {
                id: ID!
                name: String!
              }

              extend type Product {
                inStock: Boolean!
              }

              type Query {
                products: [Product]
              }
            "#, version)),
            service("reviews", graphql!(r#"
            type Query {
              reviews: [String]
            }
          "#)),
        ]));

        assert_composition_success(&api.compose_services(&[
            service("products", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@extends"])
              
              type Product @extends @key(fields: "id") {
                id: ID!
                name: String!
              }

              type Query {
                products: [Product]
              }
            "#, version)),
            service("reviews", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@extends"])
            
            type Product @extends @key(fields: "id") {
              id: ID!
              reviews: [String]
            }

            type Query {
              reviews: [String]
            }
          "#, version)),
        ]));
    }
}
