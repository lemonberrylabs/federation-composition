//! Port of `__tests__/interface-object-composition.spec.ts`.

use crate::shared::*;

mod interface_object_composition {
    use super::*;

    test_implementations! {
        fn if_link_directive_is_not_present_on_all_subgraphs_composition_should_fail(api) {
            let result = api.compose_services(&[
                service("a", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type Query {
              hello: String
            }

            interface MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }
          "#).unwrap()),
                service("b", parse(r#"
            type Query {
              otherField: String
            }

            type MyInterface @key(fields: "id") @interfaceObject {
              id: ID!
              newField: String
            }
          "#).unwrap()),
            ]);

            federation_composition_rs::compose::assert_composition_failure(&result, None).unwrap();

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[b] Unknown directive "@interfaceObject". If you meant the "@interfaceObject" federation 2 directive, note that this schema is a federation 1 schema. To be a federation 2 schema, it needs to @link to the federation specification v2."#)
                .code("INVALID_GRAPHQL")]);
        }

        fn link_directive_should_have_url_pointing_to_federation_2_3_to_enable_interface_object_on_all_subgraphs(api) {
            let result = api.compose_services(&[
                service("a", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.1"
                import: ["@key", "@interfaceObject"]
              )

            type Query {
              hello: MyInterface
            }

            interface MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }

            type MyType implements MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }
          "#).unwrap()),
                service("b", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )
            type Query {
              otherField: MyInterface
            }

            type MyInterface @key(fields: "id") @interfaceObject {
              id: ID!
              newField: String
            }
          "#).unwrap()),
            ]);

            federation_composition_rs::compose::assert_composition_failure(&result, None).unwrap();

            to_match_inline_snapshot(result.errors(), r#"
        [
          [GraphQLError: [a] Cannot import unknown element "@interfaceObject".],
        ]
      "#);
        }

        fn external_requires_interface_object(api) {
            let result = api.compose_services(&[
                service("a", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject", "@shareable"]
              )

            type Query {
              hello: MyInterface
            }

            interface MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }

            type MyType implements MyInterface @key(fields: "id") {
              id: ID!
              field: String @shareable
            }
          "#).unwrap()),
                service("b", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type MyInterface @key(fields: "id") @interfaceObject {
              id: ID!
              newField: String
            }
          "#).unwrap()),
                service("c", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: [
                  "@key"
                  "@interfaceObject"
                  "@shareable"
                  "@requires"
                  "@external"
                ]
              )

            type MyInterface
              @key(fields: "id", resolvable: false)
              @interfaceObject {
              id: ID!
              newField: String @external
              field: String @shareable @requires(fields: "newField")
            }
          "#).unwrap()),
            ]);

            federation_composition_rs::compose::assert_composition_success(&result, None).unwrap();

            to_contain_graphql(result.supergraph_sdl(), r#"
        interface MyInterface
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id", isInterfaceObject: true)
          @join__type(
            graph: C
            key: "id"
            isInterfaceObject: true
            resolvable: false
          ) {
          id: ID!
          field: String
            @join__field(graph: A)
            @join__field(graph: C, requires: "newField")
          newField: String
            @join__field(external: true, graph: C)
            @join__field(graph: B)
        }
      "#);
        }

        fn link_directive_does_not_have_to_import_interface_object_in_all_subgraphs(api) {
            let result = api.compose_services(&[
                service("a", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key"]
              )

            type Query {
              hello: MyInterface
            }

            interface MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }

            type MyType implements MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }
          "#).unwrap()),
                service("b", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )
            type Query {
              otherField: MyInterface
            }

            type MyInterface @key(fields: "id") @interfaceObject {
              id: ID!
              newField: String
            }
          "#).unwrap()),
            ]);

            federation_composition_rs::compose::assert_composition_success(&result, None).unwrap();
        }

        fn target_interface_must_have_key_directive_on_subgraph_where_it_s_defined(api) {
            let result = api.compose_services(&[
                service("a", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type Query {
              hello: MyInterface
            }

            interface MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }

            type MyType implements MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }
          "#).unwrap()),
                service("b", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )
            type Query {
              otherField: MyInterface
            }

            type MyInterface @key(fields: "id") @interfaceObject {
              id: ID!
              newField: String
            }
          "#).unwrap()),
            ]);

            federation_composition_rs::compose::assert_composition_success(&result, None).unwrap();
        }

        // Subgraph A must define every entity type in your entire supergraph that implements MyInterface.
        // Certain other subgraphs can also define these entities, but Subgraph A must define all of them.
        // You can think of a subgraph that defines an entity interface as also owning every entity that implements that interface.
        // this case is really unclear.
        // documentation: https://www.apollographql.com/docs/federation/federated-types/interfaces/#usage-rules
        fn subgraph_where_interface_is_defined_must_have_all_entity_types_which_implement_that_interface_defined(api) {
            let result = api.compose_services(&[
                service("a", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type Query {
              hello: MyInterface
            }

            interface MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }
          "#).unwrap()),
                service("b", parse(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )
            type Query {
              otherField: MyInterface
            }

            type MyInterface @key(fields: "id") @interfaceObject {
              id: ID!
              newField: String
            }

            type MyType implements MyInterface @key(fields: "id") {
              id: ID!
              field: String
            }
          "#).unwrap()),
            ]);

            federation_composition_rs::compose::assert_composition_failure(&result, None).unwrap();

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[b] Cannot implement non-interface type MyInterface (of type ObjectType)"#)
                .code("INVALID_GRAPHQL")]);
        }
    }

    mod interface_object_definition {
        use super::*;

        mod at_least_one_other_subgraph_must_define_an_interface_type_with_key_directive_which_has_the_same_name_as_the_object_type_with_interface_object {
            use super::*;

            test_implementations! {
                fn interface_type_is_not_present_on_any_subgraph_should_fail(api) {
                    let result = api.compose_services(&[
                        service("a", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )

                type Query {
                  hello: String
                }
              "#).unwrap()),
                        service("b", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )
                type Query {
                  otherField: MyInterface
                }

                type MyInterface @key(fields: "id") @interfaceObject {
                  id: ID!
                  newField: String
                }
              "#).unwrap()),
                    ]);

                    federation_composition_rs::compose::assert_composition_failure(&result, None).unwrap();

                    assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                        .message(r#"Type "MyInterface" is declared with @interfaceObject in all the subgraphs in which is is defined"#)
                        .code("INTERFACE_OBJECT_USAGE_ERROR")]);
                }

                fn interface_type_is_present_on_other_subgraph_but_doesn_t_have_key_directive_should_fail(api) {
                    let result = api.compose_services(&[
                        service("a", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )

                type Query {
                  hello: MyInterface
                }

                interface MyInterface {
                  id: ID!
                  field: String
                }

                type SomeType implements MyInterface @key(fields: "id") {
                  id: ID!
                  field: String
                }
              "#).unwrap()),
                        service("b", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )
                type Query {
                  otherField: MyInterface
                }

                type MyInterface @interfaceObject {
                  id: ID!
                  newField: String
                }
              "#).unwrap()),
                    ]);

                    federation_composition_rs::compose::assert_composition_failure(&result, None).unwrap();

                    assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                        .message(r#"[b] The @interfaceObject directive can only be applied to entity types but type "MyInterface" has no @key in this subgraph."#)
                        .code("INTERFACE_OBJECT_USAGE_ERROR")]);
                }

                fn interface_type_is_present_on_other_subgraph_with_key_directive_should_succeed_and_add_fields_from_interface_object(api) {
                    let result = api.compose_services(&[
                        service("a", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )

                type Query {
                  hello: MyInterface
                }

                interface MyInterface @key(fields: "id") {
                  id: ID!
                  field: String
                }

                type IimplementMyInterface implements MyInterface
                  @key(fields: "id") {
                  id: ID!
                  field: String
                }
              "#).unwrap()),
                        service("b", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )
                type Query {
                  otherField: MyInterface
                }

                type MyInterface @key(fields: "id") @interfaceObject {
                  id: ID!
                  hello: Int
                }
              "#).unwrap()),
                    ]);

                    federation_composition_rs::compose::assert_composition_success(&result, None).unwrap();

                    to_contain_graphql(result.supergraph_sdl(), r#"
            type IimplementMyInterface implements MyInterface
              @join__type(graph: A, key: "id")
              @join__implements(graph: A, interface: "MyInterface") {
              id: ID!
              field: String
              hello: Int @join__field
            }
          "#);

                    to_contain_graphql(result.supergraph_sdl(), r#"
            interface MyInterface
              @join__type(graph: A, key: "id")
              @join__type(graph: B, isInterfaceObject: true, key: "id") {
              id: ID!
              field: String @join__field(graph: A)
              hello: Int @join__field(graph: B)
            }
          "#);
                }

                fn interface_type_is_present_on_other_subgraph_with_key_directive_should_succeed_and_add_fields_from_a_child_of_an_interface_object(api) {
                    let result = api.compose_services(&[
                        service("a", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.6"
                    import: ["@key", "@interfaceObject"]
                  )

                interface Media @key(fields: "id") {
                  id: ID!
                  title: String!
                }

                type Book implements Media @key(fields: "id") {
                  id: ID!
                  title: String!
                }
              "#).unwrap()),
                        service("b", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.6"
                    import: ["@key", "@interfaceObject"]
                  )

                type Media @key(fields: "id") @interfaceObject {
                  id: ID!
                  reviews: [Review!]!
                }

                type Review {
                  score: Int!
                }

                type Query {
                  topRatedMedia: [Media!]!
                }
              "#).unwrap()),
                    ]);

                    federation_composition_rs::compose::assert_composition_success(&result, None).unwrap();
                }
            }
        }

        mod fields_contribution {
            use super::*;

            test_implementations! {
                fn several_subgraphs_contribute_fields_to_the_same_interface_through_interface_object_should_succeed(api) {
                    let result = api.compose_services(&[
                        service("a", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )

                type Query {
                  hello: MyInterface
                }

                interface MyInterface @key(fields: "id") {
                  id: ID!
                  field: String
                }

                type IimplementMyInterface implements MyInterface
                  @key(fields: "id") {
                  id: ID!
                  field: String
                }
              "#).unwrap()),
                        service("b", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )
                type Query {
                  otherField: MyInterface
                }

                type MyInterface @key(fields: "id") @interfaceObject {
                  id: ID!
                  hello: Int
                }
              "#).unwrap()),
                        service("c", parse(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/v2.3"
                    import: ["@key", "@interfaceObject"]
                  )
                type Query {
                  someNewField: MyInterface
                }

                type MyInterface @key(fields: "id") @interfaceObject {
                  id: ID!
                  hello2: Int
                }
              "#).unwrap()),
                    ]);

                    federation_composition_rs::compose::assert_composition_success(&result, None).unwrap();

                    to_contain_graphql(result.supergraph_sdl(), r#"
            type IimplementMyInterface implements MyInterface
              @join__type(graph: A, key: "id")
              @join__implements(graph: A, interface: "MyInterface") {
              id: ID!
              field: String
              hello: Int @join__field
              hello2: Int @join__field
            }
          "#);

                    to_contain_graphql(result.supergraph_sdl(), r#"
            interface MyInterface
              @join__type(graph: A, key: "id")
              @join__type(graph: B, isInterfaceObject: true, key: "id")
              @join__type(graph: C, isInterfaceObject: true, key: "id") {
              id: ID!
              field: String @join__field(graph: A)
              hello: Int @join__field(graph: B)
              hello2: Int @join__field(graph: C)
            }
          "#);
                }

                #[ignore = "skipped in TypeScript"]
                fn interface_object_tries_to_contribute_field_with_conflicting_type_should_fail(api) {
                }

                #[ignore = "skipped in TypeScript"]
                fn several_interfaces_and_several_interface_objects_should_succeed(api) {
                }
            }
        }
    }

    test_implementations! {
        fn satisfiability_with_interface_object(api) {
            let result = api.compose_services(&[
                service("a", parse(r#"
            type Query {
              book: Book!
            }

            interface Media @key(fields: "id") {
              id: ID!
            }

            type Book implements Media @key(fields: "id") {
              id: ID!
              isbn: String!
            }

            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key"]
              )
          "#).unwrap()),
                service("b", parse(r#"
            type Media @key(fields: "id") @interfaceObject {
              id: ID!
            }

            type Query {
              topRatedMedia: [Media!]!
            }

            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )
          "#).unwrap()),
                service("c", parse(r#"
            type Query {
              cBook: Book
            }

            type Book @key(fields: "id") {
              id: ID!
            }

            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key"]
              )
          "#).unwrap()),
            ]);

            assert!(result.errors().is_none());
        }
    }
}
