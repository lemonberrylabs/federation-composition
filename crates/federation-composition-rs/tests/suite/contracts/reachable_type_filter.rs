//! Port of `__tests__/contracts/reachable-type-filter.spec.ts`.

use crate::shared::*;

use federation_composition_rs::contracts::reachable_type_filter::{
    add_directive_on_types, get_reachable_types, AddDirectiveOnTypesArgs,
};

mod get_reachable_types {
    use super::*;

    #[test]
    fn includes_the_query_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 1);
        assert!(reachable_types.contains("Query"));
    }

    #[test]
    fn includes_the_mutation_type() {
        let document_node = parse(
            r#"
      type Mutation {
        hello: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 1);
        assert!(reachable_types.contains("Mutation"));
    }

    #[test]
    fn includes_the_subscription_type() {
        let document_node = parse(
            r#"
      type Subscription {
        hello: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 1);
        assert!(reachable_types.contains("Subscription"));
    }

    #[test]
    fn excludes_unused_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: String
      }
      type Mutation {
        hello: String
      }

      schema {
        query: Query
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 1);
        assert!(reachable_types.contains("Query"));
        assert!(!reachable_types.contains("Mutation"));
    }

    #[test]
    fn includes_object_types_referenced_by_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: Hello
      }
      type Hello {
        world: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 2);
        assert!(reachable_types.contains("Query"));
        assert!(reachable_types.contains("Hello"));
    }

    #[test]
    fn includes_scalar_types_referenced_by_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: Hello
      }
      scalar Hello
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 2);
        assert!(reachable_types.contains("Query"));
        assert!(reachable_types.contains("Hello"));
    }

    #[test]
    fn includes_input_types_referenced_by_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello(input: Hello): String
      }
      input Hello {
        world: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 2);
        assert!(reachable_types.contains("Query"));
        assert!(reachable_types.contains("Hello"));
    }

    #[test]
    fn includes_enum_types_referenced_by_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: Hello
      }
      enum Hello {
        WORLD
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 2);
        assert!(reachable_types.contains("Query"));
        assert!(reachable_types.contains("Hello"));
    }

    #[test]
    fn includes_union_type_and_union_members_referenced_by_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: Hello
      }
      union Hello = World
      union Gang = World
      type World {
        world: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 3);
        assert!(reachable_types.contains("Query"));
        assert!(reachable_types.contains("Hello"));
        assert!(reachable_types.contains("World"));
    }

    #[test]
    fn includes_interface_type_and_interface_members_referenced_by_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: Hello
      }
      interface Hello {
        world: String
      }
      type World implements Hello {
        world: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 3);
        assert!(reachable_types.contains("Query"));
        assert!(reachable_types.contains("Hello"));
        assert!(reachable_types.contains("World"));
    }

    #[test]
    fn includes_input_type_referenced_by_input_type() {
        let document_node = parse(
            r#"
      input Hello {
        world: World
      }
      input World {
        world: String
      }
      type Query {
        hello(world: Hello): String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert_eq!(reachable_types.len(), 3);
        assert!(reachable_types.contains("Query"));
        assert!(reachable_types.contains("Hello"));
        assert!(reachable_types.contains("World"));
    }

    #[test]
    fn includes_types_referenced_by_directive_arguments() {
        let document_node = parse(
            r#"
      directive @meta(options: MetaOptions) on FIELD_DEFINITION
      input MetaOptions {
        visibility: Visibility
      }
      enum Visibility {
        PUBLIC
        PRIVATE
      }
      type Query {
        hello: String
      }
    "#,
        )
        .unwrap();
        let reachable_types = get_reachable_types(&document_node).unwrap();
        assert!(reachable_types.contains("MetaOptions"));
        assert!(reachable_types.contains("Visibility"));
    }
}

mod add_directive_on_types {
    use super::*;

    #[test]
    fn add_directive_on_unused_root_type() {
        let document_node = parse(
            r#"
      type Query {
        hello: String
      }
      type Mutation {
        hello: String
      }

      schema {
        query: Query
      }
    "#,
        )
        .unwrap();
        let document = add_directive_on_types(AddDirectiveOnTypesArgs {
            document_node: &document_node,
            excluded_type_names: &get_reachable_types(&document_node).unwrap(),
            directive_name: "inaccessible",
        });
        to_match_inline_snapshot(
            print(&document),
            r#"
      "type Query {
        hello: String
      }

      type Mutation @inaccessible {
        hello: String
      }

      schema {
        query: Query
      }"
    "#,
        );
    }

    #[test]
    fn does_not_re_apply_directive() {
        let document_node = parse(
            r#"
      type Query {
        hello: String
      }

      type Mutation @inaccessible {
        hello: String
      }

      schema {
        query: Query
      }

      directive @inaccessible on OBJECT
    "#,
        )
        .unwrap();
        let document = add_directive_on_types(AddDirectiveOnTypesArgs {
            document_node: &document_node,
            excluded_type_names: &get_reachable_types(&document_node).unwrap(),
            directive_name: "inaccessible",
        });
        to_match_inline_snapshot(
            print(&document),
            r#"
      "type Query {
        hello: String
      }

      type Mutation @inaccessible {
        hello: String
      }

      schema {
        query: Query
      }

      directive @inaccessible on OBJECT"
    "#,
        );
    }

    #[test]
    fn runs_on_supergraph() {
        // This is technically not a fully valid supergraph document node.
        // It only includes the minimum required types and directives to test the functionality.
        let document_node = parse(
            r#"
      type Query @join__type(graph: BAR_GRAPHQL) {
        bar: Car @inaccessible
        barHidden: String
      }

      type Bar @join__type(graph: BAR_GRAPHQL) {
        hello: String
        helloHidden: String
      }

      type Car @join__type(graph: BAR_GRAPHQL) {
        hello: String @inaccessible
        helloHidden: String
      }

      schema {
        query: Query
      }

      ####
      # Note: all the directives and types below are part of a supergraph schema
      ####

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION
      scalar join__FieldSet
      directive @join__type(
        graph: join__Graph!
        key: join__FieldSet
        extension: Boolean! = false
        resolvable: Boolean! = true
        isInterfaceObject: Boolean! = false
      ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      enum join__Graph {
        BAR_GRAPHQL @join__graph(name: "bar.graphql", url: "")
      }
    "#,
        )
        .unwrap();

        let mut excluded_type_names = get_reachable_types(&document_node).unwrap();

        excluded_type_names.insert("join__Graph".to_string());
        excluded_type_names.insert("join__FieldSet".to_string());

        let document = add_directive_on_types(AddDirectiveOnTypesArgs {
            document_node: &document_node,
            excluded_type_names: &excluded_type_names,
            directive_name: "inaccessible",
        });

        to_match_inline_snapshot(
            print(&document),
            r#"
      "type Query @join__type(graph: BAR_GRAPHQL) {
        bar: Car @inaccessible
        barHidden: String
      }

      type Bar @join__type(graph: BAR_GRAPHQL) @inaccessible {
        hello: String
        helloHidden: String
      }

      type Car @join__type(graph: BAR_GRAPHQL) {
        hello: String @inaccessible
        helloHidden: String
      }

      schema {
        query: Query
      }

      directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

      scalar join__FieldSet

      directive @join__type(graph: join__Graph!, key: join__FieldSet, extension: Boolean! = false, resolvable: Boolean! = true, isInterfaceObject: Boolean! = false) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

      directive @join__graph(name: String!, url: String!) on ENUM_VALUE

      enum join__Graph {
        BAR_GRAPHQL @join__graph(name: "bar.graphql", url: "")
      }"
    "#,
        );
    }
}
