//! Port of `__tests__/graphql/sort-sdl.spec.ts`.

use crate::shared::*;

use graphql_rs::{Definition, Directive};

/// `sortSDL(document)`.
fn sort(document: &Document) -> Document {
    sort_sdl(document).unwrap()
}

/// `(definition as ObjectTypeDefinitionNode).name.value`, for a definition of any named kind.
fn definition_name(definition: &Definition) -> &str {
    match definition {
        Definition::ObjectTypeDefinition(d) => &d.name.value,
        Definition::ScalarTypeDefinition(d) => &d.name.value,
        Definition::InterfaceTypeDefinition(d) => &d.name.value,
        Definition::EnumTypeDefinition(d) => &d.name.value,
        Definition::UnionTypeDefinition(d) => &d.name.value,
        other => panic!("unexpected definition {other:?}"),
    }
}

/// `(definition as ObjectTypeDefinitionNode).directives`: the TS reads `directives` through the
/// cast whatever the actual kind is.
fn definition_directives(definition: &Definition) -> &Vec<Directive> {
    match definition {
        Definition::ObjectTypeDefinition(d) => &d.directives,
        Definition::InterfaceTypeDefinition(d) => &d.directives,
        Definition::EnumTypeDefinition(d) => &d.directives,
        Definition::UnionTypeDefinition(d) => &d.directives,
        other => panic!("unexpected definition {other:?}"),
    }
}

/// `(definition as ObjectTypeDefinitionNode).fields`, for object and interface types.
fn definition_fields(definition: &Definition) -> &Vec<graphql_rs::FieldDefinition> {
    match definition {
        Definition::ObjectTypeDefinition(d) => &d.fields,
        Definition::InterfaceTypeDefinition(d) => &d.fields,
        other => panic!("unexpected definition {other:?}"),
    }
}

/// `first.directives ? first.directives[0] : null` followed by `firstDirective?.name.value`.
fn first_directive_name(directives: &[Directive]) -> Option<&str> {
    directives.first().map(|d| d.name.value.as_str())
}

// TS: the spec file's top-level `describe` block has the file's name.
#[allow(clippy::module_inception)]
mod sort_sdl {
    use super::*;

    mod object_types {
        use super::*;

        #[test]
        fn should_sort_object_type_definitions() {
            let document = parse(
                r#"
              type C
              type B
              type A
            "#,
            )
            .unwrap();
            let result = sort(&document);
            assert_eq!(definition_name(&result.definitions[0]), "A");
        }

        #[test]
        fn should_sort_fields_for_object_types() {
            let document = parse(
                r#"
              type A {
                c: String
                b: String
                a: String
              }
            "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            let first_field = definition_fields(first).first();
            assert_eq!(first_field.map(|f| f.name.value.as_str()), Some("a"));
        }

        #[test]
        fn should_sort_directives_for_object_types() {
            let document = parse(
                r#"
              type A @c @b @a
            "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            assert_eq!(first_directive_name(definition_directives(first)), Some("a"));
        }

        #[test]
        fn should_sort_interfaces_for_object_types() {
            let document = parse(
                r#"
                interface C
                interface B
                interface A
                type D implements C & B & A
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = result
                .definitions
                .iter()
                .find_map(|d| match d {
                    Definition::ObjectTypeDefinition(d) => Some(d),
                    _ => None,
                })
                .unwrap();
            let first_interface = first.interfaces.first();
            assert_eq!(first_interface.map(|i| i.name.value.as_str()), Some("A"));
        }
    }

    mod scalars {
        use super::*;

        #[test]
        fn should_sort_scalars() {
            let document = parse(
                r#"
          scalar C
          scalar B
          scalar A
        "#,
            )
            .unwrap();
            let result = sort(&document);
            assert_eq!(definition_name(&result.definitions[0]), "A");
        }
    }

    mod interface_type_definitions {
        use super::*;

        #[test]
        fn should_sort_directives_for_interface_types() {
            let document = parse(
                r#"
                interface A @c @b @a
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            assert_eq!(first_directive_name(definition_directives(first)), Some("a"));
        }

        #[test]
        fn should_sort_fields_for_interface_types() {
            let document = parse(
                r#"
        interface A {
            c: String
            b: String
            a: String
        }
      "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            let first_field = definition_fields(first).first();
            assert_eq!(first_field.map(|f| f.name.value.as_str()), Some("a"));
        }
    }

    mod enum_type_definitions {
        use super::*;

        #[test]
        fn should_sort_directives_for_enum_types() {
            let document = parse(
                r#"
                enum A @c @b @a
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            assert_eq!(first_directive_name(definition_directives(first)), Some("a"));
        }

        #[test]
        fn should_sort_values_for_enum_types() {
            let document = parse(
                r#"
        enum A {
            c
            b
            a
        }
      "#,
            )
            .unwrap();
            let result = sort(&document);
            let Definition::EnumTypeDefinition(first) = &result.definitions[0] else {
                panic!("expected an enum type definition");
            };
            let first_value = first.values.first();
            assert_eq!(first_value.map(|v| v.name.value.as_str()), Some("a"));
        }
    }

    mod enum_value_definitions {
        use super::*;

        #[test]
        fn should_sort_directives_for_enum_values() {
            let document = parse(
                r#"
                enum A {
                    a @c @b @a
                }
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let Definition::EnumTypeDefinition(first) = &result.definitions[0] else {
                panic!("expected an enum type definition");
            };
            let first_value = first.values.first();
            let first_directive = first_value.and_then(|v| first_directive_name(&v.directives));
            assert_eq!(first_directive, Some("a"));
        }
    }

    mod union_type_definitions {
        use super::*;

        #[test]
        fn should_sort_directives_for_union_types() {
            let document = parse(
                r#"
                union A @c @b @a
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            assert_eq!(first_directive_name(definition_directives(first)), Some("a"));
        }

        #[test]
        fn should_sort_types_for_union_types() {
            let document = parse(
                r#"
        union A = C | B | A
      "#,
            )
            .unwrap();
            let result = sort(&document);
            let Definition::UnionTypeDefinition(first) = &result.definitions[0] else {
                panic!("expected a union type definition");
            };
            let first_value = first.types.first();
            assert_eq!(first_value.map(|t| t.name.value.as_str()), Some("A"));
        }
    }

    mod fields {
        use super::*;

        #[test]
        fn should_sort_directives_for_fields() {
            let document = parse(
                r#"
                type A {
                    a: String @c @b @a
                }
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            let first_field = definition_fields(first).first();
            let first_directive = first_field.and_then(|f| first_directive_name(&f.directives));
            assert_eq!(first_directive, Some("a"));
        }
    }

    mod directive_definitions {
        use super::*;

        #[test]
        fn should_sort_locations_for_directive_definitions() {
            let document = parse(
                r#"
                directive @directive on OBJECT | FIELD_DEFINITION | INTERFACE
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let Definition::DirectiveDefinition(first) = &result.definitions[0] else {
                panic!("expected a directive definition");
            };
            let first_location = first.locations.first();
            assert_eq!(first_location.map(|l| l.value.as_str()), Some("FIELD_DEFINITION"));
        }
    }

    mod directives {
        use super::*;

        #[test]
        fn should_sort_arguments_for_directives() {
            let document = parse(
                r#"
                type A {
                    a: String @directive(c: 1 b: 2 a: 3)
                }
                "#,
            )
            .unwrap();
            let result = sort(&document);
            let first = &result.definitions[0];
            let first_field = definition_fields(first).first();
            let first_directive = first_field.and_then(|f| f.directives.first());
            let first_argument = first_directive.and_then(|d| d.arguments.first());
            assert_eq!(first_argument.map(|a| a.name.value.as_str()), Some("a"));
        }
    }
}
