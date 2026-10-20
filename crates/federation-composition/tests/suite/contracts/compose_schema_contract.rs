//! Port of `__tests__/contracts/compose-schema-contract.spec.ts`.

use crate::shared::*;

use federation_composition::compose_schema_contract;

// TS: the spec file's top-level `describe` block has the file's name.
#[allow(clippy::module_inception)]
mod compose_schema_contract {
    use super::*;

    #[test]
    fn can_compose_a_schema_contract() {
        // `include: new Set("a")` is the set of the characters of "a".
        let result = compose_schema_contract(
            &[
                service(
                    "a",
                    parse(
                        r#"
            type Query {
              a: String @tag(name: "a")
            }
          "#,
                    )
                    .unwrap(),
                ),
                service(
                    "b",
                    parse(
                        r#"
            type Query {
              b: String @tag(name: "b")
            }
          "#,
                    )
                    .unwrap(),
                ),
            ],
            &contract_filter(&["a"], &[]),
            true,
        )
        .unwrap();

        assert_eq!(result.errors(), None);
        to_match_inline_snapshot(
            result.supergraph_sdl(),
            r#"
      "schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) @link(url: "https://specs.apollo.dev/inaccessible/v0.2", for: SECURITY) {
        query: Query
      }

      directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

      directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE

      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__unionMember(graph: join__Graph!, member: String!) repeatable on UNION

      scalar join__FieldSet

      directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA

      scalar link__Import

      enum link__Purpose {
        """
        `SECURITY` features provide metadata necessary to securely resolve fields.
        """
        SECURITY
        """
        `EXECUTION` features provide metadata necessary for operation execution.
        """
        EXECUTION
      }

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

      enum join__Graph {
        A @join__graph(name: "a", url: "")
        B @join__graph(name: "b", url: "")
      }

      type Query @join__type(graph: A) @join__type(graph: B) {
        a: String @join__field(graph: A)
        b: String @join__field(graph: B) @inaccessible
      }"
    "#,
        );
        to_match_inline_snapshot(
            result.public_sdl(),
            r#"
      "type Query {
        a: String
      }"
    "#,
        );
    }

    #[test]
    fn keeps_types_referenced_by_composed_directives_when_removing_unreachable_types() {
        let result = compose_schema_contract(
            &[service_with_url(
                "products",
                "https://products.example.com/graphql",
                parse(
                    r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.5"
                import: ["@key", "@tag", "@composeDirective"]
              )
              @link(url: "https://myspecs.dev/meta/v1.0", import: ["@meta"])
              @composeDirective(name: "@meta")

            directive @meta(options: MetaOptions) repeatable on FIELD_DEFINITION

            scalar MetaOptions

            type Product @key(fields: "id") {
              id: ID!
              name: String @meta(options: {}) @tag(name: "internal")
              secret: String @tag(name: "internal")
            }

            type Query {
              product: Product
            }
          "#,
                )
                .unwrap(),
            )],
            &contract_filter(&[], &["internal"]),
            true,
        )
        .unwrap();

        assert_eq!(result.errors(), None);
        assert!(result
            .public_sdl()
            .unwrap()
            .contains("directive @meta(options: MetaOptions) repeatable on FIELD_DEFINITION"));
        assert!(result.public_sdl().unwrap().contains("scalar MetaOptions"));
    }

    #[test]
    fn contract_mutation_type_is_not_part_of_the_public_schema_if_all_fields_are_excluded() {
        let sdl = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field1: String!
      }

      type Mutation {
        field1: ID! @tag(name: "exclude")
      }
    "#;

        let sdl2 = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field2: String!
      }

      type Mutation {
        field2: ID! @tag(name: "exclude")
      }
    "#;

        let result = compose_schema_contract(
            &[
                service_with_url("FOO_GRAPHQL", "https://lol.de", parse(sdl).unwrap()),
                service_with_url("FOO2_GRAPHQL", "https://trolol.de", parse(sdl2).unwrap()),
            ],
            &contract_filter(&[], &["exclude"]),
            true,
        )
        .unwrap();

        assert_eq!(result.errors(), None);
        to_match_inline_snapshot(
            result.public_sdl(),
            r#"
      "type Query {
        field1: String!
        field2: String!
      }"
    "#,
        );
        to_match_inline_snapshot(
            result.supergraph_sdl(),
            r#"
      "schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) @link(url: "https://specs.apollo.dev/inaccessible/v0.2", for: SECURITY) {
        query: Query
        mutation: Mutation
      }

      directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

      directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE

      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__unionMember(graph: join__Graph!, member: String!) repeatable on UNION

      scalar join__FieldSet

      directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA

      scalar link__Import

      enum link__Purpose {
        """
        `SECURITY` features provide metadata necessary to securely resolve fields.
        """
        SECURITY
        """
        `EXECUTION` features provide metadata necessary for operation execution.
        """
        EXECUTION
      }

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

      enum join__Graph {
        FOO_GRAPHQL @join__graph(name: "FOO_GRAPHQL", url: "https://lol.de")
        FOO2_GRAPHQL @join__graph(name: "FOO2_GRAPHQL", url: "https://trolol.de")
      }

      type Query @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) {
        field1: String! @join__field(graph: FOO_GRAPHQL)
        field2: String! @join__field(graph: FOO2_GRAPHQL)
      }

      type Mutation @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) @inaccessible {
        field1: ID! @join__field(graph: FOO_GRAPHQL) @inaccessible
        field2: ID! @join__field(graph: FOO2_GRAPHQL) @inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn contract_mutation_type_is_part_of_the_public_schema_if_not_all_fields_are_excluded() {
        let sdl = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field1: String!
      }

      type Mutation {
        field1: ID! @tag(name: "exclude")
      }
    "#;

        let sdl2 = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field2: String!
      }

      type Mutation {
        field2: ID!
      }
    "#;

        let result = compose_schema_contract(
            &[
                service_with_url("FOO_GRAPHQL", "https://lol.de", parse(sdl).unwrap()),
                service_with_url("FOO2_GRAPHQL", "https://trolol.de", parse(sdl2).unwrap()),
            ],
            &contract_filter(&[], &["exclude"]),
            true,
        )
        .unwrap();

        assert_eq!(result.errors(), None);
        to_match_inline_snapshot(
            result.public_sdl(),
            r#"
      "type Query {
        field1: String!
        field2: String!
      }

      type Mutation {
        field2: ID!
      }"
    "#,
        );
        to_match_inline_snapshot(
            result.supergraph_sdl(),
            r#"
      "schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) @link(url: "https://specs.apollo.dev/inaccessible/v0.2", for: SECURITY) {
        query: Query
        mutation: Mutation
      }

      directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

      directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE

      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__unionMember(graph: join__Graph!, member: String!) repeatable on UNION

      scalar join__FieldSet

      directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA

      scalar link__Import

      enum link__Purpose {
        """
        `SECURITY` features provide metadata necessary to securely resolve fields.
        """
        SECURITY
        """
        `EXECUTION` features provide metadata necessary for operation execution.
        """
        EXECUTION
      }

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

      enum join__Graph {
        FOO_GRAPHQL @join__graph(name: "FOO_GRAPHQL", url: "https://lol.de")
        FOO2_GRAPHQL @join__graph(name: "FOO2_GRAPHQL", url: "https://trolol.de")
      }

      type Query @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) {
        field1: String! @join__field(graph: FOO_GRAPHQL)
        field2: String! @join__field(graph: FOO2_GRAPHQL)
      }

      type Mutation @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) {
        field1: ID! @join__field(graph: FOO_GRAPHQL) @inaccessible
        field2: ID! @join__field(graph: FOO2_GRAPHQL)
      }"
    "#,
        );
    }

    #[test]
    fn contract_mutation_type_is_not_part_of_the_public_schema_if_no_fields_are_included() {
        let sdl = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field1: String! @tag(name: "include")
      }

      type Mutation {
        field1: ID!
      }
    "#;

        let sdl2 = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field2: String! @tag(name: "include")
      }

      type Mutation {
        field2: ID!
      }
    "#;

        let result = compose_schema_contract(
            &[
                service_with_url("FOO_GRAPHQL", "https://lol.de", parse(sdl).unwrap()),
                service_with_url("FOO2_GRAPHQL", "https://trolol.de", parse(sdl2).unwrap()),
            ],
            &contract_filter(&["include"], &[]),
            true,
        )
        .unwrap();

        assert_eq!(result.errors(), None);
        to_match_inline_snapshot(
            result.public_sdl(),
            r#"
      "type Query {
        field1: String!
        field2: String!
      }"
    "#,
        );
        to_match_inline_snapshot(
            result.supergraph_sdl(),
            r#"
      "schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) @link(url: "https://specs.apollo.dev/inaccessible/v0.2", for: SECURITY) {
        query: Query
        mutation: Mutation
      }

      directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

      directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE

      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__unionMember(graph: join__Graph!, member: String!) repeatable on UNION

      scalar join__FieldSet

      directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA

      scalar link__Import

      enum link__Purpose {
        """
        `SECURITY` features provide metadata necessary to securely resolve fields.
        """
        SECURITY
        """
        `EXECUTION` features provide metadata necessary for operation execution.
        """
        EXECUTION
      }

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

      enum join__Graph {
        FOO_GRAPHQL @join__graph(name: "FOO_GRAPHQL", url: "https://lol.de")
        FOO2_GRAPHQL @join__graph(name: "FOO2_GRAPHQL", url: "https://trolol.de")
      }

      type Query @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) {
        field1: String! @join__field(graph: FOO_GRAPHQL)
        field2: String! @join__field(graph: FOO2_GRAPHQL)
      }

      type Mutation @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) @inaccessible {
        field1: ID! @join__field(graph: FOO_GRAPHQL) @inaccessible
        field2: ID! @join__field(graph: FOO2_GRAPHQL) @inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn contract_mutation_type_is_part_of_the_public_schema_if_at_least_one_field_is_included() {
        let sdl = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field1: String! @tag(name: "include")
      }

      type Mutation {
        field1: ID!
      }
    "#;

        let sdl2 = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
        mutation: Mutation
      }

      type Query {
        field2: String! @tag(name: "include")
      }

      type Mutation {
        field2: ID! @tag(name: "include")
      }
    "#;

        let result = compose_schema_contract(
            &[
                service_with_url("FOO_GRAPHQL", "https://lol.de", parse(sdl).unwrap()),
                service_with_url("FOO2_GRAPHQL", "https://trolol.de", parse(sdl2).unwrap()),
            ],
            &contract_filter(&["include"], &[]),
            true,
        )
        .unwrap();

        assert_eq!(result.errors(), None);
        to_match_inline_snapshot(
            result.public_sdl(),
            r#"
      "type Query {
        field1: String!
        field2: String!
      }

      type Mutation {
        field2: ID!
      }"
    "#,
        );
        to_match_inline_snapshot(
            result.supergraph_sdl(),
            r#"
      "schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) @link(url: "https://specs.apollo.dev/inaccessible/v0.2", for: SECURITY) {
        query: Query
        mutation: Mutation
      }

      directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

      directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE

      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__unionMember(graph: join__Graph!, member: String!) repeatable on UNION

      scalar join__FieldSet

      directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA

      scalar link__Import

      enum link__Purpose {
        """
        `SECURITY` features provide metadata necessary to securely resolve fields.
        """
        SECURITY
        """
        `EXECUTION` features provide metadata necessary for operation execution.
        """
        EXECUTION
      }

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

      enum join__Graph {
        FOO_GRAPHQL @join__graph(name: "FOO_GRAPHQL", url: "https://lol.de")
        FOO2_GRAPHQL @join__graph(name: "FOO2_GRAPHQL", url: "https://trolol.de")
      }

      type Query @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) {
        field1: String! @join__field(graph: FOO_GRAPHQL)
        field2: String! @join__field(graph: FOO2_GRAPHQL)
      }

      type Mutation @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) {
        field1: ID! @join__field(graph: FOO_GRAPHQL) @inaccessible
        field2: ID! @join__field(graph: FOO2_GRAPHQL)
      }"
    "#,
        );
    }

    #[test]
    fn contract_scalar_is_inaccessible_despite_being_included_in_at_least_one_subraph() {
        let sdl2 = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
      }

      type Query {
        field1: Foo!
      }

      type Foo {
        field: Brr @tag(name: "include")
      }

      scalar Brr @tag(name: "include")
    "#;

        let sdl = r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.3"
          import: ["@tag"]
        ) {
        query: Query
      }

      type Query {
        c: Int @tag(name: "include")
      }

      scalar Brr
    "#;

        let result = compose_schema_contract(
            &[
                service_with_url("FOO_GRAPHQL", "https://lol.de", parse(sdl).unwrap()),
                service_with_url("FOO2_GRAPHQL", "https://trolol.de", parse(sdl2).unwrap()),
            ],
            &contract_filter(&["include"], &[]),
            false,
        )
        .unwrap();

        assert_eq!(result.errors(), None);
        to_match_inline_snapshot(
            result.public_sdl(),
            r#"
      "scalar Brr

      type Query {
        c: Int
      }

      type Foo {
        field: Brr
      }"
    "#,
        );
        to_match_inline_snapshot(
            result.supergraph_sdl(),
            r#"
      "schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) @link(url: "https://specs.apollo.dev/inaccessible/v0.2", for: SECURITY) {
        query: Query
      }

      directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

      directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE

      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__unionMember(graph: join__Graph!, member: String!) repeatable on UNION

      scalar join__FieldSet

      directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA

      scalar link__Import

      enum link__Purpose {
        """
        `SECURITY` features provide metadata necessary to securely resolve fields.
        """
        SECURITY
        """
        `EXECUTION` features provide metadata necessary for operation execution.
        """
        EXECUTION
      }

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

      enum join__Graph {
        FOO_GRAPHQL @join__graph(name: "FOO_GRAPHQL", url: "https://lol.de")
        FOO2_GRAPHQL @join__graph(name: "FOO2_GRAPHQL", url: "https://trolol.de")
      }

      scalar Brr @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL)

      type Query @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL) {
        c: Int @join__field(graph: FOO_GRAPHQL)
        field1: Foo! @join__field(graph: FOO2_GRAPHQL) @inaccessible
      }

      type Foo @join__type(graph: FOO2_GRAPHQL) {
        field: Brr
      }"
    "#,
        );

        let line = result
            .supergraph_sdl()
            .and_then(|sdl| sdl.split('\n').find(|line| line.contains("scalar Brr")));
        assert_eq!(
            line,
            Some(r#"scalar Brr @join__type(graph: FOO_GRAPHQL) @join__type(graph: FOO2_GRAPHQL)"#),
        );
    }
}
