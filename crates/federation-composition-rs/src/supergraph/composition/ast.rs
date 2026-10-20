//! Port of `src/supergraph/composition/ast.ts`: builders of supergraph AST nodes.
//!
//! The TS parameter object types (`ObjectTypeAST = inferArgument<typeof createObjectTypeNode>`)
//! are the `*Ast` structs below. Optional arrays (`tags?: string[]`) are `Vec`s (the TS
//! treats `undefined` and `[]` alike there), optional flags are `bool` (falsy when missing),
//! and other optional values are `Option`.

// The `*Ast` structs copy upstream's parameter objects field for field, and outside the crate
// they are reachable only through the unstable `__internal` module.
#![allow(missing_docs)]

use std::sync::LazyLock;

use crate::utils::collections::IndexSet;
use graphql_rs::{
    visit_edit, visit_in_parallel_edit, Ancestor, ConstArgument, ConstDirective, ConstObjectField, ConstValue,
    DirectiveDefinition, Document, EditAction, EditVisitor, EnumTypeDefinition, EnumValueDefinition, FieldDefinition,
    InputObjectTypeDefinition, InputValueDefinition, InterfaceTypeDefinition, Name, NamedType, Node, NodeRef,
    ObjectTypeDefinition, OperationType, OperationTypeDefinition, PathKey, ScalarTypeDefinition, SchemaDefinition,
    StringValue, Type, UnionTypeDefinition, VisitInfo,
};

use crate::graphql::printer::print;
use crate::subgraph::state::{ArgumentKind, Deprecated, Description};
use crate::utils::js::{number_to_string, JsError, JsResult};

/// `DescriptionAST`: `{ value, block }`.
pub type DescriptionAst = Description;

/// `Link`: the argument of `createLinkDirectiveNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkAst {
    pub url: String,
    pub import: Option<Vec<LinkImportAst>>,
    pub r#for: Option<String>,
}

/// An entry of `LinkAst::import`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkImportAst {
    pub name: String,
    pub alias: Option<String>,
}

/// `Cost`: the argument of `createCostDirectiveNode`.
#[derive(Clone, Debug, PartialEq)]
pub struct CostAst {
    pub cost: f64,
    pub directive_name: String,
}

/// `ListSize`: the argument of `createListSizeDirectiveNode`.
#[derive(Clone, Debug, PartialEq)]
pub struct ListSizeAst {
    pub assumed_size: Option<f64>,
    pub slicing_arguments: Option<Vec<String>>,
    pub sized_fields: Option<Vec<String>>,
    pub require_one_slicing_argument: bool,
    pub print_require_one_slicing_argument: bool,
    pub directive_name: String,
}

/// The `join?: { type?, implements?, field?, unionMember?, enumValue? }` property; each
/// builder reads only its own keys.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinAst {
    pub r#type: Vec<JoinTypeAst>,
    pub implements: Vec<JoinImplementsAst>,
    pub field: Vec<JoinFieldAst>,
    pub union_member: Vec<JoinUnionMemberAst>,
    pub enum_value: Vec<JoinEnumValueAst>,
}

/// `SchemaAST`: the argument of `createSchemaNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SchemaAst {
    pub query: Option<String>,
    pub mutation: Option<String>,
    pub subscription: Option<String>,
    pub links: Vec<LinkAst>,
}

/// `DirectiveAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DirectiveAst {
    pub name: String,
    pub tags: Vec<String>,
    pub arguments: Vec<FieldArgumentAst>,
    pub locations: Vec<String>,
    pub repeatable: bool,
}

/// `ObjectTypeAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectTypeAst {
    pub name: String,
    pub fields: Vec<FieldAst>,
    pub interfaces: Vec<String>,
    pub join: JoinAst,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub contexts: Vec<String>,
    pub cost: Option<CostAst>,
    pub description: Option<DescriptionAst>,
    /// `ast?.directives`.
    pub ast: Vec<ConstDirective>,
}

/// The argument of `createInterfaceTypeNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterfaceTypeAst {
    pub name: String,
    pub fields: Vec<FieldAst>,
    pub interfaces: Vec<String>,
    pub join: JoinAst,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub contexts: Vec<String>,
    pub description: Option<DescriptionAst>,
    pub ast: Vec<ConstDirective>,
}

/// The argument of `createInputObjectTypeNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputObjectTypeAst {
    pub name: String,
    pub fields: Vec<InputFieldAst>,
    pub join: JoinAst,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub description: Option<DescriptionAst>,
    pub is_one_of: bool,
    pub ast: Vec<ConstDirective>,
}

/// The argument of `createUnionTypeNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UnionTypeAst {
    pub name: String,
    pub join: JoinAst,
    pub members: Vec<String>,
    pub inaccessible: bool,
    pub contexts: Vec<String>,
    pub description: Option<DescriptionAst>,
    pub tags: Vec<String>,
    pub ast: Vec<ConstDirective>,
}

/// The argument of `createEnumTypeNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EnumTypeAst {
    pub name: String,
    pub join: JoinAst,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<CostAst>,
    pub description: Option<DescriptionAst>,
    pub values: Vec<EnumValueAst>,
    pub ast: Vec<ConstDirective>,
}

/// The argument of `createScalarTypeNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScalarTypeAst {
    pub name: String,
    pub join: JoinAst,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<CostAst>,
    pub description: Option<DescriptionAst>,
    pub specified_by: Option<String>,
    pub ast: Vec<ConstDirective>,
}

/// An entry of the argument of `createJoinGraphEnumTypeNode`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinGraphAst {
    pub name: String,
    pub enum_value: String,
    pub url: Option<String>,
}

/// `FieldAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldAst {
    pub name: String,
    pub r#type: String,
    pub arguments: Vec<FieldArgumentAst>,
    pub join: JoinAst,
    pub inaccessible: bool,
    /// NOTE(port): not in the TS `createFieldNode` parameter type, but the object and interface
    /// builders pass `authenticated`, `policies` and `scopes` on every field object and
    /// `applyDirectives(field)` reads them at runtime (emitting `@authenticated`, `@policy`,
    /// `@requiresScopes` on fields), so they are part of the field AST here.
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<CostAst>,
    pub list_size: Option<ListSizeAst>,
    pub tags: Vec<String>,
    pub description: Option<DescriptionAst>,
    pub deprecated: Option<Deprecated>,
    pub ast: Vec<ConstDirective>,
}

/// `InputFieldAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputFieldAst {
    pub name: String,
    pub r#type: String,
    pub default_value: Option<String>,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub cost: Option<CostAst>,
    pub description: Option<DescriptionAst>,
    /// Not in the TS `InputFieldAST` type, but the input object builder passes it and
    /// `applyDirectives` reads it.
    pub deprecated: Option<Deprecated>,
    pub ast: Vec<ConstDirective>,
    pub join: JoinAst,
}

/// `EnumValueAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EnumValueAst {
    pub name: String,
    pub join: JoinAst,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub description: Option<DescriptionAst>,
    pub deprecated: Option<Deprecated>,
    pub ast: Vec<ConstDirective>,
}

/// `FieldArgumentAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldArgumentAst {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub default_value: Option<String>,
    pub inaccessible: bool,
    pub cost: Option<CostAst>,
    pub tags: Vec<String>,
    pub description: Option<DescriptionAst>,
    pub deprecated: Option<Deprecated>,
    pub ast: Vec<ConstDirective>,
}

/// `JoinTypeAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinTypeAst {
    pub graph: String,
    pub key: Option<String>,
    pub is_interface_object: Option<bool>,
    pub extension: Option<bool>,
    pub resolvable: Option<bool>,
}

/// `JoinImplementsAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinImplementsAst {
    pub graph: String,
    pub interface: String,
}

/// An entry of `JoinFieldAST.contextArguments`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinFieldContextArgumentAst {
    pub name: String,
    pub r#type: String,
    pub context: String,
    pub selection: String,
}

/// `JoinFieldAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinFieldAst {
    pub graph: Option<String>,
    pub r#type: Option<String>,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub used_overridden: Option<bool>,
    pub external: Option<bool>,
    pub provides: Option<String>,
    pub requires: Option<String>,
    pub context_arguments: Option<Vec<JoinFieldContextArgumentAst>>,
}

/// `JoinUnionMemberAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinUnionMemberAst {
    pub graph: String,
    pub member: String,
}

/// `JoinEnumValueAST`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JoinEnumValueAst {
    pub graph: String,
}

/// The argument of the node builders below.
/// PERF: the builders take their argument by value, so its strings and directive nodes are
/// moved into the AST instead of copied (and the argument is not dropped afterwards). A
/// reference, as the tests pass, is cloned first; the output is the same either way.
pub trait AstArg<T> {
    fn into_arg(self) -> T;
}

impl<T> AstArg<T> for T {
    fn into_arg(self) -> T {
        self
    }
}

impl<T: Clone> AstArg<T> for &T {
    fn into_arg(self) -> T {
        self.clone()
    }
}

/// The argument of `applyDirectives`: the union of the properties the builders pass.
/// `ast` is owned so the node's own directives are moved into the result.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommonDirectivesAst<'c> {
    pub ast: Vec<ConstDirective>,
    pub join: Option<JoinAst>,
    pub deprecated: Option<&'c Deprecated>,
    pub specified_by: Option<&'c str>,
    pub tags: Vec<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: &'c [Vec<String>],
    pub scopes: &'c [Vec<String>],
    pub contexts: &'c [String],
    pub cost: Option<&'c CostAst>,
    pub list_size: Option<&'c ListSizeAst>,
    pub is_one_of: bool,
}

pub fn create_schema_node(schema: &SchemaAst) -> SchemaDefinition {
    let mut operation_types: Vec<OperationTypeDefinition> = Vec::new();
    // `schema.query ? {...} : []`: an empty name is falsy.
    if let Some(query) = schema.query.as_deref().filter(|s| !s.is_empty()) {
        operation_types.push(OperationTypeDefinition {
            operation: OperationType::Query,
            ty: create_named_type_node(query),
        });
    }
    if let Some(mutation) = schema.mutation.as_deref().filter(|s| !s.is_empty()) {
        operation_types.push(OperationTypeDefinition {
            operation: OperationType::Mutation,
            ty: create_named_type_node(mutation),
        });
    }
    if let Some(subscription) = schema.subscription.as_deref().filter(|s| !s.is_empty()) {
        operation_types.push(OperationTypeDefinition {
            operation: OperationType::Subscription,
            ty: create_named_type_node(subscription),
        });
    }

    SchemaDefinition {
        description: None,
        directives: schema.links.iter().map(create_link_directive_node).collect(),
        operation_types,
    }
}

pub fn create_directive_node(directive: impl AstArg<DirectiveAst>) -> JsResult<DirectiveDefinition> {
    let directive = directive.into_arg();
    Ok(DirectiveDefinition {
        description: None,
        name: Name::new(directive.name),
        locations: directive.locations.into_iter().map(Name::new).collect(),
        repeatable: directive.repeatable,
        arguments: directive
            .arguments
            .into_iter()
            .map(create_field_argument_node)
            .collect::<JsResult<_>>()?,
        directives: vec![],
    })
}

pub fn create_object_type_node(object_type: impl AstArg<ObjectTypeAst>) -> JsResult<ObjectTypeDefinition> {
    let object_type = object_type.into_arg();
    Ok(ObjectTypeDefinition {
        name: Name::new(object_type.name),
        directives: apply_directives(CommonDirectivesAst {
            ast: object_type.ast,
            join: Some(object_type.join),
            tags: object_type.tags,
            inaccessible: object_type.inaccessible,
            authenticated: object_type.authenticated,
            policies: &object_type.policies,
            scopes: &object_type.scopes,
            contexts: &object_type.contexts,
            cost: object_type.cost.as_ref(),
            ..Default::default()
        }),
        fields: object_type
            .fields
            .into_iter()
            .map(create_field_node)
            .collect::<JsResult<_>>()?,
        description: object_type.description.map(create_description_node),
        interfaces: object_type.interfaces.into_iter().map(create_named_type_node).collect(),
    })
}

pub fn create_interface_type_node(interface_type: impl AstArg<InterfaceTypeAst>) -> JsResult<InterfaceTypeDefinition> {
    let interface_type = interface_type.into_arg();
    Ok(InterfaceTypeDefinition {
        name: Name::new(interface_type.name),
        directives: apply_directives(CommonDirectivesAst {
            ast: interface_type.ast,
            join: Some(interface_type.join),
            tags: interface_type.tags,
            inaccessible: interface_type.inaccessible,
            authenticated: interface_type.authenticated,
            policies: &interface_type.policies,
            scopes: &interface_type.scopes,
            contexts: &interface_type.contexts,
            ..Default::default()
        }),
        description: interface_type.description.map(create_description_node),
        fields: interface_type
            .fields
            .into_iter()
            .map(create_field_node)
            .collect::<JsResult<_>>()?,
        interfaces: interface_type
            .interfaces
            .into_iter()
            .map(create_named_type_node)
            .collect(),
    })
}

pub fn create_input_object_type_node(
    input_object_type: impl AstArg<InputObjectTypeAst>,
) -> JsResult<InputObjectTypeDefinition> {
    let input_object_type = input_object_type.into_arg();
    Ok(InputObjectTypeDefinition {
        name: Name::new(input_object_type.name),
        directives: apply_directives(CommonDirectivesAst {
            ast: input_object_type.ast,
            join: Some(input_object_type.join),
            tags: input_object_type.tags,
            inaccessible: input_object_type.inaccessible,
            is_one_of: input_object_type.is_one_of,
            ..Default::default()
        }),
        fields: input_object_type
            .fields
            .into_iter()
            .map(create_input_field_node)
            .collect::<JsResult<_>>()?,
        description: input_object_type.description.map(create_description_node),
    })
}

pub fn create_union_type_node(union_type: impl AstArg<UnionTypeAst>) -> UnionTypeDefinition {
    let union_type = union_type.into_arg();
    UnionTypeDefinition {
        name: Name::new(union_type.name),
        directives: apply_directives(CommonDirectivesAst {
            ast: union_type.ast,
            join: Some(union_type.join),
            inaccessible: union_type.inaccessible,
            contexts: &union_type.contexts,
            tags: union_type.tags,
            ..Default::default()
        }),
        description: union_type.description.map(create_description_node),
        types: union_type.members.into_iter().map(NamedType::new).collect(),
    }
}

pub fn create_enum_type_node(enum_type: impl AstArg<EnumTypeAst>) -> EnumTypeDefinition {
    let enum_type = enum_type.into_arg();
    EnumTypeDefinition {
        name: Name::new(enum_type.name),
        directives: apply_directives(CommonDirectivesAst {
            ast: enum_type.ast,
            join: Some(enum_type.join),
            tags: enum_type.tags,
            inaccessible: enum_type.inaccessible,
            authenticated: enum_type.authenticated,
            policies: &enum_type.policies,
            scopes: &enum_type.scopes,
            cost: enum_type.cost.as_ref(),
            ..Default::default()
        }),
        description: enum_type.description.map(create_description_node),
        values: enum_type.values.into_iter().map(create_enum_value_node).collect(),
    }
}

pub fn create_scalar_type_node(scalar_type: impl AstArg<ScalarTypeAst>) -> ScalarTypeDefinition {
    let scalar_type = scalar_type.into_arg();
    ScalarTypeDefinition {
        name: Name::new(scalar_type.name),
        description: scalar_type.description.map(create_description_node),
        directives: apply_directives(CommonDirectivesAst {
            ast: scalar_type.ast,
            join: Some(scalar_type.join),
            tags: scalar_type.tags,
            inaccessible: scalar_type.inaccessible,
            authenticated: scalar_type.authenticated,
            policies: &scalar_type.policies,
            scopes: &scalar_type.scopes,
            cost: scalar_type.cost.as_ref(),
            specified_by: scalar_type.specified_by.as_deref(),
            ..Default::default()
        }),
    }
}

/// `createJoinGraphEnumTypeNode(graphs)`: the `join__Graph` enum.
pub fn create_join_graph_enum_type_node(graphs: &[JoinGraphAst]) -> EnumTypeDefinition {
    create_enum_type_node(EnumTypeAst {
        name: "join__Graph".to_string(),
        cost: None,
        values: graphs
            .iter()
            .map(|graph| EnumValueAst {
                name: graph.enum_value.clone(),
                ast: vec![create_join_graph_directive_node(&graph.name, graph.url.as_deref())],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    })
}

/// `parseType(field.type)`. NOTE(port): it throws on an invalid type string. The types are
/// printed by the subgraph state from the subgraphs' type nodes, so they parse when the names in
/// those nodes are valid, as in any document the parser produced; a document built in code (or
/// deserialized) can hold any string as a name, and then this throws the parser's error, like the
/// TS.
fn parse_type_string(ty: &str) -> JsResult<Type> {
    // PERF: printed type strings (`[User!]!`) have no whitespace, comments or commas, so they
    // are read directly; `parseType` would produce the same node. Anything else goes through
    // the parser.
    let mut position = 0;
    if let Some(parsed) = parse_printed_type(ty, &mut position) {
        if position == ty.len() {
            return Ok(parsed);
        }
    }
    graphql_rs::parse_type(ty).map_err(JsError::GraphQL)
}

/// `Type : NamedType | "[" Type "]" | Type "!"` (a non-null type cannot be wrapped in another
/// `!`), with names `[_A-Za-z][_0-9A-Za-z]*`. `None` when `ty` is not in that form at
/// `position`.
fn parse_printed_type(ty: &str, position: &mut usize) -> Option<Type> {
    let bytes = ty.as_bytes();
    let parsed = if bytes.get(*position) == Some(&b'[') {
        *position += 1;
        let inner = parse_printed_type(ty, position)?;
        if bytes.get(*position) != Some(&b']') {
            return None;
        }
        *position += 1;
        Type::list(inner)
    } else {
        let start = *position;
        if !bytes.get(start).is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_') {
            return None;
        }
        *position += 1;
        while bytes
            .get(*position)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            *position += 1;
        }
        Type::named(&ty[start..*position])
    };
    if bytes.get(*position) == Some(&b'!') {
        *position += 1;
        return Some(Type::non_null(parsed));
    }
    Some(parsed)
}

/// `parseConstValue(value)`. NOTE(port): it throws on invalid input. Default values are printed
/// by the subgraph state from the subgraphs' value nodes, so they parse when those nodes are
/// valid (see [`parse_type_string`]); otherwise this throws the parser's error, like the TS.
fn parse_const_value_string(value: &str) -> JsResult<ConstValue> {
    graphql_rs::parse_const_value(value).map_err(JsError::GraphQL)
}

fn create_field_node(field: impl AstArg<FieldAst>) -> JsResult<FieldDefinition> {
    let field = field.into_arg();
    Ok(FieldDefinition {
        name: Name::new(field.name),
        ty: parse_type_string(&field.r#type)?,
        directives: apply_directives(CommonDirectivesAst {
            ast: field.ast,
            join: Some(field.join),
            inaccessible: field.inaccessible,
            authenticated: field.authenticated,
            policies: &field.policies,
            scopes: &field.scopes,
            cost: field.cost.as_ref(),
            list_size: field.list_size.as_ref(),
            tags: field.tags,
            deprecated: field.deprecated.as_ref(),
            ..Default::default()
        }),
        description: field.description.map(create_description_node),
        arguments: field
            .arguments
            .into_iter()
            .map(create_field_argument_node)
            .collect::<JsResult<_>>()?,
    })
}

fn create_input_field_node(input_field: impl AstArg<InputFieldAst>) -> JsResult<InputValueDefinition> {
    let input_field = input_field.into_arg();
    Ok(InputValueDefinition {
        name: Name::new(input_field.name),
        ty: parse_type_string(&input_field.r#type)?,
        directives: apply_directives(CommonDirectivesAst {
            ast: input_field.ast,
            join: Some(input_field.join),
            tags: input_field.tags,
            inaccessible: input_field.inaccessible,
            cost: input_field.cost.as_ref(),
            deprecated: input_field.deprecated.as_ref(),
            ..Default::default()
        }),
        description: input_field.description.map(create_description_node),
        default_value: input_field
            .default_value
            .as_deref()
            .map(parse_const_value_string)
            .transpose()?,
    })
}

fn create_enum_value_node(enum_value: impl AstArg<EnumValueAst>) -> EnumValueDefinition {
    let enum_value = enum_value.into_arg();
    EnumValueDefinition {
        name: Name::new(enum_value.name),
        directives: apply_directives(CommonDirectivesAst {
            ast: enum_value.ast,
            join: Some(enum_value.join),
            tags: enum_value.tags,
            inaccessible: enum_value.inaccessible,
            deprecated: enum_value.deprecated.as_ref(),
            ..Default::default()
        }),
        description: enum_value.description.map(create_description_node),
    }
}

/// `createDefaultValue(defaultValue, kind)`: parses the printed default value back into a
/// const value node.
/// NOTE(port): the body uses graphql-js `parseConstValue`, which throws on invalid input (see
/// [`parse_const_value_string`]).
fn create_default_value(default_value: Option<&str>, kind: ArgumentKind) -> JsResult<Option<ConstValue>> {
    let Some(default_value) = default_value else {
        return Ok(None);
    };

    if kind == ArgumentKind::Enum {
        let value = if default_value.starts_with('"') && default_value.ends_with('"') {
            js_substring(default_value, 1, default_value.len() - 1)
        } else {
            default_value
        };
        return Ok(Some(ConstValue::enum_value(value)));
    }

    parse_const_value_string(default_value).map(Some)
}

/// JS `str.substring(start, end)` for byte offsets on an ASCII boundary: the arguments are
/// swapped when `start > end` (`'"'.substring(1, 0)` is `'"'`).
fn js_substring(s: &str, start: usize, end: usize) -> &str {
    let (start, end) = if start > end { (end, start) } else { (start, end) };
    &s[start.min(s.len())..end.min(s.len())]
}

fn create_field_argument_node(argument: impl AstArg<FieldArgumentAst>) -> JsResult<InputValueDefinition> {
    let argument = argument.into_arg();
    Ok(InputValueDefinition {
        name: Name::new(argument.name),
        default_value: create_default_value(argument.default_value.as_deref(), argument.kind)?,
        ty: parse_type_string(&argument.r#type)?,
        directives: apply_directives(CommonDirectivesAst {
            ast: argument.ast,
            inaccessible: argument.inaccessible,
            cost: argument.cost.as_ref(),
            tags: argument.tags,
            deprecated: argument.deprecated.as_ref(),
            ..Default::default()
        }),
        description: argument.description.map(create_description_node),
    })
}

/// A JS truthiness check on an optional string.
fn truthy(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|s| !s.is_empty())
}

/// `truthy` for an owned string, which is then moved into the node.
fn truthy_owned(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty())
}

fn create_join_type_directive_node(join: impl AstArg<JoinTypeAst>) -> ConstDirective {
    let join = join.into_arg();
    let mut arguments = vec![ConstArgument::new("graph", ConstValue::enum_value(join.graph))];
    if let Some(key) = truthy_owned(join.key) {
        arguments.push(ConstArgument::new("key", ConstValue::string(key)));
    }
    if join.resolvable == Some(false) {
        arguments.push(ConstArgument::new("resolvable", ConstValue::boolean(false)));
    }
    if join.is_interface_object == Some(true) {
        arguments.push(ConstArgument::new("isInterfaceObject", ConstValue::boolean(true)));
    }
    if join.extension == Some(true) {
        arguments.push(ConstArgument::new("extension", ConstValue::boolean(true)));
    }
    ConstDirective::new("join__type", arguments)
}

fn create_join_implements_directive_node(join: impl AstArg<JoinImplementsAst>) -> ConstDirective {
    let join = join.into_arg();
    ConstDirective::new(
        "join__implements",
        vec![
            ConstArgument::new("graph", ConstValue::enum_value(join.graph)),
            ConstArgument::new("interface", ConstValue::string(join.interface)),
        ],
    )
}

fn create_join_field_directive_node(join: impl AstArg<JoinFieldAst>) -> ConstDirective {
    let join = join.into_arg();
    let mut arguments = Vec::new();
    if let Some(graph) = truthy_owned(join.graph) {
        arguments.push(ConstArgument::new("graph", ConstValue::enum_value(graph)));
    }
    if let Some(ty) = truthy_owned(join.r#type) {
        arguments.push(ConstArgument::new("type", ConstValue::string(ty)));
    }
    if let Some(r#override) = truthy_owned(join.r#override) {
        arguments.push(ConstArgument::new("override", ConstValue::string(r#override)));
    }
    if let Some(override_label) = truthy_owned(join.override_label) {
        arguments.push(ConstArgument::new("overrideLabel", ConstValue::string(override_label)));
    }
    if join.used_overridden == Some(true) {
        arguments.push(ConstArgument::new("usedOverridden", ConstValue::boolean(true)));
    }
    if join.external == Some(true) {
        arguments.push(ConstArgument::new("external", ConstValue::boolean(true)));
    }
    if let Some(provides) = truthy_owned(join.provides) {
        arguments.push(ConstArgument::new("provides", ConstValue::string(provides)));
    }
    if let Some(requires) = truthy_owned(join.requires) {
        arguments.push(ConstArgument::new("requires", ConstValue::string(requires)));
    }
    if let Some(context_arguments) = join.context_arguments.filter(|a| !a.is_empty()) {
        arguments.push(ConstArgument::new(
            "contextArguments",
            ConstValue::list(
                context_arguments
                    .into_iter()
                    .map(|argument| {
                        ConstValue::object(vec![
                            create_string_object_field_node("context", argument.context),
                            create_string_object_field_node("name", argument.name),
                            create_string_object_field_node("type", argument.r#type),
                            create_string_object_field_node("selection", argument.selection),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    ConstDirective::new("join__field", arguments)
}

fn create_string_object_field_node(name: &str, value: String) -> ConstObjectField {
    ConstObjectField::new(name, ConstValue::string(value))
}

/// `createContextDirectiveNode({ name })`.
fn create_context_directive_node(name: &str) -> ConstDirective {
    ConstDirective::new("context", vec![ConstArgument::new("name", ConstValue::string(name))])
}

fn create_join_union_member_directive_node(join: impl AstArg<JoinUnionMemberAst>) -> ConstDirective {
    let join = join.into_arg();
    ConstDirective::new(
        "join__unionMember",
        vec![
            ConstArgument::new("graph", ConstValue::enum_value(join.graph)),
            ConstArgument::new("member", ConstValue::string(join.member)),
        ],
    )
}

fn create_join_enum_value_directive_node(join: impl AstArg<JoinEnumValueAst>) -> ConstDirective {
    let join = join.into_arg();
    ConstDirective::new(
        "join__enumValue",
        vec![ConstArgument::new("graph", ConstValue::enum_value(join.graph))],
    )
}

fn create_inaccessible_directive_node() -> ConstDirective {
    ConstDirective::new("inaccessible", vec![])
}

fn create_one_of_directive_node() -> ConstDirective {
    ConstDirective::new("oneOf", vec![])
}

fn create_authenticated_directive_node() -> ConstDirective {
    ConstDirective::new("authenticated", vec![])
}

fn string_list_list(groups: &[Vec<String>]) -> ConstValue {
    ConstValue::list(
        groups
            .iter()
            .map(|group| ConstValue::list(group.iter().map(|v| ConstValue::string(v.as_str())).collect()))
            .collect(),
    )
}

fn create_policy_directive_node(policies: &[Vec<String>]) -> ConstDirective {
    ConstDirective::new(
        "policy",
        vec![ConstArgument::new("policies", string_list_list(policies))],
    )
}

fn create_requires_scopes_directive_node(scopes: &[Vec<String>]) -> ConstDirective {
    ConstDirective::new(
        "requiresScopes",
        vec![ConstArgument::new("scopes", string_list_list(scopes))],
    )
}

fn create_tag_directive_node(name: String) -> ConstDirective {
    ConstDirective::new("tag", vec![graphql_rs::Argument::new("name", ConstValue::string(name))])
}

/// `createJoinGraphDirectiveNode({ name, url? })`.
fn create_join_graph_directive_node(name: &str, url: Option<&str>) -> ConstDirective {
    ConstDirective::new(
        "join__graph",
        vec![
            ConstArgument::new("name", ConstValue::string(name)),
            ConstArgument::new("url", ConstValue::string(url.unwrap_or(""))),
        ],
    )
}

/// `createDescriptionNode(description)`: always a block string, because the supergraph SDL
/// produced by @apollo/composition always has `block: true` (the TS ignores
/// `description.block`).
fn create_description_node(description: DescriptionAst) -> StringValue {
    StringValue::new(description.value, true)
}

fn create_deprecated_directive_node(deprecated: &Deprecated) -> ConstDirective {
    ConstDirective::new(
        "deprecated",
        match &deprecated.reason {
            Some(reason) => vec![ConstArgument::new("reason", ConstValue::string(reason.as_str()))],
            None => vec![],
        },
    )
}

fn create_specified_by_directive_node(url: &str) -> ConstDirective {
    ConstDirective::new(
        "specifiedBy",
        vec![graphql_rs::Argument::new("url", ConstValue::string(url))],
    )
}

fn create_cost_directive_node(input: &CostAst) -> ConstDirective {
    ConstDirective::new(
        input.directive_name.as_str(),
        vec![ConstArgument::new(
            "weight",
            ConstValue::int(number_to_string(input.cost)),
        )],
    )
}

fn create_list_size_directive_node(input: &ListSizeAst) -> ConstDirective {
    let mut args: Vec<ConstArgument> = Vec::new();

    if !input.require_one_slicing_argument || input.print_require_one_slicing_argument {
        args.push(ConstArgument::new(
            "requireOneSlicingArgument",
            ConstValue::boolean(input.require_one_slicing_argument),
        ));
    }

    if let Some(assumed_size) = input.assumed_size {
        args.push(ConstArgument::new(
            "assumedSize",
            ConstValue::int(number_to_string(assumed_size)),
        ));
    }

    if let Some(slicing_arguments) = &input.slicing_arguments {
        args.push(ConstArgument::new(
            "slicingArguments",
            ConstValue::list(
                slicing_arguments
                    .iter()
                    .map(|arg| ConstValue::string(arg.as_str()))
                    .collect(),
            ),
        ));
    }

    if let Some(sized_fields) = &input.sized_fields {
        args.push(ConstArgument::new(
            "sizedFields",
            ConstValue::list(
                sized_fields
                    .iter()
                    .map(|arg| ConstValue::string(arg.as_str()))
                    .collect(),
            ),
        ));
    }

    ConstDirective::new(input.directive_name.as_str(), args)
}

fn create_link_directive_node(link: &LinkAst) -> ConstDirective {
    let mut arguments = vec![ConstArgument::new("url", ConstValue::string(link.url.as_str()))];
    if let Some(r#for) = truthy(&link.r#for) {
        arguments.push(ConstArgument::new("for", ConstValue::enum_value(r#for)));
    }
    // An empty array is truthy: `import: []` is printed.
    if let Some(import) = &link.import {
        arguments.push(ConstArgument::new(
            "import",
            ConstValue::list(
                import
                    .iter()
                    .map(|imported| {
                        // NOTE(port): an aliased import prints `{ name }` only, without `as`;
                        // this is what the TS does.
                        if imported.alias.as_deref().is_some_and(|a| !a.is_empty()) {
                            return ConstValue::object(vec![ConstObjectField::new(
                                "name",
                                ConstValue::string(imported.name.as_str()),
                            )]);
                        }
                        ConstValue::string(imported.name.as_str())
                    })
                    .collect(),
            ),
        ));
    }
    ConstDirective::new("link", arguments)
}

fn create_named_type_node(name: impl Into<String>) -> NamedType {
    NamedType::new(name)
}

/// `applyDirectives(common)`: the node's own directives (deduplicated by their printed form,
/// library printer) followed by the generated federation directives.
fn apply_directives(common: CommonDirectivesAst<'_>) -> Vec<ConstDirective> {
    // `.filter((directive, index, all) => all.findIndex((d) => d.string === directive.string) === index)`
    let mut directives: Vec<ConstDirective> = Vec::new();
    // PERF: a single directive cannot have a duplicate, so it is not printed.
    if common.ast.len() <= 1 {
        directives = common.ast;
    } else {
        let printed: Vec<String> = common.ast.iter().map(print).collect();
        for (index, directive) in common.ast.into_iter().enumerate() {
            if printed.iter().position(|d| *d == printed[index]) == Some(index) {
                directives.push(directive);
            }
        }
    }

    if let Some(join) = common.join {
        directives.extend(join.r#type.into_iter().map(create_join_type_directive_node));
        directives.extend(join.implements.into_iter().map(create_join_implements_directive_node));
        directives.extend(join.field.into_iter().map(create_join_field_directive_node));
        directives.extend(
            join.union_member
                .into_iter()
                .map(create_join_union_member_directive_node),
        );
        directives.extend(join.enum_value.into_iter().map(create_join_enum_value_directive_node));
    }
    directives.extend(common.contexts.iter().map(|name| create_context_directive_node(name)));
    directives.extend(common.tags.into_iter().map(create_tag_directive_node));
    if common.inaccessible {
        directives.push(create_inaccessible_directive_node());
    }
    if common.authenticated {
        directives.push(create_authenticated_directive_node());
    }
    if !common.policies.is_empty() {
        directives.push(create_policy_directive_node(common.policies));
    }
    if !common.scopes.is_empty() {
        directives.push(create_requires_scopes_directive_node(common.scopes));
    }
    if let Some(cost) = common.cost {
        directives.push(create_cost_directive_node(cost));
    }
    if let Some(list_size) = common.list_size {
        directives.push(create_list_size_directive_node(list_size));
    }
    if let Some(deprecated) = common.deprecated {
        directives.push(create_deprecated_directive_node(deprecated));
    }
    if let Some(specified_by) = common.specified_by.filter(|s| !s.is_empty()) {
        directives.push(create_specified_by_directive_node(specified_by));
    }
    if common.is_one_of {
        directives.push(create_one_of_directive_node());
    }
    directives
}

/// `buildInDirectives`: the specified directive names.
static BUILD_IN_DIRECTIVES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_rs::specified_directives()
        .iter()
        .map(|d| d.name.value.clone())
        .collect()
});

fn is_built_in_directive(directive_name: &str) -> bool {
    BUILD_IN_DIRECTIVES.contains(directive_name)
}

fn is_federation_directive(name: &str) -> bool {
    name == "tag" || name == "link" || name.starts_with("join__") || name.starts_with("link__")
}

fn is_federation_enum(name: &str) -> bool {
    name.starts_with("join__") || name.starts_with("link__")
}

fn is_federation_scalar(name: &str) -> bool {
    name.starts_with("join__") || name.starts_with("link__")
}

/// `schemaCoordinate(paths, nodes)`: graphql-js visitor `path` and `ancestors`.
/// NOTE(port): the TS reads `node.name.value` of the list element at each numeric path key;
/// an element without a name (a `SchemaDefinition`) is a `TypeError` there. The supergraph
/// never has `@inaccessible` on the schema definition, so this panics with the JS message.
pub fn schema_coordinate(paths: &[PathKey], nodes: &[Ancestor<'_>]) -> String {
    let mut coordinate = String::new();
    for i in 0..paths.len().max(nodes.len()) {
        let prop = paths.get(i);
        let current = nodes.get(i);

        if let (Some(PathKey::Index(prop)), Some(Ancestor::List(current))) = (prop, current) {
            let name = current
                .get(*prop)
                .and_then(|node| node.name())
                .expect("Cannot read properties of undefined (reading 'value')");

            if !coordinate.is_empty() {
                coordinate = coordinate + "." + &name.value;
            } else {
                coordinate = name.value.clone();
            }
        }
    }

    coordinate
}

/// The first visitor of `stripFederation`'s `visitInParallel`: removes federation and built-in
/// directive definitions and usages, and federation enums and scalars.
struct StripFederationVisitor;

impl EditVisitor for StripFederationVisitor {
    fn enter(&mut self, node: NodeRef<'_>, _info: &VisitInfo<'_, '_>) -> EditAction {
        match node {
            NodeRef::DirectiveDefinition(node) => {
                if is_built_in_directive(&node.name.value) || is_federation_directive(&node.name.value) {
                    return EditAction::Delete;
                }
            }
            NodeRef::Directive(node) => {
                if is_built_in_directive(&node.name.value) || is_federation_directive(&node.name.value) {
                    return EditAction::Delete;
                }
            }
            NodeRef::EnumTypeDefinition(node) => {
                if is_federation_enum(&node.name.value) {
                    return EditAction::Delete;
                }
            }
            NodeRef::ScalarTypeDefinition(node) if is_federation_scalar(&node.name.value) => {
                return EditAction::Delete;
            }
            _ => {}
        }
        EditAction::Continue
    }
}

/// The second visitor of `stripFederation`'s `visitInParallel`: collects the schema coordinates
/// annotated with `@inaccessible`.
struct CollectInaccessibleVisitor<'i> {
    inaccessible: &'i mut IndexSet<String>,
}

impl EditVisitor for CollectInaccessibleVisitor<'_> {
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        if let NodeRef::Directive(directive) = node {
            if directive.name.value == "inaccessible" {
                self.inaccessible.insert(schema_coordinate(info.path, info.ancestors));
            }
        }
        EditAction::Continue
    }
}

/// The visitor of `stripFederation`'s second phase (`hideByNodeName`, `hideObjectOrInterface`,
/// `hideField`, `EnumValueDefinition`).
struct HideInaccessibleVisitor<'i> {
    inaccessible: &'i IndexSet<String>,
}

impl HideInaccessibleVisitor<'_> {
    /// `hideByNodeName(node)`.
    fn hide_by_node_name(&self, name: &Name) -> EditAction {
        if self.inaccessible.contains(&name.value) {
            return EditAction::Delete;
        }
        EditAction::Continue
    }

    /// `hideObjectOrInterface(node)`: `None` when the interfaces are left as they are.
    fn hide_object_or_interface(&self, name: &Name, interfaces: &[NamedType]) -> Result<Option<Vec<NamedType>>, ()> {
        if self.inaccessible.contains(&name.value) {
            return Err(());
        }

        let has_inaccessible_interfaces = interfaces.iter().any(|i| self.inaccessible.contains(&i.name.value));

        if has_inaccessible_interfaces {
            return Ok(Some(
                interfaces
                    .iter()
                    .filter(|i| !self.inaccessible.contains(&i.name.value))
                    .cloned()
                    .collect(),
            ));
        }

        Ok(None)
    }

    /// `hideField(node, _, __, paths, nodes)`.
    fn hide_field(&self, name: &Name, ty: &Type, info: &VisitInfo<'_, '_>) -> EditAction {
        // TODO (from the TS): check why we need to add `node.name.value` here
        if self
            .inaccessible
            .contains(&(schema_coordinate(info.path, info.ancestors) + "." + &name.value))
        {
            return EditAction::Delete;
        }

        if self.inaccessible.contains(&named_type_from_type_node(ty).name.value) {
            return EditAction::Delete;
        }

        EditAction::Continue
    }
}

impl EditVisitor for HideInaccessibleVisitor<'_> {
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        match node {
            NodeRef::ObjectTypeDefinition(node) => match self.hide_object_or_interface(&node.name, &node.interfaces) {
                Err(()) => EditAction::Delete,
                Ok(Some(interfaces)) => EditAction::Replace(Node::ObjectTypeDefinition(ObjectTypeDefinition {
                    interfaces,
                    ..node.clone()
                })),
                Ok(None) => EditAction::Continue,
            },
            NodeRef::InterfaceTypeDefinition(node) => match self.hide_object_or_interface(&node.name, &node.interfaces)
            {
                Err(()) => EditAction::Delete,
                Ok(Some(interfaces)) => EditAction::Replace(Node::InterfaceTypeDefinition(InterfaceTypeDefinition {
                    interfaces,
                    ..node.clone()
                })),
                Ok(None) => EditAction::Continue,
            },
            NodeRef::InputObjectTypeDefinition(node) => self.hide_by_node_name(&node.name),
            NodeRef::EnumTypeDefinition(node) => self.hide_by_node_name(&node.name),
            NodeRef::UnionTypeDefinition(node) => self.hide_by_node_name(&node.name),
            NodeRef::ScalarTypeDefinition(node) => self.hide_by_node_name(&node.name),
            NodeRef::FieldDefinition(node) => self.hide_field(&node.name, &node.ty, info),
            NodeRef::InputValueDefinition(node) => self.hide_field(&node.name, &node.ty, info),
            NodeRef::EnumValueDefinition(node) => {
                if self
                    .inaccessible
                    .contains(&(schema_coordinate(info.path, info.ancestors) + "." + &node.name.value))
                {
                    return EditAction::Delete;
                }
                EditAction::Continue
            }
            _ => EditAction::Continue,
        }
    }
}

/// `stripFederation(supergraph)`: removes federation directives and types and hides
/// `@inaccessible` parts. The visitor callbacks become `graphql_rs::EditVisitor` structs.
/// It's safe to look for `link__` and `join__` prefixes as Federation v2 does not allow to
/// define type names that start with these prefixes.
pub fn strip_federation(supergraph: &Document) -> Document {
    let mut inaccessible: IndexSet<String> = IndexSet::default();
    let document_without_federation = {
        let mut visitor = visit_in_parallel_edit(vec![
            Box::new(StripFederationVisitor),
            Box::new(CollectInaccessibleVisitor {
                inaccessible: &mut inaccessible,
            }),
        ]);
        visit_edit(supergraph, &mut visitor).into_document()
    };

    // No need to remove inaccessible types as they are not included in the schema
    if inaccessible.is_empty() {
        return document_without_federation;
    }

    // Remove inaccessible parts of schema.
    // This is done in two phases:
    // - the first phase detects @inaccessible directives and creates a set of schema coordinates (already done at this point).
    // - the second phase removes inaccessible bits from the schema
    //
    // This algorithm assumes that a type with all its fields inaccessible is annotated with the @inaccessible directive (this needs to be enforced by the supergraph composition logic),
    // otherwise some pieces of the schema may be left behind.
    visit_edit(
        &document_without_federation,
        &mut HideInaccessibleVisitor {
            inaccessible: &inaccessible,
        },
    )
    .into_document()
}

fn named_type_from_type_node(ty: &Type) -> &NamedType {
    match ty {
        Type::Named(n) => n,
        Type::List(l) => named_type_from_type_node(&l.ty),
        Type::NonNull(n) => named_type_from_type_node(&n.ty),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print_js<'a>(node: impl Into<NodeRef<'a>>) -> String {
        graphql_rs::print(node)
    }

    fn sdl(source: &str) -> String {
        print_js(&graphql_rs::parse(source).unwrap())
    }

    #[test]
    fn object_type_join_type_and_implements() {
        let node = create_object_type_node(&ObjectTypeAst {
            name: "Book".into(),
            join: JoinAst {
                r#type: vec![
                    JoinTypeAst {
                        graph: "A".into(),
                        key: Some("id".into()),
                        extension: Some(true),
                        ..Default::default()
                    },
                    JoinTypeAst {
                        graph: "B".into(),
                        key: Some("".into()),
                        resolvable: Some(false),
                        is_interface_object: Some(false),
                        ..Default::default()
                    },
                ],
                implements: vec![JoinImplementsAst {
                    graph: "A".into(),
                    interface: "Media".into(),
                }],
                ..Default::default()
            },
            interfaces: vec!["Media".into()],
            fields: vec![FieldAst {
                name: "title".into(),
                r#type: "[String!]!".into(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            print_js(&node),
            "type Book implements Media @join__type(graph: A, key: \"id\", extension: true) \
             @join__type(graph: B, resolvable: false) @join__implements(graph: A, interface: \"Media\") {\n  title: [String!]!\n}"
        );
    }

    #[test]
    fn apply_directives_order() {
        let node = create_field_node(&FieldAst {
            name: "f".into(),
            r#type: "Int".into(),
            join: JoinAst {
                field: vec![JoinFieldAst {
                    graph: Some("A".into()),
                    external: Some(true),
                    used_overridden: Some(true),
                    provides: Some("".into()),
                    context_arguments: Some(vec![]),
                    ..Default::default()
                }],
                ..Default::default()
            },
            tags: vec!["t1".into(), "t2".into()],
            inaccessible: true,
            authenticated: true,
            policies: vec![vec!["p1".into()], vec!["p2".into(), "p3".into()]],
            scopes: vec![],
            cost: Some(CostAst {
                cost: 1.5,
                directive_name: "cost".into(),
            }),
            list_size: Some(ListSizeAst {
                assumed_size: Some(10.0),
                slicing_arguments: None,
                sized_fields: Some(vec!["edges".into()]),
                require_one_slicing_argument: true,
                print_require_one_slicing_argument: false,
                directive_name: "listSize".into(),
            }),
            deprecated: Some(Deprecated { reason: None }),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            print_js(&node),
            "f: Int @join__field(graph: A, usedOverridden: true, external: true) @tag(name: \"t1\") \
             @tag(name: \"t2\") @inaccessible @authenticated @policy(policies: [[\"p1\"], [\"p2\", \"p3\"]]) \
             @cost(weight: 1.5) @listSize(assumedSize: 10, sizedFields: [\"edges\"]) @deprecated"
        );
    }

    #[test]
    fn join_field_context_arguments() {
        let directive = create_join_field_directive_node(&JoinFieldAst {
            graph: Some("A".into()),
            r#type: Some("Int".into()),
            r#override: Some("b".into()),
            override_label: Some("percent(5)".into()),
            requires: Some("x".into()),
            context_arguments: Some(vec![JoinFieldContextArgumentAst {
                name: "n".into(),
                r#type: "String".into(),
                context: "A__c".into(),
                selection: "{ x }".into(),
            }]),
            ..Default::default()
        });
        assert_eq!(
            print_js(&directive),
            "@join__field(graph: A, type: \"Int\", override: \"b\", overrideLabel: \"percent(5)\", requires: \"x\", \
             contextArguments: [{ context: \"A__c\", name: \"n\", type: \"String\", selection: \"{ x }\" }])"
        );
    }

    #[test]
    fn list_size_require_one_slicing_argument() {
        let mut input = ListSizeAst {
            assumed_size: None,
            slicing_arguments: Some(vec![]),
            sized_fields: None,
            require_one_slicing_argument: false,
            print_require_one_slicing_argument: false,
            directive_name: "listSize".into(),
        };
        assert_eq!(
            print_js(&create_list_size_directive_node(&input)),
            "@listSize(requireOneSlicingArgument: false, slicingArguments: [])"
        );
        input.require_one_slicing_argument = true;
        input.print_require_one_slicing_argument = true;
        assert_eq!(
            print_js(&create_list_size_directive_node(&input)),
            "@listSize(requireOneSlicingArgument: true, slicingArguments: [])"
        );
    }

    #[test]
    fn cost_weight_uses_js_number_formatting() {
        let directive = create_cost_directive_node(&CostAst {
            cost: 1e21,
            directive_name: "myCost".into(),
        });
        assert_eq!(print_js(&directive), "@myCost(weight: 1e+21)");
    }

    #[test]
    fn description_is_always_block() {
        let node = create_scalar_type_node(&ScalarTypeAst {
            name: "S".into(),
            description: Some(Description {
                value: "desc".into(),
                block: false,
            }),
            specified_by: Some("".into()),
            ..Default::default()
        });
        assert!(node.description.as_ref().unwrap().block);
        assert_eq!(print_js(&node), "\"\"\"desc\"\"\"\nscalar S");
    }

    #[test]
    fn unparsable_printed_types_and_values_throw_the_parser_error() {
        // A name that is not a GraphQL name can only come from a document that was not parsed.
        assert_eq!(
            parse_type_string("Foo Bar").unwrap_err(),
            JsError::GraphQL(graphql_rs::GraphQLError::new(
                "Syntax Error: Expected <EOF>, found Name \"Bar\"."
            ))
        );
        assert_eq!(
            create_default_value(Some("{"), ArgumentKind::Scalar).unwrap_err(),
            JsError::GraphQL(graphql_rs::GraphQLError::new(
                "Syntax Error: Expected Name, found <EOF>."
            ))
        );
        let err = create_field_node(&FieldAst {
            name: "f".into(),
            r#type: "[Int".into(),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(err.to_string(), "Syntax Error: Expected \"]\", found <EOF>.");
    }

    #[test]
    fn default_values() {
        // ENUM kind: surrounding quotes are dropped, the value becomes an enum value.
        assert_eq!(
            create_default_value(Some("\"A\""), ArgumentKind::Enum).unwrap(),
            Some(ConstValue::enum_value("A"))
        );
        // `'"'.substring(1, 0)` is `'"'`.
        assert_eq!(
            create_default_value(Some("\""), ArgumentKind::Enum).unwrap(),
            Some(ConstValue::enum_value("\""))
        );
        assert_eq!(
            create_default_value(Some("B"), ArgumentKind::Enum).unwrap(),
            Some(ConstValue::enum_value("B"))
        );
        assert_eq!(create_default_value(None, ArgumentKind::Scalar).unwrap(), None);
        assert_eq!(
            create_default_value(Some("\"A\""), ArgumentKind::Scalar).unwrap(),
            Some(ConstValue::string("A"))
        );
        let arg = create_field_argument_node(&FieldArgumentAst {
            name: "a".into(),
            r#type: "[Int]".into(),
            default_value: Some("[1, 2]".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(print_js(&arg), "a: [Int] = [1, 2]");
    }

    #[test]
    fn schema_node() {
        let node = create_schema_node(&SchemaAst {
            query: Some("Query".into()),
            mutation: Some("".into()),
            subscription: None,
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
                    import: Some(vec![
                        LinkImportAst {
                            name: "@lowercase".into(),
                            alias: None,
                        },
                        LinkImportAst {
                            name: "@upper".into(),
                            alias: Some("@up".into()),
                        },
                    ]),
                    ..Default::default()
                },
                LinkAst {
                    url: "u".into(),
                    import: Some(vec![]),
                    r#for: Some("".into()),
                },
            ],
        });
        assert_eq!(
            print_js(&node),
            "schema @link(url: \"https://specs.apollo.dev/link/v1.0\") \
             @link(url: \"https://specs.apollo.dev/join/v0.3\", for: EXECUTION) \
             @link(url: \"https://myspecs.dev/lowercase/v1.0\", import: [\"@lowercase\", { name: \"@upper\" }]) \
             @link(url: \"u\", import: []) {\n  query: Query\n}"
        );
    }

    #[test]
    fn join_graph_directive() {
        // `createJoinGraphEnumTypeNode` itself dedupes the value directives with the library
        // printer (`crate::graphql::printer`), so only its directives are checked here.
        assert_eq!(
            print_js(&create_join_graph_directive_node(
                "products",
                Some("http://products.com")
            )),
            "@join__graph(name: \"products\", url: \"http://products.com\")"
        );
        assert_eq!(
            print_js(&create_join_graph_directive_node("reviews", None)),
            "@join__graph(name: \"reviews\", url: \"\")"
        );
    }

    #[test]
    fn join_graph_enum() {
        let node = create_join_graph_enum_type_node(&[
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
        ]);
        assert_eq!(
            print_js(&node),
            "enum join__Graph {\n  PRODUCTS @join__graph(name: \"products\", url: \"http://products.com\")\n  \
             REVIEWS @join__graph(name: \"reviews\", url: \"\")\n}"
        );
    }

    #[test]
    fn union_and_input_object() {
        let union = create_union_type_node(&UnionTypeAst {
            name: "U".into(),
            members: vec!["A".into(), "B".into()],
            contexts: vec!["g__ctx".into()],
            join: JoinAst {
                union_member: vec![JoinUnionMemberAst {
                    graph: "G".into(),
                    member: "A".into(),
                }],
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            print_js(&union),
            "union U @join__unionMember(graph: G, member: \"A\") @context(name: \"g__ctx\") = A | B"
        );

        let input = create_input_object_type_node(&InputObjectTypeAst {
            name: "I".into(),
            is_one_of: true,
            fields: vec![InputFieldAst {
                name: "a".into(),
                r#type: "E".into(),
                default_value: Some("FOO".into()),
                deprecated: Some(Deprecated {
                    reason: Some("old".into()),
                }),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            print_js(&input),
            "input I @oneOf {\n  a: E = FOO @deprecated(reason: \"old\")\n}"
        );
    }

    #[test]
    fn strip_federation_directives_and_types() {
        let doc = graphql_rs::parse(
            r#"
            schema
              @link(url: "https://specs.apollo.dev/link/v1.0")
              @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) {
              query: Query
            }
            directive @join__graph on ENUM_VALUE
            directive @lowercase on ENUM_VALUE
            directive @deprecated on FIELD_DEFINITION
            directive @tag on OBJECT
            scalar link__Import
            scalar join__FieldSet
            scalar JSON
            enum join__Graph { A }
            enum link__Purpose { EXECUTION }
            enum Foo {
              PANDAS @join__graph(name: "pandas", url: "") @lowercase
              USERS @join__enumValue(graph: C)
            }
            type Query @join__type(graph: A) @tag(name: "x") {
              a: String @join__field @deprecated
            }
            "#,
        )
        .unwrap();
        assert_eq!(
            print_js(&strip_federation(&doc)),
            sdl(r#"
            schema { query: Query }
            directive @lowercase on ENUM_VALUE
            scalar JSON
            enum Foo { PANDAS @lowercase USERS }
            type Query { a: String }
            "#)
        );
    }

    #[test]
    fn strip_federation_inaccessible() {
        let doc = graphql_rs::parse(
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

            interface Node @inaccessible {
              id: ID!
            }

            interface Named {
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

            type User implements Node & Named {
              id: ID!
              name: String
              totalProductsCreated: Int @inaccessible
              profile: Profile
              role: Role
              access: Access
            }
            "#,
        )
        .unwrap();
        assert_eq!(
            print_js(&strip_federation(&doc)),
            sdl(r#"
            schema {
              query: Query
            }

            type Panda {
              name: String!
            }

            interface Named {
              name: String
            }

            type Query {
              users(anotherFilter: AnotherFilter): [User]
              panda(name: String!): Panda
            }

            input AnotherFilter {
              after: ID
            }

            enum Access {
              READ
            }

            type User implements Named {
              id: ID!
              name: String
              access: Access
            }
            "#)
        );
    }

    #[test]
    fn schema_coordinate_from_visit_info() {
        struct Collect(Vec<String>);
        impl<'a> graphql_rs::Visitor<'a> for Collect {
            fn enter(&mut self, node: NodeRef<'a>, _cx: &mut (), info: &VisitInfo<'a, '_>) -> graphql_rs::VisitAction {
                if let NodeRef::Directive(_) = node {
                    self.0.push(schema_coordinate(info.path, info.ancestors));
                }
                graphql_rs::VisitAction::Continue
            }
        }
        let doc = graphql_rs::parse(
            "type A @d { f(a: Int @d): Int @d } enum E { V @d } input I { x: Int @d } directive @x(a: Int @d) on FIELD",
        )
        .unwrap();
        let mut v = Collect(vec![]);
        graphql_rs::visit(&doc, &mut v, &mut ());
        assert_eq!(v.0, vec!["A", "A.f.a", "A.f", "E.V", "I.x", "x.a"]);
    }
}
