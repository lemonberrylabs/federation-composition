//! Port of `__tests__/subgraph/errors/EXTERNAL_UNUSED.spec.ts`.

use crate::shared::*;

test_versions! {
    fn external_unused_on_field(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              name: String! @external
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] Field "User.name" is marked @external but is not used in any federation directive (@key, @provides, @requires) or to satisfy an interface; the field declaration has no use and should be removed (or the field should not be @external)."#,
                )
                .code("EXTERNAL_UNUSED")],
        );
    }

    fn external_unused_on_type(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@external"]
              )

            type Query {
              users: [User]
            }

            type User @key(fields: "id") @external {
              id: ID!
              name: String!
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] Field "User.name" is marked @external but is not used in any federation directive (@key, @provides, @requires) or to satisfy an interface; the field declaration has no use and should be removed (or the field should not be @external)."#,
                )
                .code("EXTERNAL_UNUSED")],
        );
    }

    fn external_unused_on_requires_with_union_selection_set_does_not_raise_an_error(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@requires"]
            )

            enum Status {
              ACTIVE
              CANCELLED
              EXPIRED
            }

            type EntityA {
              status: Status! @external
            }


            union UnionType = EntityA

            extend type FederatedEntity @key(fields: "id") {
              id: ID!
              requiredField: UnionType @external
              providedField: String! @requires(fields: "requiredField { ...on EntityA { status } } ")
            }
        "#, version)),
            service("entities", graphql!(r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/${version}"
            import: ["@key", "@external", "@requires"]
          )

          enum Status {
            ACTIVE
            CANCELLED
            EXPIRED
          }

          type EntityA @key(fields: "id") {
            id: ID!
            status: Status!
          }

          union UnionType = EntityA

          type FederatedEntity @key(fields: "id") {
            id: ID!
            requiredField: UnionType
          }

          type Query {
            a(id: ID): FederatedEntity
          }
        "#, version)),
        ]);
        let errors = result.errors();

        assert!(errors.is_none(), "expected no errors, received: {errors:?}");
    }

    fn fed_v1_no_external_unused(api, version) {
        let result = api.compose_services(&[
            service("b", graphql!(r#"
          type User @key(fields: "id") {
            id: ID!
            nickname: String! @external
            age: Int!
          }

          type Query {
            user: User
          }
        "#)),
            service("a", graphql!(r#"
          type Admin @key(fields: "id") {
            id: ID!
            age: Int!
          }

          type Query {
            admin: Admin
          }
        "#)),
        ]);
        assert_composition_success(&result);
    }

    fn fed_v1_no_external_unused_when_external_is_used_on_a_type(api, version) {
        assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
            type Query {
              users: [User]
            }

            type User @key(fields: "id") @external {
              id: ID!
              name: String!
            }
          "#))]));
    }

    fn fed_v1_no_external_unused_but_external_missing_on_base_on_key_field(api, version) {
        assert_errors_contain(
            api.compose_services(&[service("users", graphql!(r#"
            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID! @external
              name: String!
            }
          "#))]),
            &[ErrorMatcher::new()
                .message_containing(r#"Field "User.id" is marked @external on all the subgraphs in which it is listed (subgraph "users")."#)
                .code("EXTERNAL_MISSING_ON_BASE")],
        );

        assert_composition_success(&api.compose_services(&[
            service("b", graphql!(r#"
            type User @key(fields: "id") {
              id: ID!
              nick: String! @external
              age: Int!
            }

            type Query {
              user: User
            }
          "#)),
            service("a", graphql!(r#"
            type Admin @key(fields: "id") {
              id: ID!
              age: Int!
            }

            type Query {
              admin: Admin
            }
          "#)),
        ]));

        // `expect(api.composeServices(..))` with no matcher: only composes (and must not throw).
        api.compose_services(&[
            service("a", graphql!(r#"
            type User @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              users: [User]
            }
          "#)),
            service("b", graphql!(r#"
            extend type User @key(fields: "nickname") {
              nickname: String! @external
              age: Int!
            }

            type Query {
              user: User
            }
          "#)),
        ]);

        assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              name: String! @external
            }
          "#))]));
    }
}
