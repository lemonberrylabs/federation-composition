//! Port of `__tests__/contracts/federation-tag-extraction.spec.ts`.

use crate::shared::*;

use federation_composition_rs::__internal::contracts::tag_extraction::{
    apply_tag_filter_on_subgraphs, apply_tag_filter_to_inaccessible_transform_on_subgraph_schema,
    build_schema_coordinate_tag_register, SchemaCoordinateToTagsRegistry, TagFilterTransformResult,
};

/// `Array.from(output.typesWithAllFieldsInaccessible.entries())`.
fn types_with_all_fields_inaccessible_entries(output: &TagFilterTransformResult) -> Vec<(String, bool)> {
    output
        .types_with_all_fields_inaccessible
        .iter()
        .map(|(name, value)| (name.clone(), *value))
        .collect()
}

mod apply_tag_filter_to_inaccessible_transform_on_subgraph_schema {
    use super::*;

    mod correct_inaccessible_directive_usage_based_on_subgraph_version_imports {
        use super::*;

        #[test]
        fn federation_1() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema {
          query: Query
        }

        type Query {
          field1: String!
          field2: Type1!
        }

        type Type1 {
          field1: String!
        }
      "#,
            )
            .unwrap();

            let output_sdl = print(
                &apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                    &sdl,
                    &SchemaCoordinateToTagsRegistry::default(),
                    &filter,
                )
                .unwrap()
                .type_defs,
            );
            to_match_inline_snapshot(
                output_sdl,
                r#"
        "schema {
          query: Query
        }

        type Query {
          field1: String! @inaccessible
          field2: Type1! @inaccessible
        }

        type Type1 {
          field1: String! @inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn federation_2_without_import_argument() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema @link(url: "https://specs.apollo.dev/federation/v2.0") {
          query: Query
        }

        type Query {
          field1: String!
          field2: Type1!
        }

        type Type1 {
          field1: String!
        }
      "#,
            )
            .unwrap();

            let output_sdl = print(
                &apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                    &sdl,
                    &SchemaCoordinateToTagsRegistry::default(),
                    &filter,
                )
                .unwrap()
                .type_defs,
            );
            to_match_inline_snapshot(
                output_sdl,
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0") {
          query: Query
        }

        type Query {
          field1: String! @federation__inaccessible
          field2: Type1! @federation__inaccessible
        }

        type Type1 {
          field1: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn federation_2_with_import_argument() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@inaccessible"]
          ) {
          query: Query
        }

        type Query {
          field1: String!
          field2: Type1!
        }

        type Type1 {
          field1: String!
        }
      "#,
            )
            .unwrap();

            let output_sdl = print(
                &apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                    &sdl,
                    &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                    &filter,
                )
                .unwrap()
                .type_defs,
            );
            to_match_inline_snapshot(
                output_sdl,
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@inaccessible"]) {
          query: Query
        }

        type Query {
          field1: String! @inaccessible
          field2: Type1! @inaccessible
        }

        type Type1 {
          field1: String! @inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn federation_2_with_import_argument_alias() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: [{ name: "@inaccessible", as: "@inaccessible__federation" }]
          ) {
          query: Query
        }

        type Query {
          field1: String!
          field2: Type1!
        }

        type Type1 {
          field1: String!
        }
      "#,
            )
            .unwrap();

            let output_sdl = print(
                &apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                    &sdl,
                    &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                    &filter,
                )
                .unwrap()
                .type_defs,
            );
            to_match_inline_snapshot(
                output_sdl,
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: [{ name: "@inaccessible", as: "@inaccessible__federation" }]) {
          query: Query
        }

        type Query {
          field1: String! @inaccessible__federation
          field2: Type1! @inaccessible__federation
        }

        type Type1 {
          field1: String! @inaccessible__federation
        }"
      "#,
            );
        }
    }

    mod object_type {
        use super::*;

        #[test]
        fn include_of_object_type_field_via_single_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: String! @tag(name: "tag1")
          field2: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: String!
          field2: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn include_of_object_type_field_via_complex_filter_include_value() {
            let filter = contract_filter(&["tag1", "tag2"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag2")
          field2_1: String! @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: String!
          field2: String!
          field2_1: String!
          field3: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_object_type_field_via_single_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: String! @tag(name: "tag1")
          field2: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: String! @federation__inaccessible
          field2: String!
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_object_type_field_via_complex_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1", "tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag2")
          field2_1: String! @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: String! @federation__inaccessible
          field2: String! @federation__inaccessible
          field2_1: String! @federation__inaccessible
          field3: String!
        }"
      "#,
            );
        }

        #[test]
        fn include_and_exclude_of_object_type_field_will_result_in_its_removal() {
            let filter = contract_filter(&["tag1"], &["tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag1")
          field1_and_2: String! @tag(name: "tag1") @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
          field4: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: String!
          field2: String!
          field1_and_2: String! @federation__inaccessible
          field3: String! @federation__inaccessible
          field4: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn include_of_object_type_field_if_external_is_applied_to_field() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag", "@external"]
          ) {
          query: Query
        }

        type Query {
          field1: String! @tag(name: "tag1") @requires(fields: "field2")
          field2: String! @external
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag", "@external"]) {
          query: Query
        }

        type Query {
          field1: String! @requires(fields: "field2")
          field2: String! @external
        }"
      "#,
            );
        }

        #[test]
        fn include_of_object_type_field_if_external_is_applied_to_object() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag", "@external"]
          ) {
          query: Query
        }

        extend type Query @external {
          field2: String!
        }

        type Query {
          field1: String! @tag(name: "tag1") @requires(fields: "field2")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag", "@external"]) {
          query: Query
        }

        extend type Query @external {
          field2: String!
        }

        type Query {
          field1: String! @requires(fields: "field2")
        }"
      "#,
            );
        }

        #[test]
        fn object_type_is_excluded_even_if_one_of_its_fields_is_included() {
            let filter = contract_filter(&["tag1"], &["tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: String! @tag(name: "tag1")
          field1_1: Type2! @tag(name: "tag1")
        }

        type Type2 @tag(name: "tag2") {
          field1: ID! @tag(name: "tag1")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: String!
          field1_1: Type2!
        }

        type Type2 @federation__inaccessible {
          field1: ID! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn mutation_object_type_is_returned_in_types_with_all_fields_inaccessible_if_all_its_fields_are_excluded() {
            let filter = contract_filter(&[], &["exclude"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
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
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            assert_eq!(
                types_with_all_fields_inaccessible_entries(&output),
                vec![("Mutation".to_string(), true)],
            );

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
          mutation: Mutation
        }

        type Query {
          field1: String!
        }

        type Mutation {
          field1: ID! @federation__inaccessible
        }"
      "#,
            );
        }
    }

    mod interface_type {
        use super::*;

        #[test]
        fn include_of_interface_type_field_via_single_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Interface1! @tag(name: "tag1")
        }

        interface Interface1 {
          field1: String! @tag(name: "tag1")
          field2: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Interface1!
        }

        interface Interface1 {
          field1: String!
          field2: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn include_of_interface_type_field_via_complex_filter_include_value() {
            let filter = contract_filter(&["tag1", "tag2"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Interface1! @tag(name: "tag1")
        }

        interface Interface1 {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
          field4: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Interface1!
        }

        interface Interface1 {
          field1: String!
          field2: String!
          field3: String! @federation__inaccessible
          field4: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_interface_type_field_via_simple_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field2: Interface1! @tag(name: "tag2")
        }

        interface Interface1 {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag2")
          field3: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field2: Interface1!
        }

        interface Interface1 {
          field1: String! @federation__inaccessible
          field2: String!
          field3: String!
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_interface_type_field_via_complex_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1", "tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field3: Interface1! @tag(name: "tag3")
        }

        interface Interface1 {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
          field4: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field3: Interface1!
        }

        interface Interface1 {
          field1: String! @federation__inaccessible
          field2: String! @federation__inaccessible
          field3: String!
          field4: String!
        }"
      "#,
            );
        }

        #[test]
        fn include_and_exclude_of_interface_type_field_will_result_in_its_removal() {
            let filter = contract_filter(&["tag1"], &["tag2", "tag3"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field3: Interface1! @tag(name: "tag1")
        }

        interface Interface1 {
          field1: String! @tag(name: "tag1") @tag(name: "tag2")
          field2: String! @tag(name: "tag3")
          field3: String! @tag(name: "tag3")
          field4: String! @tag(name: "tag1")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field3: Interface1!
        }

        interface Interface1 {
          field1: String! @federation__inaccessible
          field2: String! @federation__inaccessible
          field3: String! @federation__inaccessible
          field4: String!
        }"
      "#,
            );
        }

        #[test]
        fn include_interface_type_via_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Interface1! @tag(name: "tag1")
          field2: Interface2!
        }

        interface Interface1 @tag(name: "tag1") {
          field1: String!
        }

        interface Interface2 {
          field1: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Interface1!
          field2: Interface2! @federation__inaccessible
        }

        interface Interface1 {
          field1: String!
        }

        interface Interface2 {
          field1: String! @federation__inaccessible
        }"
      "#,
            );
        }
    }

    mod enum_type {
        use super::*;

        #[test]
        fn include_enum_value_via_single_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);

            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Enum1
        }

        enum Enum1 {
          enumValue1 @tag(name: "tag1")
          enumValue2
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Enum1 @federation__inaccessible
        }

        enum Enum1 {
          enumValue1
          enumValue2 @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn include_enum_value_via_complex_filter_include_value() {
            let filter = contract_filter(&["tag1", "tag2"], &[]);

            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Enum1
        }

        enum Enum1 {
          enumValue1 @tag(name: "tag1")
          enumValue2 @tag(name: "tag2")
          enumValue2_1 @tag(name: "tag2")
          enumValue3 @tag(name: "tag3")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Enum1 @federation__inaccessible
        }

        enum Enum1 {
          enumValue1
          enumValue2
          enumValue2_1
          enumValue3 @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_enum_value_via_single_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Enum1!
        }

        enum Enum1 {
          enumValue1 @tag(name: "tag1")
          enumValue2
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Enum1!
        }

        enum Enum1 {
          enumValue1 @federation__inaccessible
          enumValue2
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_enum_type_value_via_complex_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1", "tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Enum1!
        }

        enum Enum1 {
          enumValue1 @tag(name: "tag1")
          enumValue2 @tag(name: "tag2")
          enumValue2_1 @tag(name: "tag2")
          enumValue3 @tag(name: "tag3")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Enum1!
        }

        enum Enum1 {
          enumValue1 @federation__inaccessible
          enumValue2 @federation__inaccessible
          enumValue2_1 @federation__inaccessible
          enumValue3
        }"
      "#,
            );
        }

        #[test]
        fn include_and_exclude_of_enum_type_value_will_result_in_its_removal() {
            let filter = contract_filter(&["tag1"], &["tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1: Enum1! @tag(name: "tag1")
        }

        enum Enum1 {
          enumValue1 @tag(name: "tag1")
          enumValue2 @tag(name: "tag1")
          enumValue2_1 @tag(name: "tag1") @tag(name: "tag2")
          enumValue3 @tag(name: "tag3")
          enumValue4
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1: Enum1!
        }

        enum Enum1 {
          enumValue1
          enumValue2
          enumValue2_1 @federation__inaccessible
          enumValue3 @federation__inaccessible
          enumValue4 @federation__inaccessible
        }"
      "#,
            );
        }
    }

    mod input_object_type {
        use super::*;

        #[test]
        fn include_input_object_type_field_via_single_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(input: Type1! @tag(name: "tag1")): String! @tag(name: "tag1")
        }

        input Type1 {
          field1: String! @tag(name: "tag1")
          field2: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(input: Type1!): String!
        }

        input Type1 {
          field1: String!
          field2: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn include_input_object_value_via_complex_filter_include_value() {
            let filter = contract_filter(&["tag1", "tag2"], &[]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(input: Type1! @tag(name: "tag1")): String! @tag(name: "tag1")
        }

        input Type1 {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag2")
          field2_1: String! @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            // Note: this results in an invalid schema, but still follows the rules of the filtering
            // The filtered schema must be validated afterwards.
            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(input: Type1!): String!
        }

        input Type1 {
          field1: String!
          field2: String!
          field2_1: String!
          field3: String! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_input_object_type_field_via_single_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(input: Type1!): String! @tag(name: "tag1")
        }

        input Type1 {
          field1: String! @tag(name: "tag1")
          field2: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(input: Type1!): String! @federation__inaccessible
        }

        input Type1 {
          field1: String! @federation__inaccessible
          field2: String!
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_input_object_type_field_via_complex_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1", "tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(input: Type1!): String!
        }

        input Type1 {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag2")
          field2_1: String! @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            // Note: this results in an invalid schema, but still follows the rules of the filtering
            // The filtered schema must be validated afterwards.
            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(input: Type1!): String!
        }

        input Type1 {
          field1: String! @federation__inaccessible
          field2: String! @federation__inaccessible
          field2_1: String! @federation__inaccessible
          field3: String!
        }"
      "#,
            );
        }

        #[test]
        fn include_and_exclude_of_input_type_field_will_result_in_its_removal() {
            let filter = contract_filter(&["tag1"], &["tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(input: Type1! @tag(name: "tag1")): String! @tag(name: "tag1")
        }

        input Type1 {
          field1: String! @tag(name: "tag1")
          field2: String! @tag(name: "tag1")
          field2_1: String! @tag(name: "tag1") @tag(name: "tag2")
          field3: String! @tag(name: "tag3")
          field4: String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            // Note: this results in an invalid schema, but still follows the rules of the filtering
            // The filtered schema must be validated afterwards.
            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(input: Type1!): String!
        }

        input Type1 {
          field1: String!
          field2: String!
          field2_1: String! @federation__inaccessible
          field3: String! @federation__inaccessible
          field4: String! @federation__inaccessible
        }"
      "#,
            );
        }
    }

    mod object_type_field_arguments {
        use super::*;

        #[test]
        fn object_field_arguments_are_included_via_single_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);

            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(arg1: String! @tag(name: "tag1"), arg2: String): String!
            @tag(name: "tag1")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(arg1: String!, arg2: String @federation__inaccessible): String!
        }"
      "#,
            );
        }

        #[test]
        fn object_field_arguments_are_included_via_complex_filter_include_value() {
            let filter = contract_filter(&["tag1", "tag2"], &[]);

            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(
            arg1: String! @tag(name: "tag1")
            arg2: String! @tag(name: "tag2")
            arg3: String @tag(name: "tag3")
          ): String! @tag(name: "tag1")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(arg1: String!, arg2: String!, arg3: String @federation__inaccessible): String!
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_object_field_arguments_via_single_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(arg1: String @tag(name: "tag1"), arg2: String!): String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            // Note: this results in an invalid schema, but still follows the rules of the filtering
            // The filtered schema must be validated afterwards.
            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(arg1: String @federation__inaccessible, arg2: String!): String!
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_object_type_field_arguments_via_complex_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1", "tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(
            arg1: String @tag(name: "tag1")
            arg2: String! @tag(name: "tag2")
          ): String!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            // Note: this results in an invalid schema, but still follows the rules of the filtering
            // The filtered schema must be validated afterwards.
            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(arg1: String @federation__inaccessible, arg2: String! @federation__inaccessible): String!
        }"
      "#,
            );
        }

        #[test]
        fn include_and_exclude_of_object_type_field_arguments_will_result_in_its_removal() {
            let filter = contract_filter(&["tag1"], &["tag2"]);
            let sdl = parse(
                r#"
        schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          ) {
          query: Query
        }

        type Query {
          field1(
            arg1: String! @tag(name: "tag1")
            arg2: String! @tag(name: "tag1") @tag(name: "tag2")
          ): String! @tag(name: "tag1")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            // Note: this results in an invalid schema, but still follows the rules of the filtering
            // The filtered schema must be validated afterwards.
            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"]) {
          query: Query
        }

        type Query {
          field1(arg1: String!, arg2: String! @federation__inaccessible): String!
        }"
      "#,
            );
        }
    }

    mod scalar_type {
        use super::*;

        #[test]
        fn include_of_scalar_type_field_via_single_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        scalar Scalar1 @tag(name: "tag1")
        scalar Scalar2

        type Query {
          field1: Scalar1! @tag(name: "tag1")
          field2: Scalar2!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        scalar Scalar1

        scalar Scalar2 @federation__inaccessible

        type Query {
          field1: Scalar1!
          field2: Scalar2! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn include_of_scalar_type_via_complex_filter_include_value() {
            let filter = contract_filter(&["tag1", "tag2"], &[]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        scalar Scalar1 @tag(name: "tag1")
        scalar Scalar2 @tag(name: "tag2")

        type Query {
          field1: Scalar1! @tag(name: "tag1")
          field2: Scalar2! @tag(name: "tag2")
          field3: Scalar2!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        scalar Scalar1

        scalar Scalar2

        type Query {
          field1: Scalar1!
          field2: Scalar2!
          field3: Scalar2! @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_scalar_type_via_single_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1"]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        scalar Scalar1 @tag(name: "tag1")
        scalar Scalar2 @tag(name: "tag2")

        type Query {
          field1: Scalar1! @tag(name: "tag1")
          field2: Scalar2! @tag(name: "tag2")
          field3: Scalar2!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        scalar Scalar1 @federation__inaccessible

        scalar Scalar2

        type Query {
          field1: Scalar1! @federation__inaccessible
          field2: Scalar2!
          field3: Scalar2!
        }"
      "#,
            );
        }

        #[test]
        fn exclude_of_scalar_type_via_complex_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1", "tag2"]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        scalar Scalar1 @tag(name: "tag1")
        scalar Scalar2

        type Query {
          field1: Scalar1! @tag(name: "tag1")
          field2: Scalar2! @tag(name: "tag2")
          field3: Scalar2!
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        scalar Scalar1 @federation__inaccessible

        scalar Scalar2

        type Query {
          field1: Scalar1! @federation__inaccessible
          field2: Scalar2! @federation__inaccessible
          field3: Scalar2!
        }"
      "#,
            );
        }

        #[test]
        fn include_and_exclude_of_scalar_type_will_result_in_its_removal() {
            let filter = contract_filter(&["tag1"], &["tag2"]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        scalar Scalar1 @tag(name: "tag1")
        scalar Scalar2 @tag(name: "tag1") @tag(name: "tag2")

        type Query {
          field1: Scalar1! @tag(name: "tag1")
          field2: Scalar2! @tag(name: "tag2")
          field3: Scalar2! @tag(name: "tag2")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        scalar Scalar1

        scalar Scalar2 @federation__inaccessible

        type Query {
          field1: Scalar1!
          field2: Scalar2! @federation__inaccessible
          field3: Scalar2! @federation__inaccessible
        }"
      "#,
            );
        }
    }

    mod union_type {
        use super::*;

        #[test]
        fn include_union_type_via_single_filter_include_value() {
            let filter = contract_filter(&["tag1"], &[]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        type Type1 {
          field1: String! @tag(name: "tag1")
        }

        type Type2 {
          field1: String! @tag(name: "tag1")
        }

        union Union1 @tag(name: "tag1") = Type1 | Type2
        union Union2 = Type1 | Type2

        type Query {
          field1: Union1 @tag(name: "tag1")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        type Type1 {
          field1: String!
        }

        type Type2 {
          field1: String!
        }

        union Union1 = Type1 | Type2

        union Union2 @federation__inaccessible = Type1 | Type2

        type Query {
          field1: Union1
        }"
      "#,
            );
        }

        #[test]
        fn include_union_type_via_complex_filter_include_value() {
            let filter = contract_filter(&["tag1", "tag2"], &[]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        type Type1 {
          field1: String! @tag(name: "tag1")
        }

        type Type2 {
          field1: String! @tag(name: "tag1")
        }

        union Union1 @tag(name: "tag1") = Type1 | Type2
        union Union2 @tag(name: "tag2") = Type1 | Type2
        union Union3 @tag(name: "tag3") = Type1 | Type2

        type Query {
          field1: Union1 @tag(name: "tag1")
          field2: Union2 @tag(name: "tag2")
          field3: Union3 @tag(name: "tag3")
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        type Type1 {
          field1: String!
        }

        type Type2 {
          field1: String!
        }

        union Union1 = Type1 | Type2

        union Union2 = Type1 | Type2

        union Union3 @federation__inaccessible = Type1 | Type2

        type Query {
          field1: Union1
          field2: Union2
          field3: Union3 @federation__inaccessible
        }"
      "#,
            );
        }

        #[test]
        fn exclude_union_type_via_single_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1"]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        type Type1 {
          field1: String!
        }

        type Type2 {
          field1: String!
        }

        union Union1 @tag(name: "tag1") = Type1 | Type2
        union Union2 @tag(name: "tag2") = Type1 | Type2
        union Union3 = Type1 | Type2

        type Query {
          field1: Union1 @tag(name: "tag1")
          field2: Union2 @tag(name: "tag2")
          field3: Union3
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        type Type1 {
          field1: String!
        }

        type Type2 {
          field1: String!
        }

        union Union1 @federation__inaccessible = Type1 | Type2

        union Union2 = Type1 | Type2

        union Union3 = Type1 | Type2

        type Query {
          field1: Union1 @federation__inaccessible
          field2: Union2
          field3: Union3
        }"
      "#,
            );
        }

        #[test]
        fn exclude_union_type_via_complex_filter_exclude_value() {
            let filter = contract_filter(&[], &["tag1", "tag2"]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        type Type1 {
          field1: String!
        }

        type Type2 {
          field1: String!
        }

        union Union1 @tag(name: "tag1") = Type1 | Type2
        union Union2 @tag(name: "tag2") = Type1 | Type2
        union Union3 = Type1 | Type2

        type Query {
          field1: Union1 @tag(name: "tag1")
          field2: Union2 @tag(name: "tag2")
          field3: Union3
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        type Type1 {
          field1: String!
        }

        type Type2 {
          field1: String!
        }

        union Union1 @federation__inaccessible = Type1 | Type2

        union Union2 @federation__inaccessible = Type1 | Type2

        union Union3 = Type1 | Type2

        type Query {
          field1: Union1 @federation__inaccessible
          field2: Union2 @federation__inaccessible
          field3: Union3
        }"
      "#,
            );
        }

        #[test]
        fn include_and_exclude_of_union_type_will_result_in_its_removal() {
            let filter = contract_filter(&["tag1"], &["tag2"]);

            let sdl = parse(
                r#"
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.0"
            import: ["@tag"]
          )

        type Type1 {
          field1: String!
        }

        type Type2 {
          field1: String!
        }

        union Union1 @tag(name: "tag1") = Type1 | Type2
        union Union2 @tag(name: "tag2") @tag(name: "tag1") = Type1 | Type2
        union Union3 = Type1 | Type2

        type Query {
          field1: Union1 @tag(name: "tag1")
          field2: Union2 @tag(name: "tag2") @tag(name: "tag1")
          field3: Union3
        }
      "#,
            )
            .unwrap();

            let output = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
                &sdl,
                &build_schema_coordinate_tag_register(&[&sdl]).unwrap(),
                &filter,
            )
            .unwrap();

            to_match_inline_snapshot(
                print(&output.type_defs),
                r#"
        "extend schema @link(url: "https://specs.apollo.dev/federation/v2.0", import: ["@tag"])

        type Type1 {
          field1: String! @federation__inaccessible
        }

        type Type2 {
          field1: String! @federation__inaccessible
        }

        union Union1 = Type1 | Type2

        union Union2 @federation__inaccessible = Type1 | Type2

        union Union3 @federation__inaccessible = Type1 | Type2

        type Query {
          field1: Union1
          field2: Union2 @federation__inaccessible
          field3: Union3 @federation__inaccessible
        }"
      "#,
            );
        }
    }
}

mod apply_tag_filter_on_subgraphs {
    use super::*;

    #[test]
    fn object_types_are_inaccessible_because_all_fields_are_inaccessible() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs = parse(
            r#"
      # @note requires federation to be linked now.
      # extend schema @link(url: "https://specs.apollo.dev/federation/v2.3")

      type Query {
        field1: String!
        field2: Type1!
      }

      type Type1 {
        field1: String!
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(&[service("subgraph1", type_defs)], &filter).unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field1: String! @inaccessible
        field2: Type1! @inaccessible
      }

      type Type1 @inaccessible {
        field1: String! @inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn object_type_that_is_only_defined_in_one_subgraph_is_inaccessible() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs1 = parse(
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@tag"])

      type Query {
        helloWorld: String @tag(name: "tag1")
      }
    "#,
        )
        .unwrap();

        let type_defs2 = parse(
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])

      type Query {
        users: [User]
      }

      type User @key(fields: "id") {
        id: ID!
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(
            &[service("subgraph1", type_defs1), service("subgraph2", type_defs2)],
            &filter,
        )
        .unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@tag"])

      type Query {
        helloWorld: String
      }"
    "#,
        );
        to_match_inline_snapshot(
            print(&result[1].type_defs),
            r#"
      "extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])

      type Query {
        users: [User] @federation__inaccessible
      }

      type User @key(fields: "id") @federation__inaccessible {
        id: ID! @federation__inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn object_types_are_accessible_because_at_least_one_field_is_accessible_in_one_subgraph_but_not_in_another() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs1 = parse(
            r#"
      type Query {
        field1: String!
        field2: Type1!
      }

      type Type1 {
        field1: String!
      }
    "#,
        )
        .unwrap();

        let type_defs2 = parse(
            r#"
      type Query {
        field1: String! @tag(name: "tag1")
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(
            &[service("subgraph1", type_defs1), service("subgraph1", type_defs2)],
            &filter,
        )
        .unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field1: String!
        field2: Type1! @inaccessible
      }

      type Type1 @inaccessible {
        field1: String! @inaccessible
      }"
    "#,
        );
        to_match_inline_snapshot(
            print(&result[1].type_defs),
            r#"
      "type Query {
        field1: String!
      }"
    "#,
        );
    }

    #[test]
    fn object_with_object_extension_that_has_no_tag_is_made_inaccessible() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs1 = parse(
            r#"
      type Query {
        field1: String!
        field2: Type1!
      }

      type Type1 {
        field1: String!
      }

      extend type Type1 {
        field2: String!
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(&[service("subgraph1", type_defs1)], &filter).unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field1: String! @inaccessible
        field2: Type1! @inaccessible
      }

      type Type1 @inaccessible {
        field1: String! @inaccessible
      }

      extend type Type1 {
        field2: String! @inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn object_with_object_extension_that_has_one_tag_is_not_made_inaccessible() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs1 = parse(
            r#"
      type Query {
        field1: String!
        field2: Type1!
      }

      type Type1 {
        field1: String!
      }

      extend type Type1 {
        field2: String! @tag(name: "tag1")
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(&[service("subgraph1", type_defs1)], &filter).unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field1: String! @inaccessible
        field2: Type1! @inaccessible
      }

      type Type1 {
        field1: String! @inaccessible
      }

      extend type Type1 {
        field2: String!
      }"
    "#,
        );
    }

    #[test]
    fn interface_types_are_inaccessible_because_all_fields_are_inaccessible() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs = parse(
            r#"
      type Query {
        field2: Interface1!
      }

      interface Interface1 {
        a: String!
      }

      type Type1 implements Interface1 {
        a: String!
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(&[service("subgraph1", type_defs)], &filter).unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field2: Interface1! @inaccessible
      }

      interface Interface1 @inaccessible {
        a: String! @inaccessible
      }

      type Type1 implements Interface1 @inaccessible {
        a: String! @inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn interface_types_are_accessible_because_at_least_one_field_is_accessible_in_one_subgraph_but_not_in_another() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs1 = parse(
            r#"
      type Query {
        field2: Interface1!
      }

      interface Interface1 {
        a: String!
      }

      type Type1 implements Interface1 {
        a: String!
      }
    "#,
        )
        .unwrap();

        let type_defs2 = parse(
            r#"
      type Query {
        field2: Interface1!
      }

      interface Interface1 {
        b: String! @tag(name: "tag1")
      }

      type Type1 implements Interface1 {
        a: String!
        b: String! @tag(name: "tag1")
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(
            &[service("subgraph1", type_defs1), service("subgraph1", type_defs2)],
            &filter,
        )
        .unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field2: Interface1! @inaccessible
      }

      interface Interface1 {
        a: String! @inaccessible
      }

      type Type1 implements Interface1 {
        a: String! @inaccessible
      }"
    "#,
        );
        to_match_inline_snapshot(
            print(&result[1].type_defs),
            r#"
      "type Query {
        field2: Interface1! @inaccessible
      }

      interface Interface1 {
        b: String!
      }

      type Type1 implements Interface1 {
        a: String! @inaccessible
        b: String!
      }"
    "#,
        );
    }

    #[test]
    fn input_types_are_inaccessible_because_all_fields_are_inaccessible() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs = parse(
            r#"
      type Query {
        field2: Input1!
      }

      input Input1 {
        a: String!
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(&[service("subgraph1", type_defs)], &filter).unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field2: Input1! @inaccessible
      }

      input Input1 @inaccessible {
        a: String! @inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn input_types_are_accessible_because_at_least_one_field_is_accessible_in_one_subgraph_but_not_in_another() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs1 = parse(
            r#"
      type Query {
        field2: Input1!
      }

      input Input1 {
        a: String!
      }
    "#,
        )
        .unwrap();

        let type_defs2 = parse(
            r#"
      type Query {
        field2: Input1!
      }

      input Input1 {
        b: String! @tag(name: "tag1")
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(
            &[service("subgraph1", type_defs1), service("subgraph1", type_defs2)],
            &filter,
        )
        .unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field2: Input1! @inaccessible
      }

      input Input1 {
        a: String! @inaccessible
      }"
    "#,
        );
        to_match_inline_snapshot(
            print(&result[1].type_defs),
            r#"
      "type Query {
        field2: Input1! @inaccessible
      }

      input Input1 {
        b: String!
      }"
    "#,
        );
    }

    #[test]
    fn enum_types_are_inaccessible_because_all_fields_are_inaccessible() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs = parse(
            r#"
      type Query {
        field2: Enum1!
      }

      enum Enum1 {
        A
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(&[service("subgraph1", type_defs)], &filter).unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field2: Enum1! @inaccessible
      }

      enum Enum1 @inaccessible {
        A @inaccessible
      }"
    "#,
        );
    }

    #[test]
    fn enum_types_are_accessible_because_at_least_one_field_is_accessible_in_one_subgraph_but_not_in_another() {
        let filter = contract_filter(&["tag1"], &[]);
        let type_defs1 = parse(
            r#"
      type Query {
        field2: Enum1!
      }

      enum Enum1 {
        A
      }
    "#,
        )
        .unwrap();

        let type_defs2 = parse(
            r#"
      type Query {
        field2: Enum1!
      }

      enum Enum1 {
        B @tag(name: "tag1")
      }
    "#,
        )
        .unwrap();

        let result = apply_tag_filter_on_subgraphs(
            &[service("subgraph1", type_defs1), service("subgraph1", type_defs2)],
            &filter,
        )
        .unwrap();

        to_match_inline_snapshot(
            print(&result[0].type_defs),
            r#"
      "type Query {
        field2: Enum1! @inaccessible
      }

      enum Enum1 {
        A @inaccessible
      }"
    "#,
        );
        to_match_inline_snapshot(
            print(&result[1].type_defs),
            r#"
      "type Query {
        field2: Enum1! @inaccessible
      }

      enum Enum1 {
        B
      }"
    "#,
        );
    }
}
