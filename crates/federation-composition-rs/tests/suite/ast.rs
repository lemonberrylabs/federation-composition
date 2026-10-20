//! Port of `__tests__/ast.spec.ts`.

use crate::shared::*;

use federation_composition_rs::subgraph::state::ArgumentKind;
use federation_composition_rs::supergraph::composition::ast::{
    create_enum_type_node, create_input_object_type_node, create_interface_type_node, create_join_graph_enum_type_node,
    create_object_type_node, create_scalar_type_node, create_schema_node, create_union_type_node, strip_federation,
    EnumTypeAst, EnumValueAst, FieldArgumentAst, FieldAst, InputFieldAst, InputObjectTypeAst, InterfaceTypeAst,
    JoinAst, JoinEnumValueAst, JoinFieldAst, JoinGraphAst, JoinImplementsAst, JoinTypeAst, JoinUnionMemberAst, LinkAst,
    LinkImportAst, ObjectTypeAst, ScalarTypeAst, SchemaAst, UnionTypeAst,
};
use graphql_rs::{ConstDirective, Definition};

fn create_directive(name: &str) -> ConstDirective {
    ConstDirective::new(name, vec![])
}

fn join_type(graph: &str) -> JoinTypeAst {
    JoinTypeAst {
        graph: graph.into(),
        ..Default::default()
    }
}

fn join_type_with_key(graph: &str, key: &str) -> JoinTypeAst {
    JoinTypeAst {
        graph: graph.into(),
        key: Some(key.into()),
        ..Default::default()
    }
}

fn field(name: &str, ty: &str) -> FieldAst {
    FieldAst {
        name: name.into(),
        r#type: ty.into(),
        ..Default::default()
    }
}

fn object(object_type: ObjectTypeAst) -> Definition {
    Definition::ObjectTypeDefinition(create_object_type_node(object_type))
}

fn interface(interface_type: InterfaceTypeAst) -> Definition {
    Definition::InterfaceTypeDefinition(create_interface_type_node(interface_type))
}

fn union(union_type: UnionTypeAst) -> Definition {
    Definition::UnionTypeDefinition(create_union_type_node(union_type))
}

fn input_object(input_object_type: InputObjectTypeAst) -> Definition {
    Definition::InputObjectTypeDefinition(create_input_object_type_node(input_object_type))
}

fn enum_type(enum_type: EnumTypeAst) -> Definition {
    Definition::EnumTypeDefinition(create_enum_type_node(enum_type))
}

fn scalar(scalar_type: ScalarTypeAst) -> Definition {
    Definition::ScalarTypeDefinition(create_scalar_type_node(scalar_type))
}

fn strip(source: &str) -> Document {
    strip_federation(&parse(source).unwrap())
}

mod object_type {
    use super::*;

    #[test]
    fn join_type_graph() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![field("name", "String")],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String
      }
    "#,
        );
    }

    #[test]
    fn join_implements_graph_interface() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "Book".into(),
                join: JoinAst {
                    r#type: vec![join_type("A")],
                    implements: vec![JoinImplementsAst {
                        graph: "A".into(),
                        interface: "Media".into(),
                    }],
                    ..Default::default()
                },
                fields: vec![field("title", "String")],
                ..Default::default()
            }),
            r#"
      type Book
        @join__type(graph: A)
        @join__implements(graph: A, interface: "Media") {
        title: String
      }
    "#,
        );
    }

    #[test]
    fn join_type_graph_key() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type_with_key("A", "id"), join_type_with_key("B", "id")],
                    ..Default::default()
                },
                fields: vec![field("id", "ID!"), field("name", "String")],
                ..Default::default()
            }),
            r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id") {
        id: ID!
        name: String
      }
    "#,
        );
    }

    #[test]
    fn join_type_graph_key_extension() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![
                        join_type_with_key("A", "id"),
                        JoinTypeAst {
                            extension: Some(true),
                            ..join_type_with_key("B", "id")
                        },
                    ],
                    ..Default::default()
                },
                fields: vec![field("id", "ID!"), field("name", "String")],
                ..Default::default()
            }),
            r#"
      type User
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id", extension: true) {
        id: ID!
        name: String
      }
    "#,
        );
    }

    #[test]
    fn join_field() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![
                    FieldAst {
                        join: JoinAst {
                            field: vec![JoinFieldAst::default()],
                            ..Default::default()
                        },
                        ..field("name", "String")
                    },
                    field("id", "ID"),
                ],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String @join__field
        id: ID
      }
    "#,
        );
    }

    #[test]
    fn join_field_graph() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![
                    FieldAst {
                        join: JoinAst {
                            field: vec![JoinFieldAst {
                                graph: Some("A".into()),
                                ..Default::default()
                            }],
                            ..Default::default()
                        },
                        ..field("name", "String")
                    },
                    field("id", "ID"),
                ],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String @join__field(graph: A)
        id: ID
      }
    "#,
        );
    }

    #[test]
    fn join_field_graph_type() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    join: JoinAst {
                        field: vec![
                            JoinFieldAst {
                                graph: Some("A".into()),
                                r#type: Some("String".into()),
                                ..Default::default()
                            },
                            JoinFieldAst {
                                graph: Some("B".into()),
                                r#type: Some("String!".into()),
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                    ..field("name", "String")
                }],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String
          @join__field(graph: B, type: "String!")
          @join__field(graph: A, type: "String")
      }
    "#,
        );
    }

    #[test]
    fn join_field_graph_override() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    join: JoinAst {
                        field: vec![JoinFieldAst {
                            graph: Some("B".into()),
                            r#override: Some("a".into()),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    ..field("name", "String")
                }],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String @join__field(graph: B, override: "a")
      }
    "#,
        );
    }

    #[test]
    fn join_field_graph_external() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    join: JoinAst {
                        field: vec![JoinFieldAst {
                            graph: Some("B".into()),
                            external: Some(true),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    ..field("name", "String")
                }],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String @join__field(graph: B, external: true)
      }
    "#,
        );
    }

    #[test]
    fn join_field_graph_provides() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    join: JoinAst {
                        field: vec![JoinFieldAst {
                            graph: Some("B".into()),
                            provides: Some("name".into()),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    ..field("name", "String")
                }],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String @join__field(graph: B, provides: "name")
      }
    "#,
        );
    }

    #[test]
    fn join_field_graph_requires() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    join: JoinAst {
                        field: vec![JoinFieldAst {
                            graph: Some("B".into()),
                            requires: Some("name".into()),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    ..field("name", "String")
                }],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String @join__field(graph: B, requires: "name")
      }
    "#,
        );
    }

    #[test]
    fn inaccessible() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    inaccessible: true,
                    ..field("name", "String")
                }],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @join__type(graph: B) {
        name: String @inaccessible
      }
    "#,
        );
    }

    #[test]
    fn field_arguments() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "Building".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    arguments: vec![FieldArgumentAst {
                        name: "units".into(),
                        r#type: "String!".into(),
                        kind: ArgumentKind::Scalar,
                        ..Default::default()
                    }],
                    ..field("height", "Int!")
                }],
                ..Default::default()
            }),
            r#"
      type Building @join__type(graph: A) @join__type(graph: B) {
        height(units: String!): Int!
      }
    "#,
        );
    }

    #[test]
    fn interfaces() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "Book".into(),
                interfaces: vec!["Media".into()],
                fields: vec![field("title", "String")],
                ..Default::default()
            }),
            r#"
      type Book implements Media {
        title: String
      }
    "#,
        );
    }

    #[test]
    fn default_value() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "Building".into(),
                fields: vec![FieldAst {
                    arguments: vec![FieldArgumentAst {
                        name: "units".into(),
                        r#type: "Int!".into(),
                        // `TypeKind.SCALAR` (graphql's), the same string as `ArgumentKind.SCALAR`.
                        kind: ArgumentKind::Scalar,
                        default_value: Some("1".into()),
                        ..Default::default()
                    }],
                    ..field("height", "Int!")
                }],
                ..Default::default()
            }),
            r#"
      type Building {
        height(units: Int! = 1): Int!
      }
    "#,
        );
    }

    #[test]
    fn a_directive_on_a_field() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    ast: vec![create_directive("lowercase")],
                    ..field("name", "String!")
                }],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) {
        name: String! @lowercase
      }
    "#,
        );
    }

    #[test]
    fn a_directive_on_an_object_type() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A")],
                    ..Default::default()
                },
                fields: vec![field("name", "String!")],
                ast: vec![create_directive("lowercase")],
                ..Default::default()
            }),
            r#"
      type User @join__type(graph: A) @lowercase {
        name: String!
      }
    "#,
        );
    }

    #[test]
    fn tag() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                fields: vec![FieldAst {
                    tags: vec!["public".into()],
                    arguments: vec![FieldArgumentAst {
                        name: "limit".into(),
                        r#type: "Int".into(),
                        kind: ArgumentKind::Scalar,
                        tags: vec!["public".into()],
                        ..Default::default()
                    }],
                    ..field("name", "String")
                }],
                tags: vec!["public".into()],
                ..Default::default()
            }),
            r#"
      type User @tag(name: "public") {
        name(limit: Int @tag(name: "public")): String @tag(name: "public")
      }
    "#,
        );
    }

    #[test]
    fn inaccessible_2() {
        to_equal_graphql(
            object(ObjectTypeAst {
                name: "User".into(),
                fields: vec![FieldAst {
                    inaccessible: true,
                    arguments: vec![FieldArgumentAst {
                        name: "limit".into(),
                        r#type: "Int".into(),
                        kind: ArgumentKind::Scalar,
                        inaccessible: true,
                        ..Default::default()
                    }],
                    ..field("name", "String")
                }],
                inaccessible: true,
                ..Default::default()
            }),
            r#"
      type User @inaccessible {
        name(limit: Int @inaccessible): String @inaccessible
      }
    "#,
        );
    }
}

mod interface_type {
    use super::*;

    #[test]
    fn join_type_graph() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![field("name", "String")],
                ..Default::default()
            }),
            r#"
      interface User @join__type(graph: A) @join__type(graph: B) {
        name: String
      }
    "#,
        );
    }

    #[test]
    fn join_type_graph_key_is_interface_object() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "Media".into(),
                join: JoinAst {
                    r#type: vec![
                        join_type_with_key("A", "id"),
                        JoinTypeAst {
                            is_interface_object: Some(true),
                            ..join_type_with_key("B", "id")
                        },
                    ],
                    ..Default::default()
                },
                fields: vec![field("id", "ID!")],
                ..Default::default()
            }),
            r#"
      interface Media
        @join__type(graph: A, key: "id")
        @join__type(graph: B, key: "id", isInterfaceObject: true) {
        id: ID!
      }
    "#,
        );
    }

    #[test]
    fn join_field_graph() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "User".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![
                    FieldAst {
                        join: JoinAst {
                            field: vec![JoinFieldAst {
                                graph: Some("A".into()),
                                ..Default::default()
                            }],
                            ..Default::default()
                        },
                        ..field("name", "String")
                    },
                    field("id", "ID"),
                ],
                ..Default::default()
            }),
            r#"
      interface User @join__type(graph: A) @join__type(graph: B) {
        name: String @join__field(graph: A)
        id: ID
      }
    "#,
        );
    }

    #[test]
    fn field_arguments() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "Building".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![FieldAst {
                    arguments: vec![FieldArgumentAst {
                        name: "units".into(),
                        r#type: "Int!".into(),
                        kind: ArgumentKind::Scalar,
                        ..Default::default()
                    }],
                    ..field("height", "Int!")
                }],
                ..Default::default()
            }),
            r#"
      interface Building @join__type(graph: A) @join__type(graph: B) {
        height(units: Int!): Int!
      }
    "#,
        );
    }

    #[test]
    fn interfaces() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "Book".into(),
                interfaces: vec!["Media".into()],
                fields: vec![field("title", "String")],
                ..Default::default()
            }),
            r#"
      interface Book implements Media {
        title: String
      }
    "#,
        );
    }

    #[test]
    fn default_value() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "Skyscraper".into(),
                interfaces: vec!["Building".into()],
                fields: vec![FieldAst {
                    arguments: vec![FieldArgumentAst {
                        name: "units".into(),
                        r#type: "Int!".into(),
                        kind: ArgumentKind::Scalar,
                        default_value: Some("1".into()),
                        ..Default::default()
                    }],
                    ..field("height", "Int!")
                }],
                ..Default::default()
            }),
            r#"
      interface Skyscraper implements Building {
        height(units: Int! = 1): Int!
      }
    "#,
        );
    }

    #[test]
    fn tag() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "Building".into(),
                fields: vec![FieldAst {
                    tags: vec!["public".into()],
                    arguments: vec![FieldArgumentAst {
                        name: "units".into(),
                        r#type: "Int!".into(),
                        kind: ArgumentKind::Scalar,
                        tags: vec!["public".into()],
                        ..Default::default()
                    }],
                    ..field("height", "Int!")
                }],
                tags: vec!["public".into()],
                ..Default::default()
            }),
            r#"
      interface Building @tag(name: "public") {
        height(units: Int! @tag(name: "public")): Int! @tag(name: "public")
      }
    "#,
        );
    }

    #[test]
    fn inaccessible() {
        to_equal_graphql(
            interface(InterfaceTypeAst {
                name: "User".into(),
                fields: vec![FieldAst {
                    inaccessible: true,
                    arguments: vec![FieldArgumentAst {
                        name: "limit".into(),
                        r#type: "Int".into(),
                        kind: ArgumentKind::Scalar,
                        inaccessible: true,
                        ..Default::default()
                    }],
                    ..field("name", "String")
                }],
                inaccessible: true,
                ..Default::default()
            }),
            r#"
      interface User @inaccessible {
        name(limit: Int @inaccessible): String @inaccessible
      }
    "#,
        );
    }
}

mod union_type {
    use super::*;

    #[test]
    fn join_type_graph_join_union_member_graph_member() {
        let union_member = |graph: &str, member: &str| JoinUnionMemberAst {
            graph: graph.into(),
            member: member.into(),
        };
        to_equal_graphql(
            union(UnionTypeAst {
                name: "Media".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    union_member: vec![
                        union_member("A", "Book"),
                        union_member("B", "Book"),
                        union_member("B", "Movie"),
                    ],
                    ..Default::default()
                },
                members: vec!["Book".into(), "Movie".into()],
                ..Default::default()
            }),
            r#"
      union Media
        @join__type(graph: A)
        @join__type(graph: B)
        @join__unionMember(graph: A, member: "Book")
        @join__unionMember(graph: B, member: "Movie")
        @join__unionMember(graph: B, member: "Book") =
        | Movie
        | Book
    "#,
        );
    }

    #[test]
    fn tag() {
        to_equal_graphql(
            union(UnionTypeAst {
                name: "Media".into(),
                members: vec!["Book".into(), "Movie".into()],
                tags: vec!["public".into()],
                ..Default::default()
            }),
            r#"
      union Media @tag(name: "public") = Movie | Book
    "#,
        );
    }

    #[test]
    fn inaccessible() {
        to_equal_graphql(
            union(UnionTypeAst {
                name: "Media".into(),
                members: vec!["Book".into(), "Movie".into()],
                inaccessible: true,
                ..Default::default()
            }),
            r#"
      union Media @inaccessible = Movie | Book
    "#,
        );
    }

    #[test]
    fn directives() {
        to_equal_graphql(
            union(UnionTypeAst {
                name: "Media".into(),
                members: vec!["Book".into(), "Movie".into()],
                ast: vec![create_directive("custom")],
                ..Default::default()
            }),
            r#"
      union Media @custom = Movie | Book
    "#,
        );
    }
}

mod input_object_type {
    use super::*;

    fn input_field(name: &str, ty: &str) -> InputFieldAst {
        InputFieldAst {
            name: name.into(),
            r#type: ty.into(),
            ..Default::default()
        }
    }

    #[test]
    fn join_type_graph() {
        to_equal_graphql(
            input_object(InputObjectTypeAst {
                name: "UserInput".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                fields: vec![input_field("name", "String!")],
                ..Default::default()
            }),
            r#"
      input UserInput @join__type(graph: A) @join__type(graph: B) {
        name: String!
      }
    "#,
        );
    }

    #[test]
    fn inaccessible() {
        to_equal_graphql(
            input_object(InputObjectTypeAst {
                name: "User".into(),
                fields: vec![InputFieldAst {
                    inaccessible: true,
                    ..input_field("name", "String")
                }],
                inaccessible: true,
                ..Default::default()
            }),
            r#"
      input User @inaccessible {
        name: String @inaccessible
      }
    "#,
        );
    }

    #[test]
    fn tag() {
        to_equal_graphql(
            input_object(InputObjectTypeAst {
                name: "UserInput".into(),
                fields: vec![InputFieldAst {
                    tags: vec!["public".into()],
                    ..input_field("name", "String!")
                }],
                tags: vec!["public".into()],
                ..Default::default()
            }),
            r#"
      input UserInput @tag(name: "public") {
        name: String! @tag(name: "public")
      }
    "#,
        );
    }

    #[test]
    fn directive() {
        to_equal_graphql(
            input_object(InputObjectTypeAst {
                name: "User".into(),
                fields: vec![InputFieldAst {
                    ast: vec![create_directive("custom")],
                    ..input_field("name", "String")
                }],
                ast: vec![create_directive("custom")],
                ..Default::default()
            }),
            r#"
      input User @custom {
        name: String @custom
      }
    "#,
        );
    }

    #[test]
    fn default_value() {
        to_equal_graphql(
            input_object(InputObjectTypeAst {
                name: "Filter".into(),
                fields: vec![
                    InputFieldAst {
                        default_value: Some("2".into()),
                        ..input_field("limit", "Int")
                    },
                    InputFieldAst {
                        default_value: Some("{limit: 1}".into()),
                        ..input_field("obj", "Obj")
                    },
                ],
                ..Default::default()
            }),
            r#"
      input Filter {
        obj: Obj = { limit: 1 }
        limit: Int = 2
      }
    "#,
        );
    }
}

mod enum_object_type {
    use super::*;

    fn enum_value(name: &str) -> EnumValueAst {
        EnumValueAst {
            name: name.into(),
            ..Default::default()
        }
    }

    fn join_enum_value(graph: &str) -> JoinEnumValueAst {
        JoinEnumValueAst { graph: graph.into() }
    }

    #[test]
    fn join_type_graph_join_enum_value_graph() {
        to_equal_graphql(
            enum_type(EnumTypeAst {
                name: "UserType".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                values: vec![
                    EnumValueAst {
                        join: JoinAst {
                            enum_value: vec![join_enum_value("A"), join_enum_value("B")],
                            ..Default::default()
                        },
                        ..enum_value("ADMIN")
                    },
                    EnumValueAst {
                        join: JoinAst {
                            enum_value: vec![join_enum_value("B")],
                            ..Default::default()
                        },
                        ..enum_value("REGULAR")
                    },
                ],
                ..Default::default()
            }),
            r#"
      enum UserType @join__type(graph: B) @join__type(graph: A) {
        REGULAR @join__enumValue(graph: B)
        ADMIN @join__enumValue(graph: A) @join__enumValue(graph: B)
      }
    "#,
        );
    }

    #[test]
    fn tag() {
        to_equal_graphql(
            enum_type(EnumTypeAst {
                name: "UserType".into(),
                tags: vec!["public".into()],
                values: vec![
                    EnumValueAst {
                        tags: vec!["public".into()],
                        ..enum_value("ADMIN")
                    },
                    EnumValueAst {
                        tags: vec!["public".into()],
                        ..enum_value("REGULAR")
                    },
                ],
                ..Default::default()
            }),
            r#"
      enum UserType @tag(name: "public") {
        REGULAR @tag(name: "public")
        ADMIN @tag(name: "public")
      }
    "#,
        );
    }

    #[test]
    fn inaccessible() {
        to_equal_graphql(
            enum_type(EnumTypeAst {
                name: "Media".into(),
                values: vec![
                    enum_value("BOOK"),
                    EnumValueAst {
                        inaccessible: true,
                        ..enum_value("MOVIE")
                    },
                ],
                inaccessible: true,
                ..Default::default()
            }),
            r#"
      enum Media @inaccessible {
        BOOK
        MOVIE @inaccessible
      }
    "#,
        );
    }

    #[test]
    fn directives() {
        to_equal_graphql(
            enum_type(EnumTypeAst {
                name: "Media".into(),
                values: vec![
                    EnumValueAst {
                        ast: vec![create_directive("any")],
                        ..enum_value("BOOK")
                    },
                    enum_value("MOVIE"),
                ],
                ast: vec![create_directive("custom")],
                ..Default::default()
            }),
            r#"
      enum Media @custom {
        BOOK @any
        MOVIE
      }
    "#,
        );
    }
}

mod schema {
    use super::*;

    #[test]
    fn operation_types() {
        to_equal_graphql(
            Definition::SchemaDefinition(create_schema_node(&SchemaAst {
                query: Some("Query".into()),
                mutation: Some("Mutation".into()),
                subscription: Some("Subscription".into()),
                ..Default::default()
            })),
            r#"
      schema {
        query: Query
        mutation: Mutation
        subscription: Subscription
      }
    "#,
        );
    }

    #[test]
    fn links() {
        to_equal_graphql(
            Definition::SchemaDefinition(create_schema_node(&SchemaAst {
                query: Some("Query".into()),
                links: vec![
                    LinkAst {
                        url: "https://specs.apollo.dev/link/v1.0".into(),
                        ..Default::default()
                    },
                    LinkAst {
                        url: "https://specs.apollo.dev/join/v0.3".into(),
                        r#for: Some("EXECUTION".into()),
                        ..Default::default()
                    },
                    LinkAst {
                        url: "https://myspecs.dev/lowercase/v1.0".into(),
                        import: Some(vec![LinkImportAst {
                            name: "@lowercase".into(),
                            alias: None,
                        }]),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            })),
            r#"
      schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION)
        @link(
          url: "https://myspecs.dev/lowercase/v1.0"
          import: ["@lowercase"]
        ) {
        query: Query
      }
    "#,
        );
    }
}

mod join_graph_enum_type {
    use super::*;

    #[test]
    fn join_graph() {
        to_equal_graphql(
            Definition::EnumTypeDefinition(create_join_graph_enum_type_node(&[
                JoinGraphAst {
                    name: "products".into(),
                    url: Some("http://products.com".into()),
                    enum_value: "PRODUCTS".into(),
                },
                JoinGraphAst {
                    name: "reviews".into(),
                    url: None,
                    enum_value: "REVIEWS".into(),
                },
            ])),
            r#"
      enum join__Graph {
        PRODUCTS @join__graph(name: "products", url: "http://products.com")
        REVIEWS @join__graph(name: "reviews", url: "")
      }
    "#,
        );
    }
}

mod scalars {
    use super::*;

    #[test]
    fn scalar_and_directive_on_a_scalar() {
        to_equal_graphql(
            scalar(ScalarTypeAst {
                name: "MyScalar".into(),
                join: JoinAst {
                    r#type: vec![join_type("A"), join_type("B")],
                    ..Default::default()
                },
                ast: vec![create_directive("custom")],
                ..Default::default()
            }),
            r#"
      scalar MyScalar @custom @join__type(graph: A) @join__type(graph: B)
    "#,
        );
    }

    #[test]
    fn inaccessible() {
        to_equal_graphql(
            scalar(ScalarTypeAst {
                name: "MyScalar".into(),
                inaccessible: true,
                ..Default::default()
            }),
            r#"
      scalar MyScalar @inaccessible
    "#,
        );
    }
}

mod strip_federation {
    use super::*;

    #[test]
    fn join_graph() {
        to_equal_graphql(
            strip(
                r#"
          directive @join__graph on ENUM_VALUE
          directive @lowercase on ENUM_VALUE

          enum Foo {
            PANDAS @join__graph(name: "pandas", url: "") @lowercase
            USERS @join__graph(name: "users", url: "https://users.com")
          }
        "#,
            ),
            r#"
      directive @lowercase on ENUM_VALUE

      enum Foo {
        PANDAS @lowercase
        USERS
      }
    "#,
        );
    }

    #[test]
    fn join_enum_value() {
        to_equal_graphql(
            strip(
                r#"
          directive @join__enumValue on ENUM_VALUE

          enum Foo {
            PANDAS @join__enumValue(graph: B)
            USERS @join__enumValue(graph: C)
          }
        "#,
            ),
            r#"
      enum Foo {
        PANDAS
        USERS
      }
    "#,
        );
    }

    #[test]
    fn join_union_member() {
        to_equal_graphql(
            strip(
                r#"
          directive @join__unionMember on ENUM_VALUE

          enum Foo {
            PANDAS @join__unionMember(graph: B)
            USERS @join__unionMember(graph: C)
          }
        "#,
            ),
            r#"
      enum Foo {
        PANDAS
        USERS
      }
    "#,
        );
    }

    #[test]
    fn join_field() {
        to_equal_graphql(
            strip(
                r#"
          directive @join__field on ENUM_VALUE

          enum Foo {
            PANDAS @join__field(graph: B)
            USERS @join__field(graph: C)
          }
        "#,
            ),
            r#"
      enum Foo {
        PANDAS
        USERS
      }
    "#,
        );
    }

    #[test]
    fn join_implements() {
        to_equal_graphql(
            strip(
                r#"
          directive @join__implements on ENUM_VALUE

          enum Foo {
            PANDAS @join__implements(graph: B)
            USERS @join__implements(graph: C)
          }
        "#,
            ),
            r#"
      enum Foo {
        PANDAS
        USERS
      }
    "#,
        );
    }

    #[test]
    fn join_type() {
        to_equal_graphql(
            strip(
                r#"
          directive @join__type on ENUM_VALUE

          type Foo @join__type {
            name: String
          }
        "#,
            ),
            r#"
      type Foo {
        name: String
      }
    "#,
        );
    }

    #[test]
    fn link() {
        to_equal_graphql(
            strip(
                r#"
          schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) {
            query: Query
          }

          directive @link(
            url: String
            as: String
            for: link__Purpose
            import: [link__Import]
          ) repeatable on SCHEMA
        "#,
            ),
            r#"
      schema {
        query: Query
      }
    "#,
        );
    }

    #[test]
    fn tag() {
        to_equal_graphql(
            strip(
                r#"
          directive @tag on ENUM_VALUE

          type Foo @tag {
            name: String
          }
        "#,
            ),
            r#"
      type Foo {
        name: String
      }
    "#,
        );
    }

    #[test]
    fn enum_link_purpose() {
        to_equal_graphql(
            strip(
                r#"
          enum link__Purpose {
            EXECUTION
            DOCUMENTATION
          }

          enum Foo {
            BAR
          }
        "#,
            ),
            r#"
      enum Foo {
        BAR
      }
    "#,
        );
    }

    #[test]
    fn scalar_link_import() {
        to_equal_graphql(
            strip(
                r#"
          scalar link__Import
          scalar JSON
        "#,
            ),
            r#"
      scalar JSON
    "#,
        );
    }

    #[test]
    fn enum_join_graph() {
        to_equal_graphql(
            strip(
                r#"
          enum join__Graph {
            EXECUTION
            DOCUMENTATION
          }

          enum Foo {
            BAR
          }
        "#,
            ),
            r#"
      enum Foo {
        BAR
      }
    "#,
        );
    }

    #[test]
    fn scalar_join_field_set() {
        to_equal_graphql(
            strip(
                r#"
          scalar join__FieldSet
          scalar JSON
        "#,
            ),
            r#"
      scalar JSON
    "#,
        );
    }

    #[test]
    fn strip_inaccessible() {
        to_equal_graphql(
            strip(
                r#"
          schema {
            query: Query
          }

          type Panda {
            name: String!
            favoriteFood: String @inaccessible
          }

          type Profile @inaccessible {
            name: String
          }

          type Query {
            allPandas: [Panda] @inaccessible
            users(filter: Filter, anotherFilter: AnotherFilter): [User]
            panda(name: String!): Panda
          }

          input Filter @inaccessible {
            limit: Int
          }

          input AnotherFilter {
            limit: Int @inaccessible
            after: ID
          }

          enum Role @inaccessible {
            ADMIN
            USER
          }

          enum Access {
            READ
            WRITE @inaccessible
          }

          type User {
            id: ID!
            name: String
            totalProductsCreated: Int @inaccessible
            profile: Profile
            role: Role
            access: Access
          }
        "#,
            ),
            r#"
      schema {
        query: Query
      }

      type Panda {
        name: String!
      }

      input AnotherFilter {
        after: ID
      }

      type Query {
        panda(name: String!): Panda
        users(anotherFilter: AnotherFilter): [User]
      }

      enum Access {
        READ
      }

      type User {
        id: ID!
        name: String
        access: Access
      }
    "#,
        );
    }
}
