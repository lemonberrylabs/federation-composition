//! Port of graphql-js `utilities/extendSchema`: `extendSchemaImpl`, on the
//! path `buildASTSchema` takes.
//!
//! NOTE(port): `buildASTSchema` extends an empty schema config, so
//! `mapSchemaConfig` has no types or directives to map and only runs the
//! schema mapper, which is inlined here; `extendDirective` and the type
//! mappers (which extend existing types) never run and are not ported, nor
//! is `extendSchema` itself.

use std::sync::Arc;

use crate::ast::{
    Definition, Directive, DirectiveDefinition, DirectiveExtension, Document, EnumTypeDefinition, EnumTypeExtension,
    EnumValueDefinition, FieldDefinition, InputObjectTypeDefinition, InputObjectTypeExtension, InputValueDefinition,
    InterfaceTypeDefinition, InterfaceTypeExtension, NamedType, ObjectTypeDefinition, ObjectTypeExtension,
    OperationType, ScalarTypeDefinition, ScalarTypeExtension, SchemaDefinition, SchemaExtension, Type,
    UnionTypeDefinition, UnionTypeExtension,
};
use crate::collections::{IndexMap, IndexSet};
use crate::directive_location::DirectiveLocation;
use crate::error::GraphQLError;
use crate::execution::values::get_directive_values;
use crate::jsutils::JsValue;
use crate::r#type::definition::{
    DefaultInput, GraphQLArgumentConfig, GraphQLEnumType, GraphQLEnumTypeConfig, GraphQLEnumValueConfig,
    GraphQLFieldConfig, GraphQLInputFieldConfig, GraphQLInputObjectType, GraphQLInputObjectTypeConfig,
    GraphQLInterfaceType, GraphQLInterfaceTypeConfig, GraphQLNamedType, GraphQLObjectType, GraphQLObjectTypeConfig,
    GraphQLScalarType, GraphQLScalarTypeConfig, GraphQLType, GraphQLUnionType, GraphQLUnionTypeConfig, Lazy,
};
use crate::r#type::directives::{
    graphql_deprecated_directive, graphql_one_of_directive, graphql_specified_by_directive, GraphQLDirective,
    GraphQLDirectiveConfig,
};
use crate::r#type::schema::{std_type_map, GraphQLSchemaConfig};

/// graphql-js `AccumulatorMap`.
type AccumulatorMap<T> = IndexMap<String, Vec<T>>;

fn accumulate<T: Clone>(map: &mut AccumulatorMap<T>, key: &str, item: &T) {
    map.entry(key.to_string()).or_default().push(item.clone());
}

/// What the closures of `extendSchemaImpl` share once the named types are
/// known: `getNamedType` succeeds for the standard types and for the types
/// the document defines.
struct ExtendContext {
    type_names: IndexSet<String>,
    /// graphql-js `stdTypeMap`; also what the specified directives'
    /// argument types resolve to while their values are coerced.
    std_types: IndexMap<String, GraphQLNamedType>,
    deprecated_directive: GraphQLDirective,
    specified_by_directive: GraphQLDirective,
    one_of_directive: GraphQLDirective,
}

impl ExtendContext {
    /// graphql-js `namedTypeFromAST`.
    fn named_type_from_ast(&self, node: &NamedType) -> Result<String, GraphQLError> {
        let name = &node.name.value;
        if !self.type_names.contains(name) {
            // TS: invariant(type !== undefined, `Unknown type: "${name}".`)
            return Err(GraphQLError::new(format!("Unknown type: \"{name}\".")));
        }
        Ok(name.clone())
    }

    /// graphql-js `typeFromAST` (the closure in `extendSchemaImpl`).
    fn type_from_ast(&self, node: &Type) -> Result<GraphQLType, GraphQLError> {
        match node {
            Type::List(list) => Ok(GraphQLType::list(self.type_from_ast(&list.ty)?)),
            Type::NonNull(non_null) => Ok(GraphQLType::non_null(self.type_from_ast(&non_null.ty)?)),
            Type::Named(named) => Ok(GraphQLType::Named(self.named_type_from_ast(named)?)),
        }
    }

    /// graphql-js `buildFieldMap(nodes)`, given each node's `fields`.
    fn build_field_map<'n>(
        &self,
        nodes: impl Iterator<Item = &'n [FieldDefinition]>,
    ) -> Result<IndexMap<String, GraphQLFieldConfig>, GraphQLError> {
        let mut field_config_map = IndexMap::default();
        for node_fields in nodes {
            for field in node_fields {
                let ty = self.type_from_ast(&field.ty)?;
                let description = field.description.as_ref().map(|d| d.value.clone());
                let args = self.build_argument_map(&field.arguments)?;
                let deprecation_reason = self.get_deprecation_reason(&field.directives)?;
                field_config_map.insert(
                    field.name.value.clone(),
                    GraphQLFieldConfig {
                        description,
                        ty,
                        args,
                        deprecation_reason,
                        ast_node: Some(field.clone()),
                    },
                );
            }
        }
        Ok(field_config_map)
    }

    /// graphql-js `buildArgumentMap(args)`.
    fn build_argument_map(
        &self,
        args: &[InputValueDefinition],
    ) -> Result<IndexMap<String, GraphQLArgumentConfig>, GraphQLError> {
        let mut arg_config_map = IndexMap::default();
        for arg in args {
            let ty = self.type_from_ast(&arg.ty)?;
            arg_config_map.insert(
                arg.name.value.clone(),
                GraphQLArgumentConfig {
                    ty,
                    description: arg.description.as_ref().map(|d| d.value.clone()),
                    default: arg.default_value.clone().map(DefaultInput::Literal),
                    deprecation_reason: self.get_deprecation_reason(&arg.directives)?,
                    ast_node: Some(arg.clone()),
                },
            );
        }
        Ok(arg_config_map)
    }

    /// graphql-js `buildInputFieldMap(nodes)`, given each node's `fields`.
    fn build_input_field_map<'n>(
        &self,
        nodes: impl Iterator<Item = &'n [InputValueDefinition]>,
    ) -> Result<IndexMap<String, GraphQLInputFieldConfig>, GraphQLError> {
        let mut input_field_map = IndexMap::default();
        for fields_nodes in nodes {
            for field in fields_nodes {
                let ty = self.type_from_ast(&field.ty)?;
                input_field_map.insert(
                    field.name.value.clone(),
                    GraphQLInputFieldConfig {
                        ty,
                        description: field.description.as_ref().map(|d| d.value.clone()),
                        default: field.default_value.clone().map(DefaultInput::Literal),
                        deprecation_reason: self.get_deprecation_reason(&field.directives)?,
                        ast_node: Some(field.clone()),
                    },
                );
            }
        }
        Ok(input_field_map)
    }

    /// graphql-js `buildEnumValueMap(nodes)`, given each node's `values`.
    fn build_enum_value_map<'n>(
        &self,
        nodes: impl Iterator<Item = &'n [EnumValueDefinition]>,
    ) -> Result<IndexMap<String, GraphQLEnumValueConfig>, GraphQLError> {
        let mut enum_value_map = IndexMap::default();
        for values_nodes in nodes {
            for value in values_nodes {
                enum_value_map.insert(
                    value.name.value.clone(),
                    GraphQLEnumValueConfig {
                        description: value.description.as_ref().map(|d| d.value.clone()),
                        value: None,
                        deprecation_reason: self.get_deprecation_reason(&value.directives)?,
                        ast_node: Some(value.clone()),
                    },
                );
            }
        }
        Ok(enum_value_map)
    }

    /// graphql-js `buildInterfaces(nodes)`, given each node's `interfaces`.
    fn build_interfaces<'n>(&self, nodes: impl Iterator<Item = &'n [NamedType]>) -> Result<Vec<String>, GraphQLError> {
        let mut interfaces = Vec::new();
        for node_interfaces in nodes {
            for interface in node_interfaces {
                interfaces.push(self.named_type_from_ast(interface)?);
            }
        }
        Ok(interfaces)
    }

    /// graphql-js `buildUnionTypes(nodes)`, given each node's `types`.
    fn build_union_types<'n>(&self, nodes: impl Iterator<Item = &'n [NamedType]>) -> Result<Vec<String>, GraphQLError> {
        // Same as `buildInterfaces`: `flatMap(node => node.types?.map(namedTypeFromAST) ?? [])`.
        self.build_interfaces(nodes)
    }

    /// graphql-js `getDeprecationReason(node)`, given the node's `directives`.
    fn get_deprecation_reason(&self, directives: &[Directive]) -> Result<Option<String>, GraphQLError> {
        let deprecated = get_directive_values(&self.deprecated_directive, directives, &self.std_types)?;
        Ok(deprecated.and_then(|values| string_value(values.get("reason"))))
    }

    /// graphql-js `getSpecifiedByURL(node)`, given the node's `directives`.
    fn get_specified_by_url(&self, directives: &[Directive]) -> Result<Option<String>, GraphQLError> {
        let specified_by = get_directive_values(&self.specified_by_directive, directives, &self.std_types)?;
        Ok(specified_by.and_then(|values| string_value(values.get("url"))))
    }

    /// graphql-js `isOneOf(node)`, given the node's `directives`.
    fn is_one_of(&self, directives: &[Directive]) -> Result<bool, GraphQLError> {
        Ok(get_directive_values(&self.one_of_directive, directives, &self.std_types)?.is_some())
    }
}

/// `values.reason` / `values.url`: coerced as `String!`, so a string when present.
fn string_value(value: Option<&JsValue>) -> Option<String> {
    match value {
        Some(JsValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// graphql-js `extendSchemaImpl(emptySchemaConfig, documentAST)`.
pub(crate) fn extend_schema_impl(document_ast: &Document) -> Result<GraphQLSchemaConfig, GraphQLError> {
    let mut type_defs: Vec<&Definition> = Vec::new();
    let mut scalar_extensions: AccumulatorMap<ScalarTypeExtension> = IndexMap::default();
    let mut object_extensions: AccumulatorMap<ObjectTypeExtension> = IndexMap::default();
    let mut interface_extensions: AccumulatorMap<InterfaceTypeExtension> = IndexMap::default();
    let mut union_extensions: AccumulatorMap<UnionTypeExtension> = IndexMap::default();
    let mut enum_extensions: AccumulatorMap<EnumTypeExtension> = IndexMap::default();
    let mut input_object_extensions: AccumulatorMap<InputObjectTypeExtension> = IndexMap::default();
    let mut directive_extensions: AccumulatorMap<DirectiveExtension> = IndexMap::default();
    let mut directive_defs: Vec<&DirectiveDefinition> = Vec::new();
    let mut schema_def: Option<&SchemaDefinition> = None;
    let mut schema_extensions: Vec<&SchemaExtension> = Vec::new();
    let mut is_schema_changed = false;

    for def in &document_ast.definitions {
        match def {
            Definition::SchemaDefinition(def) => schema_def = Some(def),
            Definition::SchemaExtension(def) => schema_extensions.push(def),
            Definition::DirectiveDefinition(def) => directive_defs.push(def),
            Definition::DirectiveExtension(def) => accumulate(&mut directive_extensions, &def.name.value, def),
            Definition::ScalarTypeDefinition(_)
            | Definition::ObjectTypeDefinition(_)
            | Definition::InterfaceTypeDefinition(_)
            | Definition::UnionTypeDefinition(_)
            | Definition::EnumTypeDefinition(_)
            | Definition::InputObjectTypeDefinition(_) => type_defs.push(def),
            Definition::ScalarTypeExtension(def) => accumulate(&mut scalar_extensions, &def.name.value, def),
            Definition::ObjectTypeExtension(def) => accumulate(&mut object_extensions, &def.name.value, def),
            Definition::InterfaceTypeExtension(def) => accumulate(&mut interface_extensions, &def.name.value, def),
            Definition::UnionTypeExtension(def) => accumulate(&mut union_extensions, &def.name.value, def),
            Definition::EnumTypeExtension(def) => accumulate(&mut enum_extensions, &def.name.value, def),
            Definition::InputObjectTypeExtension(def) => accumulate(&mut input_object_extensions, &def.name.value, def),
            Definition::OperationDefinition(_) | Definition::FragmentDefinition(_) => continue,
        }
        is_schema_changed = true;
    }

    if !is_schema_changed {
        // The empty schema config, unchanged.
        return Ok(GraphQLSchemaConfig {
            directives: Some(Vec::new()),
            ..Default::default()
        });
    }

    let std_types = std_type_map();
    // NOTE(port): the names `getNamedType` resolves once the type
    // definitions are mapped. Only the thunks and the code after the loop
    // below resolve names, so they are computed up front.
    let mut type_names: IndexSet<String> = std_types.keys().cloned().collect();
    for type_node in &type_defs {
        type_names.insert(type_def_name(type_node).to_string());
    }
    let context = Arc::new(ExtendContext {
        type_names,
        std_types,
        deprecated_directive: graphql_deprecated_directive(),
        specified_by_directive: graphql_specified_by_directive(),
        one_of_directive: graphql_one_of_directive(),
    });
    let extensions = Extensions {
        scalar: scalar_extensions,
        object: object_extensions,
        interface: interface_extensions,
        union: union_extensions,
        enumeration: enum_extensions,
        input_object: input_object_extensions,
    };

    // The `SchemaElementKind.SCHEMA` mapper.
    let mut mapped_type_map: IndexMap<String, GraphQLNamedType> = IndexMap::default();
    for type_node in &type_defs {
        let name = type_def_name(type_node);
        let named_type = match context.std_types.get(name) {
            Some(std_type) => std_type.clone(),
            None => build_named_type(&context, &extensions, type_node)?,
        };
        mapped_type_map.insert(named_type.name().to_string(), named_type);
    }

    let mut operation_types: [Option<String>; 3] = [None, None, None];
    if let Some(schema_def) = schema_def {
        get_operation_types(
            &context,
            &mut operation_types,
            std::iter::once(schema_def.operation_types.as_slice()),
        )?;
    }
    get_operation_types(
        &context,
        &mut operation_types,
        schema_extensions.iter().map(|node| node.operation_types.as_slice()),
    )?;
    let [query, mutation, subscription] = operation_types;

    let mut directives = Vec::with_capacity(directive_defs.len());
    for node in directive_defs {
        directives.push(build_directive(&context, &directive_extensions, node)?);
    }

    Ok(GraphQLSchemaConfig {
        description: schema_def
            .and_then(|def| def.description.as_ref())
            .map(|d| d.value.clone()),
        query,
        mutation,
        subscription,
        types: mapped_type_map.into_values().collect(),
        directives: Some(directives),
        ast_node: schema_def.cloned(),
        extension_ast_nodes: schema_extensions.into_iter().cloned().collect(),
        assume_valid: false,
    })
}

struct Extensions {
    scalar: AccumulatorMap<ScalarTypeExtension>,
    object: AccumulatorMap<ObjectTypeExtension>,
    interface: AccumulatorMap<InterfaceTypeExtension>,
    union: AccumulatorMap<UnionTypeExtension>,
    enumeration: AccumulatorMap<EnumTypeExtension>,
    input_object: AccumulatorMap<InputObjectTypeExtension>,
}

fn type_def_name(def: &Definition) -> &str {
    &def.name().expect("type definitions have a name").value
}

/// graphql-js `getOperationTypes(nodes)`, given each node's
/// `operationTypes`, assigned onto `op_types` (`[query, mutation,
/// subscription]`) the way the object spreads assign them.
fn get_operation_types<'n>(
    context: &ExtendContext,
    op_types: &mut [Option<String>; 3],
    nodes: impl Iterator<Item = &'n [crate::ast::OperationTypeDefinition]>,
) -> Result<(), GraphQLError> {
    for operation_types_nodes in nodes {
        for operation_type in operation_types_nodes {
            let index = match operation_type.operation {
                OperationType::Query => 0,
                OperationType::Mutation => 1,
                OperationType::Subscription => 2,
            };
            op_types[index] = Some(context.named_type_from_ast(&operation_type.ty)?);
        }
    }
    Ok(())
}

/// graphql-js `buildDirective(node)`.
fn build_directive(
    context: &ExtendContext,
    directive_extensions: &AccumulatorMap<DirectiveExtension>,
    node: &DirectiveDefinition,
) -> Result<GraphQLDirective, GraphQLError> {
    let extension_ast_nodes = directive_extensions.get(&node.name.value).cloned().unwrap_or_default();
    let deprecation_reason = match context.get_deprecation_reason(&node.directives)? {
        Some(reason) => Some(reason),
        None => {
            // `.map(getDeprecationReason).find(reason => reason != null)`: every
            // extension is read before the first reason is taken.
            let mut reasons = Vec::with_capacity(extension_ast_nodes.len());
            for extension_node in &extension_ast_nodes {
                reasons.push(context.get_deprecation_reason(&extension_node.directives)?);
            }
            reasons.into_iter().flatten().next()
        }
    };
    let args = context.build_argument_map(&node.arguments)?;
    GraphQLDirective::new(GraphQLDirectiveConfig {
        name: node.name.value.clone(),
        description: node.description.as_ref().map(|d| d.value.clone()),
        locations: node
            .locations
            .iter()
            .map(|name| DirectiveLocation::from_str(&name.value).expect("the parser accepts only directive locations"))
            .collect(),
        is_repeatable: node.repeatable,
        args,
        deprecation_reason,
        ast_node: Some(node.clone()),
        extension_ast_nodes,
    })
}

fn extensions_of<T: Clone>(map: &AccumulatorMap<T>, name: &str) -> Vec<T> {
    map.get(name).cloned().unwrap_or_default()
}

/// graphql-js `buildNamedType(astNode)`.
fn build_named_type(
    context: &Arc<ExtendContext>,
    extensions: &Extensions,
    ast_node: &Definition,
) -> Result<GraphQLNamedType, GraphQLError> {
    Ok(match ast_node {
        Definition::ObjectTypeDefinition(ast_node) => {
            let name = ast_node.name.value.clone();
            let extension_ast_nodes = extensions_of(&extensions.object, &name);
            let all_nodes = Arc::new((ast_node.clone(), extension_ast_nodes.clone()));
            GraphQLNamedType::Object(GraphQLObjectType::new(GraphQLObjectTypeConfig {
                name,
                description: ast_node.description.as_ref().map(|d| d.value.clone()),
                interfaces: {
                    let (context, all_nodes) = (context.clone(), all_nodes.clone());
                    Lazy::thunk(Arc::new(move || {
                        context.build_interfaces(object_nodes(&all_nodes).map(|n| n.0))
                    }))
                },
                fields: {
                    let (context, all_nodes) = (context.clone(), all_nodes);
                    Lazy::thunk(Arc::new(move || {
                        context.build_field_map(object_nodes(&all_nodes).map(|n| n.1))
                    }))
                },
                ast_node: Some(ast_node.clone()),
                extension_ast_nodes,
            })?)
        }
        Definition::InterfaceTypeDefinition(ast_node) => {
            let name = ast_node.name.value.clone();
            let extension_ast_nodes = extensions_of(&extensions.interface, &name);
            let all_nodes = Arc::new((ast_node.clone(), extension_ast_nodes.clone()));
            GraphQLNamedType::Interface(GraphQLInterfaceType::new(GraphQLInterfaceTypeConfig {
                name,
                description: ast_node.description.as_ref().map(|d| d.value.clone()),
                interfaces: {
                    let (context, all_nodes) = (context.clone(), all_nodes.clone());
                    Lazy::thunk(Arc::new(move || {
                        context.build_interfaces(interface_nodes(&all_nodes).map(|n| n.0))
                    }))
                },
                fields: {
                    let (context, all_nodes) = (context.clone(), all_nodes);
                    Lazy::thunk(Arc::new(move || {
                        context.build_field_map(interface_nodes(&all_nodes).map(|n| n.1))
                    }))
                },
                ast_node: Some(ast_node.clone()),
                extension_ast_nodes,
            })?)
        }
        Definition::EnumTypeDefinition(ast_node) => {
            let name = ast_node.name.value.clone();
            let extension_ast_nodes = extensions_of(&extensions.enumeration, &name);
            let all_nodes: Arc<(EnumTypeDefinition, Vec<EnumTypeExtension>)> =
                Arc::new((ast_node.clone(), extension_ast_nodes.clone()));
            GraphQLNamedType::Enum(GraphQLEnumType::new(GraphQLEnumTypeConfig {
                name,
                description: ast_node.description.as_ref().map(|d| d.value.clone()),
                values: {
                    let context = context.clone();
                    Lazy::thunk(Arc::new(move || {
                        let (def, exts) = &*all_nodes;
                        context.build_enum_value_map(
                            std::iter::once(def.values.as_slice()).chain(exts.iter().map(|e| e.values.as_slice())),
                        )
                    }))
                },
                ast_node: Some(ast_node.clone()),
                extension_ast_nodes,
            })?)
        }
        Definition::UnionTypeDefinition(ast_node) => {
            let name = ast_node.name.value.clone();
            let extension_ast_nodes = extensions_of(&extensions.union, &name);
            let all_nodes: Arc<(UnionTypeDefinition, Vec<UnionTypeExtension>)> =
                Arc::new((ast_node.clone(), extension_ast_nodes.clone()));
            GraphQLNamedType::Union(GraphQLUnionType::new(GraphQLUnionTypeConfig {
                name,
                description: ast_node.description.as_ref().map(|d| d.value.clone()),
                types: {
                    let context = context.clone();
                    Lazy::thunk(Arc::new(move || {
                        let (def, exts) = &*all_nodes;
                        context.build_union_types(
                            std::iter::once(def.types.as_slice()).chain(exts.iter().map(|e| e.types.as_slice())),
                        )
                    }))
                },
                ast_node: Some(ast_node.clone()),
                extension_ast_nodes,
            })?)
        }
        Definition::ScalarTypeDefinition(ast_node) => {
            let name = ast_node.name.value.clone();
            let extension_ast_nodes: Vec<ScalarTypeExtension> = extensions_of(&extensions.scalar, &name);
            let mut specified_by_url = context.get_specified_by_url(&ast_node.directives)?;
            for extension_node in &extension_ast_nodes {
                specified_by_url = context
                    .get_specified_by_url(&extension_node.directives)?
                    .or(specified_by_url);
            }
            GraphQLNamedType::Scalar(GraphQLScalarType::new(GraphQLScalarTypeConfig {
                name,
                description: ast_node.description.as_ref().map(|d| d.value.clone()),
                specified_by_url,
                ast_node: Some(ScalarTypeDefinition::clone(ast_node)),
                extension_ast_nodes,
            })?)
        }
        Definition::InputObjectTypeDefinition(ast_node) => {
            let name = ast_node.name.value.clone();
            let extension_ast_nodes = extensions_of(&extensions.input_object, &name);
            let all_nodes: Arc<(InputObjectTypeDefinition, Vec<InputObjectTypeExtension>)> =
                Arc::new((ast_node.clone(), extension_ast_nodes.clone()));
            let fields = {
                let context = context.clone();
                Lazy::thunk(Arc::new(move || {
                    let (def, exts) = &*all_nodes;
                    context.build_input_field_map(
                        std::iter::once(def.fields.as_slice()).chain(exts.iter().map(|e| e.fields.as_slice())),
                    )
                }))
            };
            let is_one_of = context.is_one_of(&ast_node.directives)?;
            GraphQLNamedType::InputObject(GraphQLInputObjectType::new(GraphQLInputObjectTypeConfig {
                name,
                description: ast_node.description.as_ref().map(|d| d.value.clone()),
                fields,
                ast_node: Some(ast_node.clone()),
                extension_ast_nodes,
                is_one_of,
            })?)
        }
        _ => unreachable!("only type definitions are built"),
    })
}

/// The `interfaces` and `fields` of an object type definition and its extensions.
fn object_nodes(
    all_nodes: &(ObjectTypeDefinition, Vec<ObjectTypeExtension>),
) -> impl Iterator<Item = (&[NamedType], &[FieldDefinition])> {
    let (def, exts) = all_nodes;
    std::iter::once((def.interfaces.as_slice(), def.fields.as_slice()))
        .chain(exts.iter().map(|e| (e.interfaces.as_slice(), e.fields.as_slice())))
}

/// The `interfaces` and `fields` of an interface type definition and its extensions.
fn interface_nodes(
    all_nodes: &(InterfaceTypeDefinition, Vec<InterfaceTypeExtension>),
) -> impl Iterator<Item = (&[NamedType], &[FieldDefinition])> {
    let (def, exts) = all_nodes;
    std::iter::once((def.interfaces.as_slice(), def.fields.as_slice()))
        .chain(exts.iter().map(|e| (e.interfaces.as_slice(), e.fields.as_slice())))
}
