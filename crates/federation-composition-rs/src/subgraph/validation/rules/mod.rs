//! Port of `src/subgraph/validation/rules/`.

pub mod elements;
pub mod known_argument_names_on_directives_rule;
pub mod known_directives_rule;
pub mod known_federation_directive_rule;
pub mod known_root_type_rule;
pub mod known_type_names_rule;
pub mod lone_schema_definition_rule;
pub mod only_interface_implementation_rule;
pub mod provided_arguments_on_directives_rule;
pub mod provided_required_arguments_on_directives_rule;
pub mod query_root_type_inaccessible_rule;
pub mod reserved_subgraph_name_rule;
pub mod root_type_used_rule;
pub mod unique_argument_definition_names_rule;
pub mod unique_argument_names_rule;
pub mod unique_directive_names_rule;
pub mod unique_directives_per_location_rule;
pub mod unique_enum_value_names_rule;
pub mod unique_field_definition_names_rule;
pub mod unique_input_field_names_rule;
pub mod unique_operation_types_rule;
pub mod unique_type_names_rule;

/// The graphql rules that run with the `SimpleValidationContext` need only the `graphql_rs`
/// crate, so they are tested here on small documents.
#[cfg(test)]
mod tests {
    use graphql_rs::{NodeRef, VisitAction, VisitInfo, Visitor};

    use super::known_argument_names_on_directives_rule::KnownArgumentNamesOnDirectivesRule;
    use super::known_directives_rule::KnownDirectivesRule;
    use super::lone_schema_definition_rule::LoneSchemaDefinitionRule;
    use super::only_interface_implementation_rule::OnlyInterfaceImplementationRule;
    use super::unique_argument_definition_names_rule::UniqueArgumentDefinitionNamesRule;
    use super::unique_argument_names_rule::UniqueArgumentNamesRule;
    use super::unique_directive_names_rule::UniqueDirectiveNamesRule;
    use super::unique_directives_per_location_rule::UniqueDirectivesPerLocationRule;
    use super::unique_enum_value_names_rule::UniqueEnumValueNamesRule;
    use super::unique_field_definition_names_rule::UniqueFieldDefinitionNamesRule;
    use super::unique_input_field_names_rule::UniqueInputFieldNamesRule;
    use super::unique_operation_types_rule::UniqueOperationTypesRule;
    use super::unique_type_names_rule::UniqueTypeNamesRule;
    use crate::subgraph::validation::validation_context::{
        create_simple_validation_context, SimpleRule, SimpleValidationContext,
    };

    fn run<R: for<'a> SimpleRule<'a>>(sdl: &str) -> Vec<String> {
        let doc = graphql_rs::parse(sdl).unwrap();
        let mut cx = create_simple_validation_context(&doc);
        let rule = R::new(&mut cx);
        let mut visitor = graphql_rs::visit_in_parallel(vec![Box::new(rule)]);
        graphql_rs::visit(&doc, &mut visitor, &mut cx);
        cx.collect_reported_errors()
            .into_iter()
            .map(|e| {
                assert_eq!(e.code(), Some("INVALID_GRAPHQL"));
                e.message
            })
            .collect()
    }

    #[test]
    fn known_directives_rule() {
        let errors = run::<KnownDirectivesRule>(
            r#"
            directive @a on FIELD_DEFINITION
            type Query @a { f(x: Int @a): Int @a @deprecated @b }
            input I { x: Int @a }
            "#,
        );
        assert_eq!(
            errors,
            vec![
                "Directive \"@a\" may not be used on OBJECT.",
                "Directive \"@a\" may not be used on ARGUMENT_DEFINITION.",
                "Unknown directive \"@b\".",
                "Directive \"@a\" may not be used on INPUT_FIELD_DEFINITION.",
            ]
        );
    }

    #[test]
    fn unique_argument_definition_names_rule_skips_type_children() {
        let errors = run::<UniqueArgumentDefinitionNamesRule>(
            r#"
            directive @d(a: Int, a: Int) on FIELD
            type Q { f(a: Int, a: String): Int }
            extend interface I { g(b: Int, b: Int, b: Int): Int }
            "#,
        );
        assert_eq!(
            errors,
            vec![
                "Argument \"@d(a:)\" can only be defined once.",
                "Argument \"Q.f(a:)\" can only be defined once.",
                "Argument \"I.g(b:)\" can only be defined once.",
                "Argument \"I.g(b:)\" can only be defined once.",
            ]
        );
    }

    #[test]
    fn unique_directives_per_location_rule() {
        let errors = run::<UniqueDirectivesPerLocationRule>(
            r#"
            directive @u on OBJECT | FIELD_DEFINITION
            directive @r repeatable on OBJECT
            type T @u @r @r { f: Int @u @u g: Int @u }
            extend type T @u
            "#,
        );
        assert_eq!(
            errors,
            vec![
                "The directive \"@u\" can only be used once at this location.",
                "The directive \"@u\" can only be used once at this location.",
            ]
        );
    }

    #[test]
    fn unique_input_field_names_rule_scopes_nested_objects() {
        let errors =
            run::<UniqueInputFieldNamesRule>(r#"type Q { f(a: In = { x: 1, y: { x: 1, z: 1 }, x: 2, z: 3 }): Int }"#);
        assert_eq!(errors, vec!["There can be only one input field named \"x\"."]);
    }

    #[test]
    fn only_interface_implementation_rule() {
        let errors = run::<OnlyInterfaceImplementationRule>(
            r#"
            type A implements B & C & D { a: Int }
            scalar B
            interface C { a: Int }
            extend type X implements A { a: Int }
            "#,
        );
        assert_eq!(
            errors,
            vec![
                "Cannot implement non-interface type B (of type ObjectType)",
                "Cannot implement non-interface type A (of type ObjectType)",
            ]
        );
    }

    #[test]
    fn schema_rules() {
        let sdl = "schema { query: Q } schema { query: Q mutation: M } extend schema { mutation: M }";
        assert_eq!(
            run::<LoneSchemaDefinitionRule>(sdl),
            vec!["Must provide only one schema definition."]
        );
        assert_eq!(
            run::<UniqueOperationTypesRule>(sdl),
            vec![
                "There can be only one query type in schema.",
                "There can be only one mutation type in schema.",
            ]
        );
    }

    #[test]
    fn unique_name_rules() {
        assert_eq!(
            run::<UniqueTypeNamesRule>(
                "type A { a: Int } scalar A extend type A { b: Int } enum B { X } input B { a: Int }"
            ),
            vec![
                "There can be only one type named \"A\".",
                "There can be only one type named \"B\"."
            ]
        );
        assert_eq!(
            run::<UniqueEnumValueNamesRule>("enum E { A B A } extend enum E { B }"),
            vec![
                "Enum value \"E.A\" can only be defined once.",
                "Enum value \"E.B\" can only be defined once."
            ]
        );
        assert_eq!(
            run::<UniqueFieldDefinitionNamesRule>(
                "type T { a: Int a: Int } extend type T { a: Int } input I { x: Int x: Int }"
            ),
            vec![
                "Field \"T.a\" can only be defined once.",
                "Field \"T.a\" can only be defined once.",
                "Field \"I.x\" can only be defined once.",
            ]
        );
        assert_eq!(
            run::<UniqueDirectiveNamesRule>("directive @a on FIELD directive @a on OBJECT"),
            vec!["There can be only one directive named \"@a\"."]
        );
        assert_eq!(
            run::<UniqueArgumentNamesRule>("type T @d(a: 1, a: 2) { f: Int @d(b: 1, b: 2) }"),
            vec![
                "There can be only one argument named \"a\".",
                "There can be only one argument named \"b\".",
            ]
        );
    }

    #[test]
    fn known_argument_names_on_directives_rule() {
        assert_eq!(
            run::<KnownArgumentNamesOnDirectivesRule>(
                "directive @d(a: Int) on FIELD_DEFINITION type Q { f: Int @d(a: 1, b: 2) @unknown(z: 1) }"
            ),
            vec!["Unknown argument \"b\" on directive \"@d\"."]
        );
    }

    /// Records `getSchemaCoordinate(ancestors)` for every directive.
    struct CoordinateRecorder(Vec<String>);

    impl<'a> Visitor<'a, SimpleValidationContext<'a>> for CoordinateRecorder {
        fn enter(
            &mut self,
            node: NodeRef<'a>,
            cx: &mut SimpleValidationContext<'a>,
            info: &VisitInfo<'a, '_>,
        ) -> VisitAction {
            if let NodeRef::Directive(_) = node {
                self.0.push(cx.get_schema_coordinate(info.ancestors));
            }
            VisitAction::Continue
        }
    }

    #[test]
    fn schema_coordinate() {
        let doc = graphql_rs::parse(
            r#"
            schema @a { query: Q }
            type Q @a { f(x: Int @a): Int @a }
            enum E { V @a }
            extend schema @a
            "#,
        )
        .unwrap();
        let mut cx = create_simple_validation_context(&doc);
        let mut recorder = CoordinateRecorder(Vec::new());
        graphql_rs::visit(&doc, &mut recorder, &mut cx);
        assert_eq!(recorder.0, vec!["schema", "Q", "Q.f.x", "Q.f", "E.V", "schema"]);
    }
}
