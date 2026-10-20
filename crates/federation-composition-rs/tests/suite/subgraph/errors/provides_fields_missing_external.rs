//! Port of `__tests__/subgraph/errors/PROVIDES_FIELDS_MISSING_EXTERNAL.spec.ts`.

use crate::shared::*;

test_versions! {
    fn provides_fields_missing_external(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@provides"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              internalId: ID!
              profile: Profile @provides(fields: "name")
            }

            type Profile {
              name: String
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] On field "User.profile", for @provides(fields: "name"): field "Profile.name" should not be part of a @provides since it is already provided by this subgraph (it is not marked @external)"#,
                )
                .code("PROVIDES_FIELDS_MISSING_EXTERNAL")],
        );
    }

    fn provides_fields_missing_external_on_external_historical(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
            }
          "#, version)),
                service("reviews", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@provides", "@external", "@extends"]
              )

            type Query {
              review: Review
            }

            type Review {
              author: User! @provides(fields: "id")
            }

            type User @key(fields: "id") @extends {
              id: ID! @external # part of the key but also part of the @provides (which means it does not need @external, for historical reasons)
            }
          "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[reviews] On field "Review.author", for @provides(fields: "id"): field "User.id" should not be part of a @provides since it is already "effectively" provided by this subgraph (while it is marked @external, it is a @key field of an extension type, which are not internally considered external for historical/backward compatibility reasons)"#,
                )
                .code("PROVIDES_FIELDS_MISSING_EXTERNAL")],
        );
    }

    fn fed_v1_no_provides_fields_missing_external_on_external_historical(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          type Query {
            users: [User]
          }

          type User @key(fields: "id") {
            id: ID!
          }
        "#)),
            service("reviews", graphql!(r#"
          type Query {
            review: Review
          }

          type Review {
            author: User! @provides(fields: "id")
          }

          type User @key(fields: "id") @extends {
            id: ID! @external # part of the key but also part of the @provides (which means it does not need @external, for historical reasons)
          }
        "#)),
        ]);
        assert_composition_success(&result);
    }
}
