//! The public accessors of a built schema's type system: root types, the type map, each kind of
//! named type, fields, arguments and their defaults, enum values, input fields, directives and
//! deprecation. (`schema_differential.rs` compares the same accessors with graphql-js over the
//! schema corpus.)

use graphql_rs::{
    build_schema, print, validate_schema, DirectiveLocation, GraphQLArgument, GraphQLEnumType, GraphQLInputObjectType,
    GraphQLInterfaceType, GraphQLNamedType, GraphQLObjectType, GraphQLSchema, GraphQLType, GraphQLUnionType,
    OperationType, DEFAULT_DEPRECATION_REASON,
};

const SDL: &str = r#"
"The schema."
schema {
  query: Root
  mutation: Mutation
}

"Restricts a field or type to a role."
directive @auth(role: Role = ADMIN, scopes: [String!]) repeatable on FIELD_DEFINITION | OBJECT

scalar DateTime @specifiedBy(url: "https://example.com/datetime")

"The query root."
type Root {
  "A node by ID."
  node(id: ID!): Node
  search(text: String = "graphql", first: Int = 10, filter: Filter): [SearchResult!]! @auth(role: USER)
  old: String @deprecated(reason: "Use `search`.")
  older(flag: Boolean @deprecated): String @deprecated
}

type Mutation {
  noop: Boolean
}

interface Node {
  id: ID!
}

interface Entity implements Node {
  id: ID!
}

type User implements Node & Entity @auth {
  id: ID!
  created: DateTime
}

extend type User {
  score: Float
}

type Post implements Node {
  id: ID!
}

union SearchResult = User | Post

enum Role {
  ADMIN
  "A signed-in user."
  USER @deprecated(reason: "Use ADMIN.")
  GUEST
}

input Filter {
  role: Role = GUEST
  tags: [String!] = ["a", "b"]
  limit: Int!
  legacy: String @deprecated
  pick: Pick
}

input Pick @oneOf {
  id: ID
  name: String
}
"#;

fn schema() -> GraphQLSchema {
    let schema = build_schema(SDL).expect("the SDL builds");
    assert!(validate_schema(&schema).expect("validates").is_empty());
    schema
}

fn object<'s>(schema: &'s GraphQLSchema, name: &str) -> &'s GraphQLObjectType {
    match schema.get_type(name) {
        Some(GraphQLNamedType::Object(object)) => object,
        other => panic!("{name} is not an object type: {other:?}"),
    }
}

fn interface<'s>(schema: &'s GraphQLSchema, name: &str) -> &'s GraphQLInterfaceType {
    match schema.get_type(name) {
        Some(GraphQLNamedType::Interface(interface)) => interface,
        other => panic!("{name} is not an interface type: {other:?}"),
    }
}

fn union<'s>(schema: &'s GraphQLSchema, name: &str) -> &'s GraphQLUnionType {
    match schema.get_type(name) {
        Some(GraphQLNamedType::Union(union)) => union,
        other => panic!("{name} is not a union type: {other:?}"),
    }
}

fn enumeration<'s>(schema: &'s GraphQLSchema, name: &str) -> &'s GraphQLEnumType {
    match schema.get_type(name) {
        Some(GraphQLNamedType::Enum(enum_type)) => enum_type,
        other => panic!("{name} is not an enum type: {other:?}"),
    }
}

fn input_object<'s>(schema: &'s GraphQLSchema, name: &str) -> &'s GraphQLInputObjectType {
    match schema.get_type(name) {
        Some(GraphQLNamedType::InputObject(input)) => input,
        other => panic!("{name} is not an input object type: {other:?}"),
    }
}

/// The printed default value.
fn default_of(arg: &GraphQLArgument) -> Option<String> {
    arg.default_value().map(|value| print(&*value))
}

#[test]
fn schema_description_root_types_and_ast_nodes() {
    let schema = schema();
    assert_eq!(schema.description(), Some("The schema."));
    assert_eq!(schema.ast_node().map(|node| node.operation_types.len()), Some(2));
    assert!(schema.extension_ast_nodes().is_empty());

    assert_eq!(schema.get_query_type().map(GraphQLNamedType::name), Some("Root"));
    assert_eq!(schema.get_mutation_type().map(GraphQLNamedType::name), Some("Mutation"));
    assert!(schema.get_subscription_type().is_none());
    assert_eq!(
        schema
            .get_root_type(OperationType::Mutation)
            .map(GraphQLNamedType::name),
        Some("Mutation")
    );
    assert!(schema.get_root_type(OperationType::Subscription).is_none());

    // Without a schema definition, the root types are the types named after the operations.
    let schema = build_schema("type Query { a: Int } type Subscription { b: Int }").unwrap();
    assert!(schema.ast_node().is_none());
    assert_eq!(schema.get_query_type().map(GraphQLNamedType::name), Some("Query"));
    assert_eq!(
        schema.get_subscription_type().map(GraphQLNamedType::name),
        Some("Subscription")
    );
}

#[test]
fn type_map_has_the_defined_used_scalar_and_introspection_types() {
    let schema = schema();
    // The order is graphql-js's, which `schema_differential.rs` checks.
    let names: Vec<&str> = schema.get_type_map().keys().map(String::as_str).collect();
    for name in [
        "DateTime",
        "Root",
        "Mutation",
        "Node",
        "Entity",
        "User",
        "Post",
        "SearchResult",
        "Role",
        "Filter",
        "Pick",
        "ID",
        "String",
        "Int",
        "Float",
        "Boolean",
        "__Schema",
        "__Type",
        "__TypeKind",
    ] {
        assert!(names.contains(&name), "{name} is in the type map");
    }
    assert_eq!(names.len(), 19 + 5, "and the other introspection types");
    assert!(schema.get_type("Missing").is_none());

    let id = schema.get_type("ID").unwrap();
    assert!(id.is_leaf_type());
    assert!(!id.is_composite_type());
    assert_eq!(id.to_string(), "ID");
}

#[test]
fn scalar_types() {
    let schema = schema();
    let Some(GraphQLNamedType::Scalar(date_time)) = schema.get_type("DateTime") else {
        panic!("DateTime is a scalar");
    };
    assert_eq!(date_time.name(), "DateTime");
    assert_eq!(date_time.specified_by_url(), Some("https://example.com/datetime"));
    assert_eq!(
        date_time.ast_node().map(|node| node.name.value.as_str()),
        Some("DateTime")
    );
    assert!(date_time.description().is_none());

    let Some(GraphQLNamedType::Scalar(int)) = schema.get_type("Int") else {
        panic!("Int is a scalar");
    };
    assert!(int.ast_node().is_none());
    assert!(int.specified_by_url().is_none());
    assert!(int.description().unwrap().starts_with("The `Int` scalar type"));
}

#[test]
fn object_type_fields_and_their_types() {
    let schema = schema();
    let root = object(&schema, "Root");
    assert_eq!(root.name(), "Root");
    assert_eq!(root.description(), Some("The query root."));
    assert!(root.interfaces().is_empty());

    let fields: Vec<(&str, String)> = root
        .fields()
        .values()
        .map(|field| (field.name(), field.ty().to_string()))
        .collect();
    assert_eq!(
        fields,
        [
            ("node", "Node".to_string()),
            ("search", "[SearchResult!]!".to_string()),
            ("old", "String".to_string()),
            ("older", "String".to_string()),
        ]
    );

    let node = &root.fields()["node"];
    assert_eq!(node.description(), Some("A node by ID."));
    assert_eq!(node.to_string(), "Root.node");

    let search = &root.fields()["search"];
    let ty = search.ty();
    assert!(ty.is_non_null());
    assert!(ty.nullable().is_list());
    assert_eq!(ty.named_type(), "SearchResult");
    assert_eq!(
        *ty,
        GraphQLType::non_null(GraphQLType::list(GraphQLType::non_null(GraphQLType::named(
            "SearchResult"
        ))))
    );
    assert_eq!(
        ty.of_type().and_then(GraphQLType::of_type).map(ToString::to_string),
        Some("SearchResult!".to_string())
    );
    assert!(matches!(
        schema.get_type(ty.named_type()),
        Some(GraphQLNamedType::Union(_))
    ));

    // Applied directives are read from the definition.
    let directives: Vec<&str> = search
        .ast_node()
        .unwrap()
        .directives
        .iter()
        .map(|directive| directive.name.value.as_str())
        .collect();
    assert_eq!(directives, ["auth"]);
}

#[test]
fn arguments_defaults_and_required() {
    let schema = schema();
    let root = object(&schema, "Root");

    let id = &root.fields()["node"].args()[0];
    assert_eq!(id.name(), "id");
    assert_eq!(id.ty().to_string(), "ID!");
    assert!(id.is_required());
    assert!(id.default_value().is_none());
    assert_eq!(id.to_string(), "Root.node(id:)");

    let args = root.fields()["search"].args();
    let shapes: Vec<(&str, String, Option<String>, bool)> = args
        .iter()
        .map(|arg| (arg.name(), arg.ty().to_string(), default_of(arg), arg.is_required()))
        .collect();
    assert_eq!(
        shapes,
        [
            ("text", "String".to_string(), Some("\"graphql\"".to_string()), false),
            ("first", "Int".to_string(), Some("10".to_string()), false),
            ("filter", "Filter".to_string(), None, false),
        ]
    );
    assert_eq!(args[0].ast_node().map(|node| node.name.value.as_str()), Some("text"));
}

#[test]
fn deprecation() {
    let schema = schema();
    let root = object(&schema, "Root");
    assert!(root.fields()["search"].deprecation_reason().is_none());
    assert_eq!(root.fields()["old"].deprecation_reason(), Some("Use `search`."));
    let older = &root.fields()["older"];
    assert_eq!(older.deprecation_reason(), Some(DEFAULT_DEPRECATION_REASON));
    assert_eq!(older.args()[0].deprecation_reason(), Some(DEFAULT_DEPRECATION_REASON));

    let role = enumeration(&schema, "Role");
    let user = role.value("USER").unwrap().unwrap();
    assert_eq!(user.deprecation_reason(), Some("Use ADMIN."));
    assert!(role.value("ADMIN").unwrap().unwrap().deprecation_reason().is_none());

    let filter = input_object(&schema, "Filter");
    assert_eq!(
        filter.fields()["legacy"].deprecation_reason(),
        Some(DEFAULT_DEPRECATION_REASON)
    );
}

#[test]
fn interfaces_implementations_and_possible_types() {
    let schema = schema();
    let user = object(&schema, "User");
    assert_eq!(user.interfaces(), ["Node", "Entity"]);
    // The extension's field comes after the definition's.
    let fields: Vec<&str> = user.fields().keys().map(String::as_str).collect();
    assert_eq!(fields, ["id", "created", "score"]);
    assert_eq!(user.extension_ast_nodes().len(), 1);
    assert_eq!(user.ast_node().map(|node| node.directives.len()), Some(1));

    let entity = interface(&schema, "Entity");
    assert_eq!(entity.name(), "Entity");
    assert_eq!(entity.interfaces(), ["Node"]);
    assert_eq!(entity.fields().keys().collect::<Vec<_>>(), ["id"]);

    let node = schema.get_type("Node").unwrap();
    assert!(node.is_interface_type());
    assert!(node.is_abstract_type());
    assert_eq!(schema.get_possible_types(node), ["User", "Post"]);
    let implementations = schema.get_implementations("Node");
    assert_eq!(implementations.objects, ["User", "Post"]);
    assert_eq!(implementations.interfaces, ["Entity"]);
    assert!(schema.is_sub_type(node, "User"));
    assert!(schema.is_sub_type(node, "Entity"));
    assert!(!schema.is_sub_type(node, "Root"));
    assert!(schema.get_possible_types(schema.get_type("Root").unwrap()).is_empty());
}

#[test]
fn union_types() {
    let schema = schema();
    let search_result = union(&schema, "SearchResult");
    assert_eq!(search_result.types(), ["User", "Post"]);
    assert_eq!(search_result.ast_node().map(|node| node.types.len()), Some(2));
    let named = schema.get_type("SearchResult").unwrap();
    assert!(named.is_union_type());
    assert_eq!(schema.get_possible_types(named), ["User", "Post"]);
    assert!(schema.is_sub_type(named, "Post"));
    assert!(!schema.is_sub_type(named, "Root"));
}

#[test]
fn enum_values() {
    let schema = schema();
    let role = enumeration(&schema, "Role");
    let values: Vec<&str> = role.values().unwrap().iter().map(|value| value.name()).collect();
    assert_eq!(values, ["ADMIN", "USER", "GUEST"]);
    let user = role.value("USER").unwrap().unwrap();
    assert_eq!(user.description(), Some("A signed-in user."));
    assert_eq!(user.to_string(), "Role.USER");
    assert_eq!(user.ast_node().map(|node| node.name.value.as_str()), Some("USER"));
    assert!(role.value("OWNER").unwrap().is_none());
}

#[test]
fn enum_values_that_throw_in_graphql_js_are_an_err() {
    let schema = build_schema("type Query { a: E } enum E { A @deprecated(reason: 1) }").unwrap();
    let e = enumeration(&schema, "E");
    let expected = "Argument \"@deprecated(reason:)\" has invalid value: String cannot represent a non string value: 1";
    assert_eq!(e.values().unwrap_err().message, expected);
    assert_eq!(e.value("A").unwrap_err().message, expected);
    assert_eq!(validate_schema(&schema).unwrap_err().message, expected);
}

#[test]
fn input_object_fields() {
    let schema = schema();
    let filter = input_object(&schema, "Filter");
    assert!(!filter.is_one_of());
    let fields: Vec<(&str, String, Option<String>, bool)> = filter
        .fields()
        .values()
        .map(|field| {
            (
                field.name(),
                field.ty().to_string(),
                field.default_value().map(|value| print(&*value)),
                field.is_required(),
            )
        })
        .collect();
    assert_eq!(
        fields,
        [
            ("role", "Role".to_string(), Some("GUEST".to_string()), false),
            (
                "tags",
                "[String!]".to_string(),
                Some("[\"a\", \"b\"]".to_string()),
                false
            ),
            ("limit", "Int!".to_string(), None, true),
            ("legacy", "String".to_string(), None, false),
            ("pick", "Pick".to_string(), None, false),
        ]
    );
    assert_eq!(filter.fields()["tags"].to_string(), "Filter.tags");
    assert!(input_object(&schema, "Pick").is_one_of());
    assert!(schema.get_type("Filter").unwrap().is_input_object_type());
}

#[test]
fn directives_defined_and_specified() {
    let schema = schema();
    let names: Vec<&str> = schema.get_directives().iter().map(|d| d.name()).collect();
    assert_eq!(names, ["auth", "include", "skip", "deprecated", "specifiedBy", "oneOf"]);

    let auth = schema.get_directive("auth").unwrap();
    assert_eq!(auth.to_string(), "@auth");
    assert_eq!(auth.description(), Some("Restricts a field or type to a role."));
    assert!(auth.is_repeatable());
    assert_eq!(
        auth.locations(),
        [DirectiveLocation::FieldDefinition, DirectiveLocation::Object]
    );
    assert!(auth.ast_node().is_some());
    let args: Vec<(&str, String, Option<String>)> = auth
        .args()
        .iter()
        .map(|arg| (arg.name(), arg.ty().to_string(), default_of(arg)))
        .collect();
    assert_eq!(
        args,
        [
            ("role", "Role".to_string(), Some("ADMIN".to_string())),
            ("scopes", "[String!]".to_string(), None),
        ]
    );
    assert_eq!(auth.args()[0].to_string(), "@auth(role:)");

    // A specified directive: no AST node, and a default given as a value, as a literal.
    let deprecated = schema.get_directive("deprecated").unwrap();
    assert!(deprecated.ast_node().is_none());
    assert!(!deprecated.is_repeatable());
    let reason = &deprecated.args()[0];
    assert_eq!(reason.ty().to_string(), "String!");
    assert!(!reason.is_required());
    assert_eq!(default_of(reason), Some(format!("\"{DEFAULT_DEPRECATION_REASON}\"")));
    assert!(schema.get_directive("missing").is_none());
}

#[test]
fn introspection_types() {
    let schema = schema();
    let type_type = object(&schema, "__Type");
    let include_deprecated = &type_type.fields()["fields"].args()[0];
    assert_eq!(include_deprecated.name(), "includeDeprecated");
    assert_eq!(default_of(include_deprecated), Some("false".to_string()));
    assert!(type_type.ast_node().is_none());

    let type_kind = enumeration(&schema, "__TypeKind");
    assert_eq!(type_kind.values().unwrap().len(), 8);
}
