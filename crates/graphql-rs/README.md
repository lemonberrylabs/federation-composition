# graphql-rs

A Rust port of [graphql-js](https://github.com/graphql/graphql-js) 17.0.2: the language layer (AST, lexer, parser, printer, visitor, `stripIgnoredCharacters`, `concatAST`) and schema building and validation (`buildSchema`, `buildASTSchema`, `validateSchema`, `assertValidSchema`, `validateSDL`). Parse results, error messages and printed output match graphql-js byte for byte; the tests in `tests/` compare the two directly.

## Usage

```toml
[dependencies]
graphql-rs = "0.1"
```

Parse a document, inspect it and print it back:

```rust
use graphql_rs::{parse, print, Definition};

let document = parse("type Query { hello(name: String = \"world\"): String }")?;

let Definition::ObjectTypeDefinition(query) = &document.definitions[0] else {
    unreachable!("the document starts with an object type");
};
assert_eq!(query.name.value, "Query");
assert_eq!(query.fields[0].arguments[0].name.value, "name");

assert_eq!(
    print(&document),
    "type Query {\n  hello(name: String = \"world\"): String\n}"
);
# Ok::<(), graphql_rs::GraphQLError>(())
```

Syntax errors are `GraphQLError`s with graphql-js's message:

```rust
let error = graphql_rs::parse("type Query {").unwrap_err();
assert_eq!(error.message, "Syntax Error: Expected Name, found <EOF>.");
```

Build a schema from SDL and validate it:

```rust
use graphql_rs::{assert_valid_schema, build_schema, validate_schema};

let schema = build_schema("type Query { user(id: ID!): User } type User { id: ID! }")?;
assert!(validate_schema(&schema)?.is_empty());

// `buildSchema` validates the SDL itself, and `validateSchema` the type system.
let unknown_type = build_schema("type Query { user: Usr } type User { id: ID! }").unwrap_err();
assert_eq!(unknown_type.message, "Unknown type \"Usr\". Did you mean \"User\"?");

let schema = build_schema("type Query { node: Node } interface Node { id: ID! } type User implements Node { name: String }")?;
let error = assert_valid_schema(&schema).unwrap_err();
assert_eq!(error.message, "Interface field Node.id expected but User does not provide it.");
# Ok::<(), graphql_rs::GraphQLError>(())
```

Look up the types of a built schema and read their fields and arguments:

```rust
use graphql_rs::{build_schema, print, GraphQLNamedType};

let schema = build_schema(
    r#"
    type Query {
      user(id: ID!): User
      users(first: Int = 10, role: Role): [User!]!
    }
    type User { id: ID! name: String @deprecated(reason: "Use `fullName`.") fullName: String }
    enum Role { ADMIN MEMBER }
    "#,
)?;

let Some(GraphQLNamedType::Object(query)) = schema.get_query_type() else {
    unreachable!("the query root type is an object type");
};
let users = &query.fields()["users"];
assert_eq!(users.ty().to_string(), "[User!]!");
assert_eq!(users.ty().named_type(), "User");

let args: Vec<(&str, String, Option<String>)> = users
    .args()
    .iter()
    .map(|arg| (arg.name(), arg.ty().to_string(), arg.default_value().map(|value| print(&*value))))
    .collect();
assert_eq!(
    args,
    [("first", "Int".to_string(), Some("10".to_string())), ("role", "Role".to_string(), None)]
);

// Named types are referenced by name; `get_type` looks them up.
let Some(GraphQLNamedType::Object(user)) = schema.get_type(users.ty().named_type()) else {
    unreachable!("User is an object type");
};
let deprecated: Vec<(&str, Option<&str>)> = user
    .fields()
    .values()
    .map(|field| (field.name(), field.deprecation_reason()))
    .collect();
assert_eq!(
    deprecated,
    [("id", None), ("name", Some("Use `fullName`.")), ("fullName", None)]
);
# Ok::<(), graphql_rs::GraphQLError>(())
```

The type system is read-only: `GraphQLSchema` (`get_type`, `get_type_map`, the root types, `get_directives`, `get_possible_types`, `get_implementations`, `is_sub_type`), `GraphQLNamedType` and the type of each kind (`GraphQLObjectType`, `GraphQLInterfaceType`, `GraphQLUnionType`, `GraphQLEnumType`, `GraphQLInputObjectType`, `GraphQLScalarType`), `GraphQLField`, `GraphQLArgument`, `GraphQLInputField`, `GraphQLEnumValue`, `GraphQLDirective` and `GraphQLType`. Each accessor is named after its graphql-js property, and the AST node a type, field or value was built from (`ast_node`) gives its applied directives. An enum's values are the one fallible accessor: graphql-js computes them on first access and throws for an invalid `@deprecated(reason:)`, which `validate_schema` reports too.

Walk a document with a `Visitor`:

```rust
use graphql_rs::{parse, visit, NodeRef, VisitAction, VisitInfo, Visitor};

/// Collects the names of the fields defined in a document.
struct FieldNames(Vec<String>);

impl<'a> Visitor<'a> for FieldNames {
    fn enter(&mut self, node: NodeRef<'a>, _cx: &mut (), _info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::FieldDefinition(field) = node {
            self.0.push(field.name.value.clone());
        }
        VisitAction::Continue
    }
}

let document = parse("type Query { a: Int b: String } type Mutation { c: Boolean }")?;
let mut names = FieldNames(Vec::new());
visit(&document, &mut names, &mut ());
assert_eq!(names.0, ["a", "b", "c"]);
# Ok::<(), graphql_rs::GraphQLError>(())
```

`visit_edit` is graphql-js `visit` with callbacks that replace or delete nodes; it returns the edited copy. `visit_in_parallel` and `visit_in_parallel_edit` run several visitors in one traversal.

## The AST

AST structs are named like graphql-js node kinds without the `Node` suffix (`FieldDefinitionNode` is `FieldDefinition`), with graphql-js's fields in snake_case (`type` is spelled `ty`). Optional lists are plain `Vec`s, optional nodes `Option`s, and every node implements `Default`, so nodes can be built with struct update syntax. Serde produces and reads graphql-js's JSON form of a document.

## Scope

The parser, printer and visitor are complete ports. Of the type system, only what `buildSchema`/`buildASTSchema` and `validateSchema` need is ported: there is no `extendSchema` of an existing schema, no execution and no validation of executable documents. A built `GraphQLSchema` can be validated and inspected, but not executed, printed back to SDL or introspected with a query, and its introspection types have no descriptions. The crate has a hidden `__internal` module for this repository's differential tests; it is not part of the public API and can change in any release.

One addition to graphql-js: the parser rejects documents that nest more than `MAX_NESTING_DEPTH` (128) levels of list values, object values, selection sets and list types (`ParseOptions::max_nesting_depth` changes it), so that the recursive printer, visitor and schema building cannot overflow the stack; `check_nesting_depth` checks a document built in code. See [SECURITY.md](https://github.com/lemonberrylabs/federation-composition/blob/main/SECURITY.md#untrusted-input).

## Credits

graphql-rs is a port of graphql-js by the GraphQL Contributors, and keeps its MIT license. See the `LICENSE` file.
