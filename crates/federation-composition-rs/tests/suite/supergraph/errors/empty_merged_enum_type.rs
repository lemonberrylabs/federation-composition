//! Port of `__tests__/supergraph/errors/EMPTY_MERGED_ENUM_TYPE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn empty_merged_enum_type(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

          type User @shareable {
            name: String!
          }

          enum UserType {
            ADMIN
            UNREGULAR
          }
        "#, version)),
            service("b", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@shareable"])

          type User @shareable {
            name: String!
          }

          enum UserType {
            REGULAR
            ANONYMOUS
          }

          type Query {
            users(type: UserType): [User]
          }
        "#, version)),
        ]);

        assert_errors_contain(
            &result,
            &[
                ErrorMatcher::new()
                    .message(r#"None of the values of enum type "UserType" are defined consistently in all the subgraphs defining that type. As only values common to all subgraphs are merged, this would result in an empty type."#)
                    .code("EMPTY_MERGED_ENUM_TYPE"),
            ],
        );
    }
}
