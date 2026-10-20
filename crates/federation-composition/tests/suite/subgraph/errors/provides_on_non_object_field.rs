//! Port of `__tests__/subgraph/errors/PROVIDES_ON_NON_OBJECT_FIELD.spec.ts`.

use crate::shared::*;

test_versions! {
    fn provides_on_non_object_field(api, version) {
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

            type RegisteredUser implements User @key(fields: "id") {
              id: ID!
              name: String!
              email: String
            }

            interface User @key(fields: "id") {
              id: ID!
              name: String!
              email: String @provides(fields: "name")
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"[users] Invalid @provides directive on field "User.email": field has type "String" which is not a Composite Type"#,
                )
                .code("PROVIDES_ON_NON_OBJECT_FIELD")],
        );
    }

    fn provides_on_non_object_field_is_not_raised_for_union_field_with_fragment_object_type_selection_sets(api, version) {
        let result = api.compose_services(&[
            service("users", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@provides", "@external"]
            )

          type Query {
            result: ResultType
          }

          type EntityA {
            field1: String @external
            field2: String @external
          }

          type EntityB {
            field1: String @external
            field2: String @external
            field3: String @external
          }

          union UnionType = EntityA | EntityB

          type ResultType {
            results: [UnionType!]! @provides(fields: "... on EntityA { field1 field2 } ... on EntityB { field1 field2 field3 }")
          }
        "#, version)),
            service("other", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@shareable"]
            )

          type Query {
            a: EntityA
            b: EntityB
          }

          union UnionType = EntityA | EntityB

          type EntityA @key(fields: "field1") {
            field1: String @shareable
            field2: String @shareable
          }

          type EntityB @key(fields: "field1")  {
            field1: String @shareable
            field2: String @shareable
            field3: String @shareable
          }
        "#, version)),
        ]);
        let errors = result.errors();
        assert!(errors.is_none(), "expected no errors, received: {errors:?}");
    }

    fn provides_on_non_object_field_is_not_raised_for_union_field_with_fragment_object_type_selection_sets_variant(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable", "@external"]
            )

          type Query {
            media: [Media] @shareable
          }

          union Media = Book | Movie

          type Book @key(fields: "id") {
            id: ID!
          }

          type Movie @key(fields: "id") {
            id: ID!
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable", "@provides", "@external"]
            )

          type Query {
            media: [Media] @shareable @provides(fields: "... on Book { title }")
          }

          union Media = Book | Movie

          type Book @key(fields: "id") {
            id: ID!
            title: String @external
          }

          type Movie @key(fields: "id") {
            id: ID!
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id") {
            id: ID!
            title: String @shareable
          }

          type Movie @key(fields: "id") {
            id: ID!
            title: String @shareable
          }
        "#)),
        ]);
        assert_composition_success(&result);
    }
}
