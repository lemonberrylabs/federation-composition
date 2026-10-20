//! Port of `__tests__/subgraph/errors/QUERY_ROOT_TYPE_INACCESSIBLE.spec.ts`.

use crate::shared::*;

test_versions! {
    fn query_root_type_inaccessible(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("book", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )

            extend schema {
              query: RootQuery
            }

            type RootQuery @inaccessible {
              words: [String]
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(r#"Type "Query" is @inaccessible but is the root query type, which must be in the API schema."#)
                .code("QUERY_ROOT_TYPE_INACCESSIBLE")],
        );

        assert_errors_contain(
            api.compose_services(&[service("book", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@inaccessible"]
              )

            type Query @inaccessible {
              words: [String]
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(r#"Type "Query" is @inaccessible but is the root query type, which must be in the API schema."#)
                .code("QUERY_ROOT_TYPE_INACCESSIBLE")],
        );
    }
}
