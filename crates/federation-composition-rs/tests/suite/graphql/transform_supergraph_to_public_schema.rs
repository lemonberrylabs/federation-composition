//! Port of `__tests__/graphql/transform-supergraph-to-public-schema.spec.ts`.

use crate::shared::*;

use federation_composition_rs::transform_supergraph_to_public_schema;

/// `print(transformSupergraphToPublicSchema(sdl))`, with graphql-js `print`.
fn print_public_schema(sdl: &Document) -> String {
    print(&transform_supergraph_to_public_schema(sdl).unwrap())
}

// TS: the spec file's top-level `describe` block has the file's name.
#[allow(clippy::module_inception)]
mod transform_supergraph_to_public_schema {
    use super::*;

    mod inaccessible {
        use super::*;

        #[test]
        fn scalar_removal() {
            let sdl = parse(
                r#"
        scalar Scalar1 @inaccessible
        scalar Scalar2
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(result_sdl, r#""scalar Scalar2""#);
        }

        #[test]
        fn enum_removal() {
            let sdl = parse(
                r#"
        enum Enum1 @inaccessible {
          VALUE1
          VALUE2
        }
        enum Enum2 {
          VALUE1
          VALUE2
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "enum Enum2 {
          VALUE1
          VALUE2
        }"
      "#,
            );
        }

        #[test]
        fn enum_value_removal() {
            let sdl = parse(
                r#"
        enum Enum {
          VALUE1 @inaccessible
          VALUE2
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "enum Enum {
          VALUE2
        }"
      "#,
            );
        }

        #[test]
        fn object_type_removal() {
            let sdl = parse(
                r#"
        type Object1 @inaccessible {
          field1: String
        }
        type Object2 {
          field1: String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "type Object2 {
          field1: String
        }"
      "#,
            );
        }

        #[test]
        fn object_field_removal() {
            let sdl = parse(
                r#"
        type Object {
          field1: String @inaccessible
          field2: String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "type Object {
          field2: String
        }"
      "#,
            );
        }

        #[test]
        fn interface_type_removal() {
            let sdl = parse(
                r#"
        interface Interface1 @inaccessible {
          field1: String
        }
        interface Interface2 {
          field1: String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "interface Interface2 {
          field1: String
        }"
      "#,
            );
        }

        #[test]
        fn interface_field_removal() {
            let sdl = parse(
                r#"
        interface Interface {
          field1: String @inaccessible
          field2: String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "interface Interface {
          field2: String
        }"
      "#,
            );
        }

        #[test]
        fn union_type_removal() {
            let sdl = parse(
                r#"
        union Union1 @inaccessible = Type1 | Type2
        union Union2 = Type1 | Type2
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(result_sdl, r#""union Union2 = Type1 | Type2""#);
        }

        #[test]
        fn object_field_argument_removal() {
            let sdl = parse(
                r#"
        type Object {
          field1(arg1: String @inaccessible): String
          field2(arg1: String): String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "type Object {
          field1: String
          field2(arg1: String): String
        }"
      "#,
            );
        }

        #[test]
        fn interface_field_argument_removal() {
            let sdl = parse(
                r#"
        interface Object {
          field1(arg1: String @inaccessible): String
          field2(arg1: String): String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "interface Object {
          field1: String
          field2(arg1: String): String
        }"
      "#,
            );
        }

        #[test]
        fn input_object_type_removal() {
            let sdl = parse(
                r#"
        input Input1 @inaccessible {
          field1: String
        }
        input Input2 {
          field1: String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "input Input2 {
          field1: String
        }"
      "#,
            );
        }

        #[test]
        fn input_object_field_removal() {
            let sdl = parse(
                r#"
        input Input {
          field1: String @inaccessible
          field2: String
        }
      "#,
            )
            .unwrap();
            let result_sdl = print_public_schema(&sdl);
            to_match_inline_snapshot(
                result_sdl,
                r#"
        "input Input {
          field2: String
        }"
      "#,
            );
        }
    }

    #[test]
    fn strips_out_all_federation_directives_and_types() {
        let sdl = parse(
            r#"
      schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION)
        @link(
          url: "https://specs.apollo.dev/inaccessible/v0.2"
          as: "federation__inaccessible"
          for: SECURITY
        ) {
        query: Query
      }

      directive @federation__inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION

      directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      directive @join__implements(
        graph: join__Graph!
        interface: String!
      ) repeatable on OBJECT | INTERFACE

      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__unionMember(
        graph: join__Graph!
        member: String!
      ) repeatable on UNION

      directive @link(
        url: String
        as: String
        for: link__Purpose
        import: [link__Import]
      ) repeatable on SCHEMA

      scalar join__FieldSet

      enum join__Graph {
        BRRT @join__graph(name: "brrt", url: "http://localhost/graphql")
        BUBUBU @join__graph(name: "bububu", url: "http://localhost:1/graphql")
      }

      scalar link__Import

      enum link__Purpose {
        SECURITY

        EXECUTION
      }

      type Query @join__type(graph: BRRT) @join__type(graph: BUBUBU) {
        foo: Int! @join__field(graph: BRRT)
        ok1: Int! @join__field(graph: BRRT)
        a: String! @join__field(graph: BUBUBU)
        oi: Type2 @federation__inaccessible @join__field(graph: BUBUBU)
      }

      type Type2
        @join__type(graph: BRRT, key: "id", extension: true)
        @join__type(graph: BUBUBU, key: "id") {
        id: ID! @federation__inaccessible
        inStock: Boolean! @join__field(graph: BRRT)
        field1: String! @federation__inaccessible @join__field(graph: BUBUBU)
      }
    "#,
        )
        .unwrap();

        let result_sdl = print_public_schema(&sdl);
        to_match_inline_snapshot(
            result_sdl,
            r#"
      "type Query {
        foo: Int!
        ok1: Int!
        a: String!
      }

      type Type2 {
        inStock: Boolean!
      }"
    "#,
        );
    }

    #[test]
    fn graphql_specification_directives_are_omitted_from_the_sdl() {
        let sdl = parse(
            r#"
      directive @skip(if: Boolean!) on FIELD | FRAGMENT_SPREAD | INLINE_FRAGMENT
      directive @include(
        if: Boolean!
      ) on FIELD | FRAGMENT_SPREAD | INLINE_FRAGMENT
      directive @deprecated(
        reason: String = "No longer supported"
      ) on FIELD_DEFINITION | ENUM_VALUE
    "#,
        )
        .unwrap();
        let result_sdl = print_public_schema(&sdl);
        to_match_inline_snapshot(result_sdl, r#""""#);
    }

    #[test]
    fn does_not_omit_deprecated_directive() {
        let sdl = parse(
            r#"
      type Query {
        foo: String @deprecated(reason: "jooo")
      }
    "#,
        )
        .unwrap();
        let result_sdl = print_public_schema(&sdl);
        to_match_inline_snapshot(
            result_sdl,
            r#"
      "type Query {
        foo: String @deprecated(reason: "jooo")
      }"
    "#,
        );
    }
}
