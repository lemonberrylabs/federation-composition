//! Port of graphql-js `type/definition`: the named types, wrapping types,
//! fields, arguments, enum values and input fields of a schema, and the type
//! predicates.
//!
//! graphql-js types are objects compared by identity. Within one schema every
//! named type is a single object per name (`GraphQLSchema` rejects two types
//! with the same name), so here a named type is referenced by its name
//! ([`GraphQLType::Named`]) and looked up through a [`TypeLookup`] (the schema,
//! or the specified scalars while a schema is being built). Wrapping types
//! are values; graphql-js compares them structurally too wherever it matters
//! (`isEqualType`, `isTypeSubTypeOf` recurse when identity fails).
//!
//! The `fields`, `interfaces`, `types` and `values` thunks graphql-js
//! resolves on first access are [`Lazy`] values: building a schema with
//! [`crate::build_ast_schema`] resolves them in graphql-js order, so a thunk
//! that throws makes the same call fail.

// The type system is not part of the public API yet: outside the crate it is reachable only
// through the unstable `__internal` module, for the differential tests.
#![allow(missing_docs)]

use std::fmt;
use std::sync::{Arc, OnceLock};

use crate::ast::{
    EnumTypeDefinition, EnumTypeExtension, EnumValueDefinition, FieldDefinition, InputObjectTypeDefinition,
    InputObjectTypeExtension, InputValueDefinition, InterfaceTypeDefinition, InterfaceTypeExtension,
    ObjectTypeDefinition, ObjectTypeExtension, ScalarTypeDefinition, ScalarTypeExtension, UnionTypeDefinition,
    UnionTypeExtension, Value,
};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::jsutils::did_you_mean::did_you_mean;
use crate::jsutils::inspect::inspect;
use crate::jsutils::suggestion_list::suggestion_list;
use crate::jsutils::JsValue;
use crate::printer::print;
use crate::utilities::value_from_ast_untyped::value_from_ast_untyped;

use super::assert_name::{assert_enum_value_name, assert_name};
use super::scalars;

/// A graphql-js thunk (`() => T`) that may throw.
pub type Thunk<T> = Arc<dyn Fn() -> Result<T, GraphQLError> + Send + Sync>;

/// A value graphql-js computes from a thunk on first access and then keeps
/// (`getFields()`, `getInterfaces()`, `getTypes()`, `getValues()`). A thunk
/// that throws is not kept, so the next access runs it again, as in
/// graphql-js.
pub struct Lazy<T> {
    value: OnceLock<T>,
    thunk: Option<Thunk<T>>,
}

impl<T> Lazy<T> {
    /// An already resolved value (a graphql-js config passing a value, not a thunk).
    pub fn new(value: T) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(value);
        Lazy {
            value: cell,
            thunk: None,
        }
    }

    pub fn thunk(thunk: Thunk<T>) -> Self {
        Lazy {
            value: OnceLock::new(),
            thunk: Some(thunk),
        }
    }

    /// The value, running the thunk if it was not resolved yet.
    pub fn get(&self) -> Result<&T, GraphQLError> {
        if let Some(value) = self.value.get() {
            return Ok(value);
        }
        let thunk = self.thunk.as_ref().expect("a Lazy without a value has a thunk");
        let value = thunk()?;
        Ok(self.value.get_or_init(|| value))
    }

    /// The value of a thunk known to be resolved: `GraphQLSchema::new`
    /// resolves every `fields`, `interfaces` and `types` thunk of its types.
    pub fn resolved(&self) -> &T {
        self.value.get().expect("resolved when the schema was constructed")
    }
}

impl<T: Clone> Clone for Lazy<T> {
    fn clone(&self) -> Self {
        Lazy {
            value: self.value.clone(),
            thunk: self.thunk.clone(),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Lazy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value.get() {
            Some(value) => value.fmt(f),
            None => f.write_str("<thunk>"),
        }
    }
}

/// graphql-js `GraphQLType`: a named type (by name) or a wrapping type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphQLType {
    Named(String),
    /// graphql-js `GraphQLList`.
    List(Box<GraphQLType>),
    /// graphql-js `GraphQLNonNull`.
    NonNull(Box<GraphQLType>),
}

impl GraphQLType {
    pub fn named(name: impl Into<String>) -> Self {
        GraphQLType::Named(name.into())
    }

    pub fn list(of_type: GraphQLType) -> Self {
        GraphQLType::List(Box::new(of_type))
    }

    pub fn non_null(of_type: GraphQLType) -> Self {
        GraphQLType::NonNull(Box::new(of_type))
    }

    /// graphql-js `getNamedType`: the name of the innermost named type.
    pub fn named_type(&self) -> &str {
        match self {
            GraphQLType::Named(name) => name,
            GraphQLType::List(of_type) | GraphQLType::NonNull(of_type) => of_type.named_type(),
        }
    }

    /// graphql-js `isNonNullType`.
    pub fn is_non_null(&self) -> bool {
        matches!(self, GraphQLType::NonNull(_))
    }

    /// graphql-js `isListType`.
    pub fn is_list(&self) -> bool {
        matches!(self, GraphQLType::List(_))
    }

    /// graphql-js `isWrappingType`.
    pub fn is_wrapping(&self) -> bool {
        !matches!(self, GraphQLType::Named(_))
    }

    /// `ofType` of a wrapping type.
    pub fn of_type(&self) -> Option<&GraphQLType> {
        match self {
            GraphQLType::Named(_) => None,
            GraphQLType::List(of_type) | GraphQLType::NonNull(of_type) => Some(of_type),
        }
    }

    /// graphql-js `getNullableType`.
    pub fn nullable(&self) -> &GraphQLType {
        match self {
            GraphQLType::NonNull(of_type) => of_type,
            other => other,
        }
    }
}

impl fmt::Display for GraphQLType {
    /// graphql-js `String(type)` (`toString`, also what `inspect` prints).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLType::Named(name) => f.write_str(name),
            GraphQLType::List(of_type) => write!(f, "[{of_type}]"),
            GraphQLType::NonNull(of_type) => write!(f, "{of_type}!"),
        }
    }
}

/// Resolves the named types a [`GraphQLType`] refers to.
pub trait TypeLookup {
    fn lookup_type(&self, name: &str) -> Option<&GraphQLNamedType>;
}

impl TypeLookup for IndexMap<String, GraphQLNamedType> {
    fn lookup_type(&self, name: &str) -> Option<&GraphQLNamedType> {
        self.get(name)
    }
}

fn named<'t>(types: &'t dyn TypeLookup, ty: &GraphQLType) -> Option<&'t GraphQLNamedType> {
    match ty {
        GraphQLType::Named(name) => types.lookup_type(name),
        _ => None,
    }
}

/// graphql-js `isInputType`.
pub fn is_input_type(types: &dyn TypeLookup, ty: &GraphQLType) -> bool {
    match ty {
        GraphQLType::Named(_) => matches!(
            named(types, ty),
            Some(GraphQLNamedType::Scalar(_) | GraphQLNamedType::Enum(_) | GraphQLNamedType::InputObject(_))
        ),
        GraphQLType::List(of_type) | GraphQLType::NonNull(of_type) => is_input_type(types, of_type),
    }
}

/// graphql-js `isOutputType`.
pub fn is_output_type(types: &dyn TypeLookup, ty: &GraphQLType) -> bool {
    match ty {
        GraphQLType::Named(_) => matches!(
            named(types, ty),
            Some(
                GraphQLNamedType::Scalar(_)
                    | GraphQLNamedType::Object(_)
                    | GraphQLNamedType::Interface(_)
                    | GraphQLNamedType::Union(_)
                    | GraphQLNamedType::Enum(_)
            )
        ),
        GraphQLType::List(of_type) | GraphQLType::NonNull(of_type) => is_output_type(types, of_type),
    }
}

/// graphql-js `GraphQLNamedType`.
#[derive(Clone, Debug)]
pub enum GraphQLNamedType {
    Scalar(GraphQLScalarType),
    Object(GraphQLObjectType),
    Interface(GraphQLInterfaceType),
    Union(GraphQLUnionType),
    Enum(GraphQLEnumType),
    InputObject(GraphQLInputObjectType),
}

impl GraphQLNamedType {
    pub fn name(&self) -> &str {
        match self {
            GraphQLNamedType::Scalar(t) => &t.name,
            GraphQLNamedType::Object(t) => &t.name,
            GraphQLNamedType::Interface(t) => &t.name,
            GraphQLNamedType::Union(t) => &t.name,
            GraphQLNamedType::Enum(t) => &t.name,
            GraphQLNamedType::InputObject(t) => &t.name,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            GraphQLNamedType::Scalar(t) => t.description.as_deref(),
            GraphQLNamedType::Object(t) => t.description.as_deref(),
            GraphQLNamedType::Interface(t) => t.description.as_deref(),
            GraphQLNamedType::Union(t) => t.description.as_deref(),
            GraphQLNamedType::Enum(t) => t.description.as_deref(),
            GraphQLNamedType::InputObject(t) => t.description.as_deref(),
        }
    }

    /// graphql-js `isObjectType`.
    pub fn is_object_type(&self) -> bool {
        matches!(self, GraphQLNamedType::Object(_))
    }

    /// graphql-js `isInterfaceType`.
    pub fn is_interface_type(&self) -> bool {
        matches!(self, GraphQLNamedType::Interface(_))
    }

    /// graphql-js `isUnionType`.
    pub fn is_union_type(&self) -> bool {
        matches!(self, GraphQLNamedType::Union(_))
    }

    /// graphql-js `isInputObjectType`.
    pub fn is_input_object_type(&self) -> bool {
        matches!(self, GraphQLNamedType::InputObject(_))
    }

    /// graphql-js `isAbstractType`.
    pub fn is_abstract_type(&self) -> bool {
        matches!(self, GraphQLNamedType::Interface(_) | GraphQLNamedType::Union(_))
    }

    /// graphql-js `isLeafType`.
    pub fn is_leaf_type(&self) -> bool {
        matches!(self, GraphQLNamedType::Scalar(_) | GraphQLNamedType::Enum(_))
    }

    /// graphql-js `isCompositeType`.
    pub fn is_composite_type(&self) -> bool {
        matches!(
            self,
            GraphQLNamedType::Object(_) | GraphQLNamedType::Interface(_) | GraphQLNamedType::Union(_)
        )
    }
}

impl fmt::Display for GraphQLNamedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The input value a default is given as: graphql-js `{ literal }` (a
/// default from SDL) or `{ value }` (a default given in code, such as the
/// introspection types' and specified directives').
#[derive(Clone, Debug, PartialEq)]
pub enum DefaultInput {
    Literal(Value),
    Value(JsValue),
}

/// Which coercion functions a scalar has: the specified scalars' (see
/// `type/scalars`), or none, as for a scalar built from SDL (identity
/// coercion and the default `parseLiteral`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarKind {
    Custom,
    String,
    Int,
    Float,
    Boolean,
    Id,
}

/// graphql-js `GraphQLScalarTypeConfig`, reduced to what SDL can express.
#[derive(Clone, Debug, Default)]
pub struct GraphQLScalarTypeConfig {
    pub name: String,
    pub description: Option<String>,
    pub specified_by_url: Option<String>,
    pub ast_node: Option<ScalarTypeDefinition>,
    pub extension_ast_nodes: Vec<ScalarTypeExtension>,
}

/// graphql-js `GraphQLScalarType`.
#[derive(Clone, Debug)]
pub struct GraphQLScalarType {
    pub name: String,
    pub description: Option<String>,
    pub specified_by_url: Option<String>,
    pub ast_node: Option<ScalarTypeDefinition>,
    pub extension_ast_nodes: Vec<ScalarTypeExtension>,
    pub kind: ScalarKind,
}

impl GraphQLScalarType {
    pub fn new(config: GraphQLScalarTypeConfig) -> Result<Self, GraphQLError> {
        Self::with_kind(config, ScalarKind::Custom)
    }

    pub(crate) fn with_kind(config: GraphQLScalarTypeConfig, kind: ScalarKind) -> Result<Self, GraphQLError> {
        assert_name(&config.name)?;
        Ok(GraphQLScalarType {
            name: config.name,
            description: config.description,
            specified_by_url: config.specified_by_url,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
            kind,
        })
    }

    /// `scalar.coerceInputValue(inputValue)`.
    pub fn coerce_input_value(&self, input_value: &JsValue) -> Result<JsValue, GraphQLError> {
        match self.kind {
            // `config.coerceInputValue ?? identityFunc`.
            ScalarKind::Custom => Ok(input_value.clone()),
            kind => scalars::coerce_input_value(kind, input_value),
        }
    }

    /// Whether the scalar has `coerceInputLiteral` (the specified scalars do;
    /// a scalar built from SDL has only the default `parseLiteral`).
    pub fn has_coerce_input_literal(&self) -> bool {
        self.kind != ScalarKind::Custom
    }

    /// `scalar.coerceInputLiteral(valueNode)` of a specified scalar.
    pub fn coerce_input_literal(&self, value_node: &Value) -> Result<JsValue, GraphQLError> {
        scalars::coerce_input_literal(self.kind, value_node)
    }

    /// The default `parseLiteral` of a scalar without one:
    /// `this.coerceInputValue(valueFromASTUntyped(node, variables))`; `None` is
    /// `undefined`.
    pub fn parse_literal(&self, value_node: &Value) -> Result<Option<JsValue>, GraphQLError> {
        match value_from_ast_untyped(value_node) {
            None => Ok(None),
            Some(value) => self.coerce_input_value(&value).map(Some),
        }
    }
}

/// graphql-js `GraphQLFieldConfig`.
#[derive(Clone, Debug)]
pub struct GraphQLFieldConfig {
    pub description: Option<String>,
    pub ty: GraphQLType,
    pub args: IndexMap<String, GraphQLArgumentConfig>,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<FieldDefinition>,
}

/// graphql-js `GraphQLArgumentConfig`.
#[derive(Clone, Debug)]
pub struct GraphQLArgumentConfig {
    pub description: Option<String>,
    pub ty: GraphQLType,
    pub default: Option<DefaultInput>,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<InputValueDefinition>,
}

/// graphql-js `GraphQLObjectTypeConfig`.
pub struct GraphQLObjectTypeConfig {
    pub name: String,
    pub description: Option<String>,
    pub interfaces: Lazy<Vec<String>>,
    pub fields: Lazy<IndexMap<String, GraphQLFieldConfig>>,
    pub ast_node: Option<ObjectTypeDefinition>,
    pub extension_ast_nodes: Vec<ObjectTypeExtension>,
}

/// graphql-js `GraphQLObjectType`.
#[derive(Clone, Debug)]
pub struct GraphQLObjectType {
    pub name: String,
    pub description: Option<String>,
    pub ast_node: Option<ObjectTypeDefinition>,
    pub extension_ast_nodes: Vec<ObjectTypeExtension>,
    fields: Lazy<IndexMap<String, GraphQLField>>,
    interfaces: Lazy<Vec<String>>,
}

impl GraphQLObjectType {
    pub fn new(config: GraphQLObjectTypeConfig) -> Result<Self, GraphQLError> {
        assert_name(&config.name)?;
        Ok(GraphQLObjectType {
            fields: define_field_map(&config.name, config.fields),
            interfaces: config.interfaces,
            name: config.name,
            description: config.description,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
        })
    }

    /// graphql-js `getFields()`.
    pub fn get_fields(&self) -> Result<&IndexMap<String, GraphQLField>, GraphQLError> {
        self.fields.get()
    }

    /// graphql-js `getInterfaces()` (names).
    pub fn get_interfaces(&self) -> Result<&Vec<String>, GraphQLError> {
        self.interfaces.get()
    }

    /// `getFields()` of a type in a constructed schema.
    pub fn fields(&self) -> &IndexMap<String, GraphQLField> {
        self.fields.resolved()
    }

    /// `getInterfaces()` of a type in a constructed schema.
    pub fn interfaces(&self) -> &[String] {
        self.interfaces.resolved()
    }
}

/// graphql-js `defineFieldMap`.
fn define_field_map(
    parent_type: &str,
    fields: Lazy<IndexMap<String, GraphQLFieldConfig>>,
) -> Lazy<IndexMap<String, GraphQLField>> {
    let parent_type = parent_type.to_string();
    Lazy::thunk(Arc::new(move || {
        let field_map = fields.get()?;
        let mut out = IndexMap::default();
        for (field_name, field_config) in field_map {
            out.insert(
                field_name.clone(),
                GraphQLField::new(&parent_type, field_name, field_config.clone())?,
            );
        }
        Ok(out)
    }))
}

/// graphql-js `GraphQLField`.
#[derive(Clone, Debug)]
pub struct GraphQLField {
    /// The name of the parent type.
    pub parent_type: String,
    pub name: String,
    pub description: Option<String>,
    pub ty: GraphQLType,
    pub args: Vec<GraphQLArgument>,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<FieldDefinition>,
}

impl GraphQLField {
    pub fn new(parent_type: &str, name: &str, config: GraphQLFieldConfig) -> Result<Self, GraphQLError> {
        assert_name(name)?;
        let mut args = Vec::with_capacity(config.args.len());
        for (arg_name, arg_config) in config.args {
            args.push(GraphQLArgument::new(
                ArgumentParent::Field {
                    type_name: parent_type.to_string(),
                    field_name: name.to_string(),
                },
                &arg_name,
                arg_config,
            )?);
        }
        Ok(GraphQLField {
            parent_type: parent_type.to_string(),
            name: name.to_string(),
            description: config.description,
            ty: config.ty,
            args,
            deprecation_reason: config.deprecation_reason,
            ast_node: config.ast_node,
        })
    }
}

impl fmt::Display for GraphQLField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.parent_type, self.name)
    }
}

/// The `parent` of a graphql-js `GraphQLArgument`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgumentParent {
    Field {
        type_name: String,
        field_name: String,
    },
    /// A directive, by name.
    Directive(String),
}

impl fmt::Display for ArgumentParent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgumentParent::Field { type_name, field_name } => write!(f, "{type_name}.{field_name}"),
            ArgumentParent::Directive(name) => write!(f, "@{name}"),
        }
    }
}

/// graphql-js `GraphQLArgument`.
#[derive(Clone, Debug)]
pub struct GraphQLArgument {
    pub parent: ArgumentParent,
    pub name: String,
    pub description: Option<String>,
    pub ty: GraphQLType,
    /// graphql-js `default`. (The legacy `defaultValue` is never set by SDL.)
    pub default: Option<DefaultInput>,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<InputValueDefinition>,
}

impl GraphQLArgument {
    pub fn new(parent: ArgumentParent, name: &str, config: GraphQLArgumentConfig) -> Result<Self, GraphQLError> {
        assert_name(name)?;
        Ok(GraphQLArgument {
            parent,
            name: name.to_string(),
            description: config.description,
            ty: config.ty,
            default: config.default,
            deprecation_reason: config.deprecation_reason,
            ast_node: config.ast_node,
        })
    }
}

impl fmt::Display for GraphQLArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({}:)", self.parent, self.name)
    }
}

/// graphql-js `isRequiredArgument`.
pub fn is_required_argument(arg: &GraphQLArgument) -> bool {
    arg.ty.is_non_null() && arg.default.is_none()
}

/// graphql-js `GraphQLInterfaceTypeConfig`.
pub struct GraphQLInterfaceTypeConfig {
    pub name: String,
    pub description: Option<String>,
    pub interfaces: Lazy<Vec<String>>,
    pub fields: Lazy<IndexMap<String, GraphQLFieldConfig>>,
    pub ast_node: Option<InterfaceTypeDefinition>,
    pub extension_ast_nodes: Vec<InterfaceTypeExtension>,
}

/// graphql-js `GraphQLInterfaceType`.
#[derive(Clone, Debug)]
pub struct GraphQLInterfaceType {
    pub name: String,
    pub description: Option<String>,
    pub ast_node: Option<InterfaceTypeDefinition>,
    pub extension_ast_nodes: Vec<InterfaceTypeExtension>,
    fields: Lazy<IndexMap<String, GraphQLField>>,
    interfaces: Lazy<Vec<String>>,
}

impl GraphQLInterfaceType {
    pub fn new(config: GraphQLInterfaceTypeConfig) -> Result<Self, GraphQLError> {
        assert_name(&config.name)?;
        Ok(GraphQLInterfaceType {
            fields: define_field_map(&config.name, config.fields),
            interfaces: config.interfaces,
            name: config.name,
            description: config.description,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
        })
    }

    /// graphql-js `getFields()`.
    pub fn get_fields(&self) -> Result<&IndexMap<String, GraphQLField>, GraphQLError> {
        self.fields.get()
    }

    /// graphql-js `getInterfaces()` (names).
    pub fn get_interfaces(&self) -> Result<&Vec<String>, GraphQLError> {
        self.interfaces.get()
    }

    /// `getFields()` of a type in a constructed schema.
    pub fn fields(&self) -> &IndexMap<String, GraphQLField> {
        self.fields.resolved()
    }

    /// `getInterfaces()` of a type in a constructed schema.
    pub fn interfaces(&self) -> &[String] {
        self.interfaces.resolved()
    }
}

/// graphql-js `GraphQLUnionTypeConfig`.
pub struct GraphQLUnionTypeConfig {
    pub name: String,
    pub description: Option<String>,
    pub types: Lazy<Vec<String>>,
    pub ast_node: Option<UnionTypeDefinition>,
    pub extension_ast_nodes: Vec<UnionTypeExtension>,
}

/// graphql-js `GraphQLUnionType`.
#[derive(Clone, Debug)]
pub struct GraphQLUnionType {
    pub name: String,
    pub description: Option<String>,
    pub ast_node: Option<UnionTypeDefinition>,
    pub extension_ast_nodes: Vec<UnionTypeExtension>,
    types: Lazy<Vec<String>>,
}

impl GraphQLUnionType {
    pub fn new(config: GraphQLUnionTypeConfig) -> Result<Self, GraphQLError> {
        assert_name(&config.name)?;
        Ok(GraphQLUnionType {
            name: config.name,
            description: config.description,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
            types: config.types,
        })
    }

    /// graphql-js `getTypes()` (names).
    pub fn get_types(&self) -> Result<&Vec<String>, GraphQLError> {
        self.types.get()
    }

    /// `getTypes()` of a type in a constructed schema.
    pub fn types(&self) -> &[String] {
        self.types.resolved()
    }
}

/// graphql-js `GraphQLEnumValueConfig`.
#[derive(Clone, Debug, Default)]
pub struct GraphQLEnumValueConfig {
    pub description: Option<String>,
    /// `None` is `undefined`: the value is the name.
    pub value: Option<JsValue>,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<EnumValueDefinition>,
}

/// graphql-js `GraphQLEnumTypeConfig`.
pub struct GraphQLEnumTypeConfig {
    pub name: String,
    pub description: Option<String>,
    pub values: Lazy<IndexMap<String, GraphQLEnumValueConfig>>,
    pub ast_node: Option<EnumTypeDefinition>,
    pub extension_ast_nodes: Vec<EnumTypeExtension>,
}

/// graphql-js `GraphQLEnumType`.
#[derive(Clone, Debug)]
pub struct GraphQLEnumType {
    pub name: String,
    pub description: Option<String>,
    pub ast_node: Option<EnumTypeDefinition>,
    pub extension_ast_nodes: Vec<EnumTypeExtension>,
    values: Lazy<Vec<GraphQLEnumValue>>,
}

impl GraphQLEnumType {
    pub fn new(config: GraphQLEnumTypeConfig) -> Result<Self, GraphQLError> {
        assert_name(&config.name)?;
        Ok(GraphQLEnumType {
            values: define_enum_values(&config.name, config.values),
            name: config.name,
            description: config.description,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
        })
    }

    /// graphql-js `getValues()`. Unlike the other thunks, graphql-js does not
    /// resolve this one when constructing a schema, so it can still throw.
    pub fn get_values(&self) -> Result<&[GraphQLEnumValue], GraphQLError> {
        self.values.get().map(Vec::as_slice)
    }

    /// graphql-js `getValue(name)`.
    pub fn get_value(&self, name: &str) -> Result<Option<&GraphQLEnumValue>, GraphQLError> {
        // NOTE(port): `_nameLookup ??= keyMap(values, v => v.name)`: the last
        // value of a name wins.
        Ok(self.get_values()?.iter().rev().find(|value| value.name == name))
    }

    /// graphql-js `coerceInputValue(inputValue, hideSuggestions)`.
    pub fn coerce_input_value(&self, input_value: &JsValue, hide_suggestions: bool) -> Result<JsValue, GraphQLError> {
        let JsValue::String(input_value) = input_value else {
            let value_str = inspect(input_value);
            let suggestion = if hide_suggestions {
                String::new()
            } else {
                did_you_mean_enum_value(self, &value_str)?
            };
            return Err(GraphQLError::new(format!(
                "Enum \"{}\" cannot represent non-string value: {value_str}.{suggestion}",
                self.name
            )));
        };
        let Some(enum_value) = self.get_value(input_value)? else {
            let suggestion = if hide_suggestions {
                String::new()
            } else {
                did_you_mean_enum_value(self, input_value)?
            };
            return Err(GraphQLError::new(format!(
                "Value \"{input_value}\" does not exist in \"{}\" enum.{suggestion}",
                self.name
            )));
        };
        Ok(enum_value.value.clone())
    }

    /// graphql-js `coerceInputLiteral(valueNode, hideSuggestions)`.
    pub fn coerce_input_literal(&self, value_node: &Value, hide_suggestions: bool) -> Result<JsValue, GraphQLError> {
        let Value::Enum(enum_node) = value_node else {
            let value_str = print(value_node);
            let suggestion = if hide_suggestions {
                String::new()
            } else {
                did_you_mean_enum_value(self, &value_str)?
            };
            return Err(GraphQLError::new(format!(
                "Enum \"{}\" cannot represent non-enum value: {value_str}.{suggestion}",
                self.name
            )));
        };
        let Some(enum_value) = self.get_value(&enum_node.value)? else {
            let value_str = print(value_node);
            let suggestion = if hide_suggestions {
                String::new()
            } else {
                did_you_mean_enum_value(self, &value_str)?
            };
            return Err(GraphQLError::new(format!(
                "Value \"{value_str}\" does not exist in \"{}\" enum.{suggestion}",
                self.name
            )));
        };
        Ok(enum_value.value.clone())
    }
}

/// graphql-js `defineEnumValues`.
fn define_enum_values(
    parent_enum: &str,
    values: Lazy<IndexMap<String, GraphQLEnumValueConfig>>,
) -> Lazy<Vec<GraphQLEnumValue>> {
    let parent_enum = parent_enum.to_string();
    Lazy::thunk(Arc::new(move || {
        let value_map = values.get()?;
        let mut out = Vec::with_capacity(value_map.len());
        for (value_name, value_config) in value_map {
            out.push(GraphQLEnumValue::new(&parent_enum, value_name, value_config.clone())?);
        }
        Ok(out)
    }))
}

/// graphql-js `didYouMeanEnumValue`.
fn did_you_mean_enum_value(enum_type: &GraphQLEnumType, unknown_value_str: &str) -> Result<String, GraphQLError> {
    let all_names: Vec<&str> = enum_type
        .get_values()?
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let suggested_values = suggestion_list(unknown_value_str, &all_names);
    Ok(did_you_mean(Some("the enum value"), &suggested_values))
}

/// graphql-js `GraphQLEnumValue`.
#[derive(Clone, Debug)]
pub struct GraphQLEnumValue {
    /// The name of the enum.
    pub parent_enum: String,
    pub name: String,
    pub description: Option<String>,
    pub value: JsValue,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<EnumValueDefinition>,
}

impl GraphQLEnumValue {
    pub fn new(parent_enum: &str, name: &str, config: GraphQLEnumValueConfig) -> Result<Self, GraphQLError> {
        assert_enum_value_name(name)?;
        Ok(GraphQLEnumValue {
            parent_enum: parent_enum.to_string(),
            name: name.to_string(),
            description: config.description,
            value: config.value.unwrap_or_else(|| JsValue::String(name.to_string())),
            deprecation_reason: config.deprecation_reason,
            ast_node: config.ast_node,
        })
    }
}

impl fmt::Display for GraphQLEnumValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.parent_enum, self.name)
    }
}

/// graphql-js `GraphQLInputFieldConfig`.
#[derive(Clone, Debug)]
pub struct GraphQLInputFieldConfig {
    pub description: Option<String>,
    pub ty: GraphQLType,
    pub default: Option<DefaultInput>,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<InputValueDefinition>,
}

/// graphql-js `GraphQLInputObjectTypeConfig`.
pub struct GraphQLInputObjectTypeConfig {
    pub name: String,
    pub description: Option<String>,
    pub fields: Lazy<IndexMap<String, GraphQLInputFieldConfig>>,
    pub ast_node: Option<InputObjectTypeDefinition>,
    pub extension_ast_nodes: Vec<InputObjectTypeExtension>,
    pub is_one_of: bool,
}

/// graphql-js `GraphQLInputObjectType`.
#[derive(Clone, Debug)]
pub struct GraphQLInputObjectType {
    pub name: String,
    pub description: Option<String>,
    pub ast_node: Option<InputObjectTypeDefinition>,
    pub extension_ast_nodes: Vec<InputObjectTypeExtension>,
    pub is_one_of: bool,
    fields: Lazy<IndexMap<String, GraphQLInputField>>,
}

impl GraphQLInputObjectType {
    pub fn new(config: GraphQLInputObjectTypeConfig) -> Result<Self, GraphQLError> {
        assert_name(&config.name)?;
        Ok(GraphQLInputObjectType {
            fields: define_input_field_map(&config.name, config.fields),
            name: config.name,
            description: config.description,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
            is_one_of: config.is_one_of,
        })
    }

    /// graphql-js `getFields()`.
    pub fn get_fields(&self) -> Result<&IndexMap<String, GraphQLInputField>, GraphQLError> {
        self.fields.get()
    }

    /// `getFields()` of a type in a constructed schema.
    pub fn fields(&self) -> &IndexMap<String, GraphQLInputField> {
        self.fields.resolved()
    }
}

/// graphql-js `defineInputFieldMap`.
fn define_input_field_map(
    parent_type: &str,
    fields: Lazy<IndexMap<String, GraphQLInputFieldConfig>>,
) -> Lazy<IndexMap<String, GraphQLInputField>> {
    let parent_type = parent_type.to_string();
    Lazy::thunk(Arc::new(move || {
        let field_map = fields.get()?;
        let mut out = IndexMap::default();
        for (field_name, field_config) in field_map {
            out.insert(
                field_name.clone(),
                GraphQLInputField::new(&parent_type, field_name, field_config.clone())?,
            );
        }
        Ok(out)
    }))
}

/// graphql-js `GraphQLInputField`.
#[derive(Clone, Debug)]
pub struct GraphQLInputField {
    /// The name of the parent input object type.
    pub parent_type: String,
    pub name: String,
    pub description: Option<String>,
    pub ty: GraphQLType,
    /// graphql-js `default`. (The legacy `defaultValue` is never set by SDL.)
    pub default: Option<DefaultInput>,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<InputValueDefinition>,
}

impl GraphQLInputField {
    pub fn new(parent_type: &str, name: &str, config: GraphQLInputFieldConfig) -> Result<Self, GraphQLError> {
        // NOTE(port): the `'resolve' in config` devAssert cannot fail: the
        // config type has no `resolve`.
        assert_name(name)?;
        Ok(GraphQLInputField {
            parent_type: parent_type.to_string(),
            name: name.to_string(),
            description: config.description,
            ty: config.ty,
            default: config.default,
            deprecation_reason: config.deprecation_reason,
            ast_node: config.ast_node,
        })
    }
}

impl fmt::Display for GraphQLInputField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.parent_type, self.name)
    }
}

/// graphql-js `isRequiredInputField`.
pub fn is_required_input_field(field: &GraphQLInputField) -> bool {
    field.ty.is_non_null() && field.default.is_none()
}

/// What graphql-js reads from a `GraphQLArgument | GraphQLInputField` (an
/// "input value"): its type and default, and how it prints.
pub trait GraphQLInputValue: fmt::Display {
    fn input_type(&self) -> &GraphQLType;
    fn default_input(&self) -> Option<&DefaultInput>;
}

impl GraphQLInputValue for GraphQLArgument {
    fn input_type(&self) -> &GraphQLType {
        &self.ty
    }
    fn default_input(&self) -> Option<&DefaultInput> {
        self.default.as_ref()
    }
}

impl GraphQLInputValue for GraphQLInputField {
    fn input_type(&self) -> &GraphQLType {
        &self.ty
    }
    fn default_input(&self) -> Option<&DefaultInput> {
        self.default.as_ref()
    }
}
