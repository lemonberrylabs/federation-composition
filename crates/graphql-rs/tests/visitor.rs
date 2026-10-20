//! Visitor API checks beyond the differential corpus: that visitors can keep
//! references into the document (as the library's `TypeNodeInfo` does), and
//! how a context is shared by visitors running in parallel.

use graphql_rs::*;

/// A port of the library's `src/graphql/type-node-info.ts`, as a design check
/// of the visitor API: it keeps `&'a` references to nodes of the document.
#[derive(Default)]
struct TypeNodeInfo<'a> {
    ty: Option<NodeRef<'a>>,
    field: Option<NodeRef<'a>>,
    arg: Option<&'a InputValueDefinition>,
    value: Option<&'a EnumValueDefinition>,
}

impl<'a> TypeNodeInfo<'a> {
    fn enter(&mut self, node: NodeRef<'a>) {
        match node {
            NodeRef::ObjectTypeDefinition(_)
            | NodeRef::ObjectTypeExtension(_)
            | NodeRef::InterfaceTypeDefinition(_)
            | NodeRef::InterfaceTypeExtension(_)
            | NodeRef::UnionTypeDefinition(_)
            | NodeRef::UnionTypeExtension(_)
            | NodeRef::EnumTypeDefinition(_)
            | NodeRef::EnumTypeExtension(_)
            | NodeRef::InputObjectTypeDefinition(_)
            | NodeRef::InputObjectTypeExtension(_)
            | NodeRef::ScalarTypeDefinition(_)
            | NodeRef::ScalarTypeExtension(_)
            | NodeRef::DirectiveDefinition(_) => self.ty = Some(node),
            NodeRef::EnumValueDefinition(v) => self.value = Some(v),
            NodeRef::FieldDefinition(_) => self.field = Some(node),
            NodeRef::InputValueDefinition(v) => {
                if self.field.is_some() {
                    self.arg = Some(v);
                } else {
                    self.field = Some(node);
                }
            }
            _ => {}
        }
    }

    fn leave(&mut self, node: NodeRef<'a>) {
        match node {
            NodeRef::ObjectTypeDefinition(_)
            | NodeRef::ObjectTypeExtension(_)
            | NodeRef::InterfaceTypeDefinition(_)
            | NodeRef::InterfaceTypeExtension(_)
            | NodeRef::UnionTypeDefinition(_)
            | NodeRef::UnionTypeExtension(_)
            | NodeRef::EnumTypeDefinition(_)
            | NodeRef::EnumTypeExtension(_)
            | NodeRef::InputObjectTypeDefinition(_)
            | NodeRef::InputObjectTypeExtension(_)
            | NodeRef::ScalarTypeDefinition(_)
            | NodeRef::ScalarTypeExtension(_)
            | NodeRef::DirectiveDefinition(_) => self.ty = None,
            NodeRef::FieldDefinition(_) => self.field = None,
            NodeRef::EnumValueDefinition(_) => self.value = None,
            NodeRef::InputValueDefinition(_) => {
                if self.arg.is_some() {
                    self.arg = None;
                } else {
                    self.field = None;
                }
            }
            _ => {}
        }
    }
}

/// The shared context of the "rules": the type info plus what they report.
#[derive(Default)]
struct Context<'a> {
    type_info: TypeNodeInfo<'a>,
    reports: Vec<String>,
    deprecated_fields: Vec<&'a FieldDefinition>,
}

/// The library's `visitWithTypeNodeInfo`.
struct WithTypeNodeInfo<V>(V);

impl<'a, V: Visitor<'a, Context<'a>>> Visitor<'a, Context<'a>> for WithTypeNodeInfo<V> {
    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut Context<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        cx.type_info.enter(node);
        let result = self.0.enter(node, cx, info);
        if result != VisitAction::Continue {
            cx.type_info.leave(node);
        }
        result
    }

    fn leave(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut Context<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let result = self.0.leave(node, cx, info);
        cx.type_info.leave(node);
        result
    }
}

/// Reports every directive with the coordinate TypeNodeInfo tracks.
struct DirectiveCoordinates;

impl<'a> Visitor<'a, Context<'a>> for DirectiveCoordinates {
    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut Context<'a>,
        _: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        if let NodeRef::Directive(d) = node {
            let ti = &cx.type_info;
            let mut coordinate = ti
                .ty
                .and_then(|t| t.name())
                .map(|n| n.value.clone())
                .unwrap_or_default();
            if let Some(field) = ti.field.and_then(|f| f.name()) {
                coordinate = format!("{coordinate}.{}", field.value);
            }
            if let Some(arg) = ti.arg {
                coordinate = format!("{coordinate}({}:)", arg.name.value);
            }
            if let Some(value) = ti.value {
                coordinate = format!("{coordinate}.{}", value.name.value);
            }
            cx.reports
                .push(format!("@{} on {coordinate}", d.name.value));
        }
        VisitAction::Continue
    }
}

/// Keeps references to deprecated fields, beyond the visit.
struct CollectDeprecated;

impl<'a> Visitor<'a, Context<'a>> for CollectDeprecated {
    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut Context<'a>,
        _: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        if let NodeRef::FieldDefinition(f) = node {
            if f.directives.iter().any(|d| d.name.value == "deprecated") {
                cx.deprecated_fields.push(f);
            }
        }
        VisitAction::Continue
    }
}

#[test]
fn type_node_info_keeps_references() {
    let doc = parse(
        r#"
        type Query @key(fields: "id") {
          a(x: Int @tag(name: "x")): Int @deprecated
          b: Int
        }
        enum E { V @tag(name: "v") }
        input I { f: Int @tag(name: "f") }
        directive @d(arg: Int @tag(name: "arg")) on FIELD
        "#,
    )
    .unwrap();
    let mut cx = Context::default();
    let mut rules = WithTypeNodeInfo(visit_in_parallel(vec![
        Box::new(DirectiveCoordinates),
        Box::new(CollectDeprecated),
    ]));
    visit(&doc, &mut rules, &mut cx);
    assert_eq!(
        cx.reports,
        [
            "@key on Query",
            "@tag on Query.a(x:)",
            "@deprecated on Query.a",
            "@tag on E.V",
            "@tag on I.f",
            "@tag on d.arg",
        ]
    );
    let deprecated: Vec<&str> = cx
        .deprecated_fields
        .iter()
        .map(|f| f.name.value.as_str())
        .collect();
    assert_eq!(deprecated, ["a"]);
    assert!(cx.type_info.ty.is_none() && cx.type_info.field.is_none());
}

#[test]
fn ancestors_give_schema_coordinates() {
    // The library's `schemaCoordinate(paths, ancestors)` from
    // supergraph/composition/ast.ts.
    fn schema_coordinate(path: &[PathKey], ancestors: &[Ancestor<'_>]) -> String {
        let mut coordinate = String::new();
        for i in 0..path.len().max(ancestors.len()) {
            if let (Some(PathKey::Index(index)), Some(Ancestor::List(list))) =
                (path.get(i), ancestors.get(i))
            {
                let name = list.get(*index).unwrap().name().unwrap().value.clone();
                if coordinate.is_empty() {
                    coordinate = name;
                } else {
                    coordinate = format!("{coordinate}.{name}");
                }
            }
        }
        coordinate
    }

    struct Inaccessible(Vec<String>);
    impl EditVisitor for Inaccessible {
        fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
            if let NodeRef::Directive(d) = node {
                if d.name.value == "inaccessible" {
                    self.0.push(schema_coordinate(info.path, info.ancestors));
                }
            }
            EditAction::Continue
        }
    }

    let doc = parse(
        "type A { f: Int @inaccessible g(x: Int @inaccessible): Int } enum E { V @inaccessible } scalar S @inaccessible",
    )
    .unwrap();
    let mut v = Inaccessible(Vec::new());
    visit_edit(&doc, &mut v);
    assert_eq!(v.0, ["A.f", "A.g.x", "E.V", "S"]);
}

#[test]
fn edit_result_conversions() {
    let doc = parse("type A @a { f: Int @b } type B { g: Int }").unwrap();

    struct DropB;
    impl EditVisitor for DropB {
        fn enter(&mut self, node: NodeRef<'_>, _: &VisitInfo<'_, '_>) -> EditAction {
            match node {
                NodeRef::ObjectTypeDefinition(o) if o.name.value == "B" => EditAction::Delete,
                NodeRef::Directive(_) => EditAction::Delete,
                _ => EditAction::Continue,
            }
        }
    }
    let out = visit_edit(&doc, &mut DropB).into_document();
    assert_eq!(print(&out), "type A {\n  f: Int\n}");
    // The input is untouched.
    assert_eq!(doc.definitions.len(), 2);

    struct DeleteRoot;
    impl EditVisitor for DeleteRoot {
        fn enter(&mut self, _: NodeRef<'_>, _: &VisitInfo<'_, '_>) -> EditAction {
            EditAction::Delete
        }
    }
    assert_eq!(visit_edit(&doc, &mut DeleteRoot), EditResult::Deleted);
}

#[test]
#[should_panic(expected = "cannot delete required field Field.name")]
fn deleting_a_required_field_panics() {
    struct DeleteNames;
    impl EditVisitor for DeleteNames {
        fn enter(&mut self, node: NodeRef<'_>, _: &VisitInfo<'_, '_>) -> EditAction {
            match node {
                NodeRef::Name(_) => EditAction::Delete,
                _ => EditAction::Continue,
            }
        }
    }
    let doc = parse("{ a }").unwrap();
    visit_edit(&doc, &mut DeleteNames);
}

#[test]
fn concat_ast_and_predicates() {
    let a = parse("type A { a: Int }").unwrap();
    let b = parse("extend type A { b: Int } query { a }").unwrap();
    let both = concat_ast([&a, &b]);
    assert_eq!(both.definitions.len(), 3);
    assert!(is_type_definition_node(&both.definitions[0]));
    assert!(is_type_extension_node(&both.definitions[1]));
    assert!(is_type_system_extension_node(&both.definitions[1]));
    assert!(is_executable_definition_node(&both.definitions[2]));
    assert!(both.definitions.iter().all(is_definition_node));
    assert!(!is_type_system_definition_node(&both.definitions[2]));
    assert!(is_const_value_node(&parse_value("[1, $a]").unwrap()));
    assert!(!is_const_value_node(&parse_value("[]").unwrap()));
    assert!(!is_const_value_node(&parse_value("{ a: $b }").unwrap()));
    assert!(is_type_node(&parse_type("[A!]").unwrap()));
}

#[test]
fn errors_serialize_like_graphql_js() {
    let e = parse("{").unwrap_err();
    assert_eq!(e.to_string(), "Syntax Error: Expected Name, found <EOF>.");
    assert_eq!(
        serde_json::to_string(&e).unwrap(),
        r#"{"message":"Syntax Error: Expected Name, found <EOF>."}"#
    );
    let e = GraphQLError::with_code("Bad.", "INVALID_GRAPHQL");
    assert_eq!(e.code(), Some("INVALID_GRAPHQL"));
    assert_eq!(
        serde_json::to_string(&e).unwrap(),
        r#"{"message":"Bad.","extensions":{"code":"INVALID_GRAPHQL"}}"#
    );
}
