//! Port of `src/supergraph/composition/ast.ts`: builders of supergraph AST nodes.
//!
//! The TS parameter object types (`ObjectTypeAST = inferArgument<typeof createObjectTypeNode>`)
//! are the `*Ast` structs below. Optional arrays (`tags?: string[]`) are `Vec`s (the TS
//! treats `undefined` and `[]` alike there), optional flags are `bool` (falsy when missing),
//! and other optional values are `Option`.

use std::sync::LazyLock;

use graphql_js::{
    Ancestor, ConstDirective, ConstObjectField, ConstValue, DirectiveDefinition, Document, EnumTypeDefinition,
    EnumValueDefinition, FieldDefinition, InputObjectTypeDefinition, InputValueDefinition, InterfaceTypeDefinition,
    NamedType, ObjectTypeDefinition, PathKey, ScalarTypeDefinition, SchemaDefinition, StringValue, Type,
    UnionTypeDefinition,
};
use indexmap::IndexSet;

use crate::subgraph::state::{ArgumentKind, Deprecated, Description};
use crate::utils::js::JsResult;

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

/// The argument of `applyDirectives`: the union of the properties the builders pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommonDirectivesAst<'c> {
    pub ast: &'c [ConstDirective],
    pub join: Option<&'c JoinAst>,
    pub deprecated: Option<&'c Deprecated>,
    pub specified_by: Option<&'c str>,
    pub tags: &'c [String],
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
    todo!()
}

pub fn create_directive_node(directive: &DirectiveAst) -> DirectiveDefinition {
    todo!()
}

pub fn create_object_type_node(object_type: &ObjectTypeAst) -> ObjectTypeDefinition {
    todo!()
}

pub fn create_interface_type_node(interface_type: &InterfaceTypeAst) -> InterfaceTypeDefinition {
    todo!()
}

pub fn create_input_object_type_node(input_object_type: &InputObjectTypeAst) -> InputObjectTypeDefinition {
    todo!()
}

pub fn create_union_type_node(union_type: &UnionTypeAst) -> UnionTypeDefinition {
    todo!()
}

pub fn create_enum_type_node(enum_type: &EnumTypeAst) -> EnumTypeDefinition {
    todo!()
}

pub fn create_scalar_type_node(scalar_type: &ScalarTypeAst) -> ScalarTypeDefinition {
    todo!()
}

/// `createJoinGraphEnumTypeNode(graphs)`: the `join__Graph` enum.
pub fn create_join_graph_enum_type_node(graphs: &[JoinGraphAst]) -> EnumTypeDefinition {
    todo!()
}

fn create_field_node(field: &FieldAst) -> FieldDefinition {
    todo!()
}

fn create_input_field_node(input_field: &InputFieldAst) -> InputValueDefinition {
    todo!()
}

fn create_enum_value_node(enum_value: &EnumValueAst) -> EnumValueDefinition {
    todo!()
}

/// `createDefaultValue(defaultValue, kind)`: parses the printed default value back into a
/// const value node.
/// NOTE(port): the body uses graphql-js `parseConstValue`; it throws on invalid input, which
/// validation rules out before composition, so the port may `expect` with the parser message.
fn create_default_value(default_value: Option<&str>, kind: ArgumentKind) -> Option<ConstValue> {
    todo!()
}

fn create_field_argument_node(argument: &FieldArgumentAst) -> InputValueDefinition {
    todo!()
}

fn create_join_type_directive_node(join: &JoinTypeAst) -> ConstDirective {
    todo!()
}

fn create_join_implements_directive_node(join: &JoinImplementsAst) -> ConstDirective {
    todo!()
}

fn create_join_field_directive_node(join: &JoinFieldAst) -> ConstDirective {
    todo!()
}

fn create_string_object_field_node(name: &str, value: &str) -> ConstObjectField {
    ConstObjectField::new(name, ConstValue::string(value))
}

/// `createContextDirectiveNode({ name })`.
fn create_context_directive_node(name: &str) -> ConstDirective {
    todo!()
}

fn create_join_union_member_directive_node(join: &JoinUnionMemberAst) -> ConstDirective {
    todo!()
}

fn create_join_enum_value_directive_node(join: &JoinEnumValueAst) -> ConstDirective {
    todo!()
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

fn create_policy_directive_node(policies: &[Vec<String>]) -> ConstDirective {
    todo!()
}

fn create_requires_scopes_directive_node(scopes: &[Vec<String>]) -> ConstDirective {
    todo!()
}

fn create_tag_directive_node(name: &str) -> ConstDirective {
    ConstDirective::new("tag", vec![graphql_js::Argument::new("name", ConstValue::string(name))])
}

/// `createJoinGraphDirectiveNode({ name, url? })`.
fn create_join_graph_directive_node(name: &str, url: Option<&str>) -> ConstDirective {
    todo!()
}

fn create_description_node(description: &DescriptionAst) -> StringValue {
    StringValue::new(description.value.clone(), description.block)
}

fn create_deprecated_directive_node(deprecated: &Deprecated) -> ConstDirective {
    todo!()
}

fn create_specified_by_directive_node(url: &str) -> ConstDirective {
    ConstDirective::new("specifiedBy", vec![graphql_js::Argument::new("url", ConstValue::string(url))])
}

fn create_cost_directive_node(input: &CostAst) -> ConstDirective {
    todo!()
}

fn create_list_size_directive_node(input: &ListSizeAst) -> ConstDirective {
    todo!()
}

fn create_link_directive_node(link: &LinkAst) -> ConstDirective {
    todo!()
}

fn create_named_type_node(name: &str) -> NamedType {
    NamedType::new(name)
}

/// `applyDirectives(common)`: the node's own directives (deduplicated by their printed form,
/// library printer) followed by the generated federation directives.
fn apply_directives(common: &CommonDirectivesAst<'_>) -> Vec<ConstDirective> {
    todo!()
}

fn non_empty<T>(value: &Option<T>) -> bool {
    value.is_some()
}

/// `buildInDirectives`: the specified directive names.
static BUILD_IN_DIRECTIVES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_js::specified_directives()
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
pub fn schema_coordinate(paths: &[PathKey], nodes: &[Ancestor<'_>]) -> String {
    todo!()
}

/// `stripFederation(supergraph)`: removes federation directives and types and hides
/// `@inaccessible` parts. The visitor callbacks become `graphql_js::EditVisitor` structs.
pub fn strip_federation(supergraph: &Document) -> Document {
    todo!()
}

fn named_type_from_type_node(ty: &Type) -> &NamedType {
    match ty {
        Type::Named(n) => n,
        Type::List(l) => named_type_from_type_node(&l.ty),
        Type::NonNull(n) => named_type_from_type_node(&n.ty),
    }
}
