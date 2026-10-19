//! Port of `__tests__/supergraph/errors/FIELD_TYPE_MISMATCH.spec.ts`.

use crate::shared::*;

test_versions! {
    fn field_type_mismatch(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            type Query {
              users: [User!]!
            }

            input Filter {
              ids: [ID!]!
            }

            type User @key(fields: "id") {
              id: ID
              name: String!
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
            
            extend type User @key(fields: "id") {
              id: ID
              name: UserName!
            }

            input Filter {
              ids: [Int!]
            }

            type UserName {
              first: String!
              last: String!
            }
          "#, version)),
            ]),
            &[
                ErrorMatcher::new()
                    .message_containing(
                        r#"Type of field "User.name" is incompatible across subgraphs: it has type "UserName!" in subgraph "feed" but type "String!" in subgraph "users""#,
                    )
                    .code("FIELD_TYPE_MISMATCH"),
                ErrorMatcher::new()
                    .message_containing(
                        r#"Type of field "Filter.ids" is incompatible across subgraphs: it has type "[Int!]" in subgraph "feed" but type "[ID!]!" in subgraph "users""#,
                    )
                    .code("FIELD_TYPE_MISMATCH"),
            ],
        );

        assert_errors_contain(
            &api.compose_services(&[
                service("auth", graphql!(r#"
            type User @key(fields: "id") {
              id: ID!
              name: String
            }

            type Query {
              me: User
            }
          "#)),
                service("images", graphql!(r#"
            type Image @key(fields: "url") {
              url: Url
              type: MimeType
            }

            type Query {
              images: [Image]
            }

            extend type User {
              favorite: Image
            }

            scalar Url
            scalar MimeType
          "#)),
                service("albums", graphql!(r#"
            type Album @key(fields: "id") {
              id: ID!
              user: User
              photos: [Image!]
            }

            extend type Image {
              albums: [Album!]
            }

            extend type User {
              albums: [Album!]
              favorite: Album
            }
          "#)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"Type of field "User.favorite" is incompatible across subgraphs: it has type "Album" in subgraph "albums" but type "Image" in subgraph "images""#,
                )
                .code("FIELD_TYPE_MISMATCH")],
        );

        assert_composition_success(&api.compose_services(&[
            service("me", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Query {
              me: User
            }

            type User @key(fields: "id") {
              id: ID!
              tags: [Tag!] @shareable
            }

            type Tag @key(fields: "id") {
              id: ID!
            }
          "#, version)),
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              tags: [Tag] @shareable
            }

            type Tag @key(fields: "id") {
              id: ID!
            }
          "#, version)),
        ]));

        assert_composition_success(&api.compose_services(&[
            service("me", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Query {
              me: User
            }

            type User @key(fields: "id") {
              id: ID!
              tags: [Tag!]! @shareable
            }

            type Tag @key(fields: "id") {
              id: ID!
            }
          "#, version)),
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              tags: [Tag]! @shareable
            }

            type Tag @key(fields: "id") {
              id: ID!
            }
          "#, version)),
        ]));
    }

    fn field_type_mismatch_unions(api, version) {
        assert_composition_success(&api.compose_services(&[
            service("foo", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

          type Query {
            foo: Item @shareable
          }

          union Item = A | B

          type A {
            id: ID! @shareable
            name: String @shareable
          }

          type B {
            id: ID!
            name: String!
          }

          type C {
            id: ID! @shareable
            name: String! @shareable
          }
        "#, version)),
            service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Query {
              foo: A @shareable
            }

            type A {
              id: ID! @shareable
              name: String @shareable
            }
          "#, version)),
        ]));

        assert_errors_contain(
            &api.compose_services(&[
                service("foo", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])
          
          type Query {
            foo: Item @shareable
          }

          union Item = A | B

          type A {
            id: ID! @shareable
            name: String @shareable
          }

          type B {
            id: ID!
            name: String!
          }

          type C {
            id: ID! @shareable
            name: String! @shareable
          }
        "#, version)),
                service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type Query {
              foo: AnotherItem @shareable
            }

            union AnotherItem = A | C

            type A {
              id: ID! @shareable
              name: String @shareable
            }

            type C {
              id: ID! @shareable
              name: String! @shareable
            }
          "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"Type of field "Query.foo" is incompatible across subgraphs: it has type "AnotherItem" in subgraph "bar" but type "Item" in subgraph "foo""#,
                )
                .code("FIELD_TYPE_MISMATCH")],
        );
    }
}
