//! Port of `__tests__/subgraph/errors/PROVIDES_FIELDS_HAS_ARGS.spec.ts`.

use crate::shared::*;

test_versions! {
    fn provides_fields_has_args(api, version) {
        assert_errors_contain(
            &api.compose_services(&[
                service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              tags(limit: UserType = ADMIN): [String]
            }

            enum UserType {
              REGULAR
              ADMIN
            }
          "#, version)),
                service("feed", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external", "@provides"]
              )

            type Query {
              feed: [Article]
            }

            type Article @key(fields: "id") {
              id: ID!
              author: User! @provides(fields: "tags")
            }

            extend type User @key(fields: "isd") {
              id: ID!
              tags(limit: UserType = REGULAR): [String] @external
            }

            enum UserType {
              REGULAR
              ADMIN
            }
          "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[feed] On field "Article.author", for @provides(fields: "tags"): field User.tags cannot be included because it has arguments (fields with argument are not allowed in @provides)"#,
                )
                .code("PROVIDES_FIELDS_HAS_ARGS")],
        );
    }
}
