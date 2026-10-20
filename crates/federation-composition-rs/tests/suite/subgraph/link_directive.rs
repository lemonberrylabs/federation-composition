//! Port of `__tests__/subgraph/link-directive.spec.ts`.

use crate::shared::*;

test_versions! {
    fn invalid_link_identifier(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("billing", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@override", "@external", "@provides"]
                  )
                  @link(url: "https://specs.apollo.dev", import: [{ name: "@key", as: "@renamed" }])

                extend type Payment @key(fields: "id") {
                  id: ID!
                  amount: Int! @external
                }

                type Invoice @key(fields: "id") {
                  id: ID!
                  amount: Int!
                  payment: Payment
                }
              "#, version)),
                service("payments", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: [{ name: "@key", as: "@renamed" }]
                  )

                type Query {
                  payments: [Payment]
                }

                type Payment @renamed(fields: "id") {
                  id: ID!
                  amount: Int!
                }
              "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    "Invalid @link url 'https://specs.apollo.dev': expected '/<feature>/vX.Y' (for example '/federation/v2.9')",
                )
                .code("INVALID_LINK_IDENTIFIER")],
        );

        assert_errors_contain(
            api.compose_services(&[
                service("billing", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@override", "@external", "@provides", "@key", "@requires"]
              )
              @link(url: "https://specs.apollo.dev/federation/not-a-version", import: [{ name: "@key", as: "@renamed" }])

            extend type Payment @key(fields: "id") {
              id: ID!
              tax: Int! @requires(fields: "amount")
              amount: Int! @external
            }

            type Invoice @key(fields: "id") {
              id: ID!
              amount: Int!
              payment: Payment
            }
          "#, version)),
                service("payments", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: [{ name: "@key", as: "@renamed" }]
              )

            type Query {
              payments: [Payment]
            }

            type Payment @renamed(fields: "id") {
              id: ID!
              amount: Int!
            }
          "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing(
                    "Expected a version string (of the form v1.2), got", // Hive correctly says "got null", apollo says "got not-a-version"
                )
                .code("INVALID_LINK_IDENTIFIER")],
        );
    }

    fn invalid_link_directive_usage_duplicate_inclusion_of_feature(api, version) {
        assert_errors_contain(
            api.compose_services(&[
                service("billing", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@override", "@external", "@provides"]
                )
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: [{ name: "@key", as: "@renamed" }]
                )

              extend type Payment @key(fields: "id") {
                id: ID!
                amount: Int! @override(from: "payments") @external
              }

              type Invoice @key(fields: "id") {
                id: ID!
                amount: Int!
                payment: Payment @provides(fields: true)
              }
            "#, version)),
                service("payments", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: [{ name: "@key", as: "@renamed" }]
                )

              type Query {
                payments: [Payment]
              }

              type Payment @renamed(fields: "id") {
                id: ID!
                amount: Int!
              }
            "#, version)),
            ]),
            &[ErrorMatcher::new()
                .message_containing("[billing] Duplicate inclusion of feature https://specs.apollo.dev/federation")
                .code("INVALID_LINK_DIRECTIVE_USAGE")],
        );
    }

    fn invalid_link_directive_usage_unknown_element(api, version) {
        assert_errors_equal(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@flexibility"])

            type User @key(fields: "name") {
              name: String
            }

            type Query {
              users: [User!]!
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(r#"[users] Cannot import unknown element "@flexibility"."#)
                .code("INVALID_LINK_DIRECTIVE_USAGE")],
        );
    }

    fn unknown_federation_link_version(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v6.9"
                import: ["@key"]
              )

            type Query {
              words: [String!]!
            }
          "#))]),
            &[ErrorMatcher::new()
                .message_containing("[users] Invalid version v6.9 for the federation feature in @link directive on schema")
                .code("UNKNOWN_FEDERATION_LINK_VERSION")],
        );
    }

    fn unknown_link_version(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/link/v6.9")

            type Query {
              words: [String!]!
            }
          "#))]),
            &[ErrorMatcher::new()
                .message_containing("[users] Schema uses unknown version v6.9 of the link spec")
                .code("UNKNOWN_LINK_VERSION")],
        );
    }

    fn remove_duplicated_link_spec_definitions(api, version) {
        assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
            schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@shareable"]
              ) {
              query: Query
            }

            directive @link(
              url: String!
              as: String
              for: link__Purpose
              import: [link__Import]
            ) repeatable on SCHEMA

            scalar link__Import

            enum link__Purpose {
              SECURITY
              EXECUTION
            }

            type Query {
              foo: String
            }
          "#, version))]));
    }

    // If url: is not a valid RFC 3986 url, then it MUST be treated as an opaque
    // identifier for the foreign schema. Such non‐URL inputs to url: SHOULD
    // NOT have name and version information extracted from them—both are null.
    // source: https://specs.apollo.dev/link/v1.0/#@link.url
    // (`api.runIf("guild", ..)` in the TS.)
    fn non_rfc_3986_url(api, version) {
        let result = api.compose_services(&[
            service("billing", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@override", "@external", "@provides", "@key", "@requires", "@shareable"]
              )
              @link(url: "file:///foo/bar", import: ["Bar"])

              scalar Bar

            extend type Payment @key(fields: "id") {
              id: ID!
              tax: Int! @requires(fields: "amount")
              amount: Int! @external
              bar: Bar @shareable
            }

            type Invoice @key(fields: "id") {
              id: ID!
              amount: Int!
              payment: Payment
            }
          "#, version)),
            service("payments", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: [{ name: "@key", as: "@renamed" }, "@shareable"]
              )
              @link(url: "file:///foo/bar", import: ["Bar"])

            scalar Bar

            type Query {
              payments: [Payment]
            }

            type Payment @renamed(fields: "id") {
              id: ID!
              amount: Int!
              bar: Bar @shareable
            }
          "#, version)),
        ]);
        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
        scalar Bar @join__type(graph: BILLING) @join__type(graph: PAYMENTS)
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
        type Payment
          @join__type(graph: BILLING, key: "id", extension: true)
          @join__type(graph: PAYMENTS, key: "id") {
          id: ID!
          tax: Int! @join__field(graph: BILLING, requires: "amount")
          amount: Int!
            @join__field(graph: BILLING, external: true)
            @join__field(graph: PAYMENTS)
          bar: Bar
        }
      "#);
    }
}
