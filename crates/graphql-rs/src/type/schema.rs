//! Port of graphql-js `type/schema`: `GraphQLSchema`.

use std::sync::OnceLock;

use crate::ast::{OperationType, SchemaDefinition, SchemaExtension};
use crate::collections::{IndexMap, IndexSet};
use crate::error::GraphQLError;

use super::definition::{GraphQLField, GraphQLNamedType, TypeLookup};
use super::directives::{specified_directives, GraphQLDirective};
use super::introspection::introspection_types;
use super::scalars::specified_scalar_types;

/// graphql-js `GraphQLSchemaConfig`.
#[derive(Debug, Default)]
pub struct GraphQLSchemaConfig {
    pub description: Option<String>,
    /// The query root type, by name.
    pub query: Option<String>,
    pub mutation: Option<String>,
    pub subscription: Option<String>,
    pub types: Vec<GraphQLNamedType>,
    /// `None` is `undefined`: the specified directives.
    pub directives: Option<Vec<GraphQLDirective>>,
    pub ast_node: Option<SchemaDefinition>,
    pub extension_ast_nodes: Vec<SchemaExtension>,
    pub assume_valid: bool,
}

/// The `{ objects, interfaces }` graphql-js `getImplementations` returns, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Implementations {
    pub objects: Vec<String>,
    pub interfaces: Vec<String>,
}

/// graphql-js `GraphQLSchema`, as [`build_schema`](crate::build_schema) and
/// [`build_ast_schema`](crate::build_ast_schema) build it. Check it with
/// [`validate_schema`](crate::validate_schema) or [`assert_valid_schema`](crate::assert_valid_schema).
///
/// Only the paths of `buildSchema` and `validateSchema` are ported, so the type system behind a
/// schema (its types, fields and directives) is not part of the stable API yet.
#[derive(Debug)]
pub struct GraphQLSchema {
    /// The schema's description (`description`).
    pub description: Option<String>,
    /// The schema definition it was built from (`astNode`).
    pub ast_node: Option<SchemaDefinition>,
    /// The schema extensions it was built from (`extensionASTNodes`).
    pub extension_ast_nodes: Vec<SchemaExtension>,
    /// graphql-js `assumeValid`.
    pub assume_valid: bool,
    query_type: Option<String>,
    mutation_type: Option<String>,
    subscription_type: Option<String>,
    directives: Vec<GraphQLDirective>,
    type_map: IndexMap<String, GraphQLNamedType>,
    implementations_map: IndexMap<String, Implementations>,
    /// graphql-js `__validationErrors`, set by `validateSchema`.
    pub(crate) validation_errors: OnceLock<Vec<GraphQLError>>,
}

/// The specified scalars and introspection types by name (graphql-js
/// `stdTypeMap` in `extendSchema` and `mapSchemaConfig`).
pub(crate) fn std_type_map() -> IndexMap<String, GraphQLNamedType> {
    specified_scalar_types()
        .into_iter()
        .map(GraphQLNamedType::Scalar)
        .chain(introspection_types())
        .map(|t| (t.name().to_string(), t))
        .collect()
}

impl GraphQLSchema {
    /// graphql-js `new GraphQLSchema(config)`. Resolves the `interfaces`,
    /// `fields` and `types` thunks of every type, in graphql-js order, and
    /// fails with the error the first failing thunk throws.
    ///
    /// NOTE(port): crate-private because named types are identified by name.
    /// graphql-js tells apart two type objects with the same name and throws
    /// `Schema must contain uniquely named types but contains multiple types
    /// named "X".`; the only caller, `buildASTSchema`, never passes two types
    /// with the same name (its types are a map by name, and a type named like
    /// a specified scalar or an introspection type is that type).
    pub(crate) fn new(config: GraphQLSchemaConfig) -> Result<Self, GraphQLError> {
        let directives = config.directives.unwrap_or_else(specified_directives);

        // The type objects graphql-js can reach: the config's types and the
        // standard types (which a config type with the same name is).
        let mut pool = std_type_map();
        let config_type_names: Vec<String> = config.types.iter().map(|t| t.name().to_string()).collect();
        for named_type in config.types {
            pool.insert(named_type.name().to_string(), named_type);
        }

        let mut all_referenced_types: IndexSet<String> = config_type_names.iter().cloned().collect();
        for type_name in &config_type_names {
            all_referenced_types.shift_remove(type_name);
            collect_referenced_types(type_name, &pool, &mut all_referenced_types)?;
        }
        for root in [&config.query, &config.mutation, &config.subscription]
            .into_iter()
            .flatten()
        {
            collect_referenced_types(root, &pool, &mut all_referenced_types)?;
        }
        for directive in &directives {
            for arg in &directive.args {
                collect_referenced_types(arg.ty.named_type(), &pool, &mut all_referenced_types)?;
            }
        }
        collect_referenced_types("__Schema", &pool, &mut all_referenced_types)?;

        let mut type_map: IndexMap<String, GraphQLNamedType> = IndexMap::default();
        let mut implementations_map: IndexMap<String, Implementations> = IndexMap::default();
        for type_name in &all_referenced_types {
            let named_type = pool.shift_remove(type_name).expect("collected types are in the pool");
            type_map.insert(type_name.clone(), named_type);
        }
        for (type_name, named_type) in &type_map {
            match named_type {
                GraphQLNamedType::Interface(t) => {
                    for iface in t.interfaces() {
                        if type_map.get(iface).is_some_and(|i| i.is_interface_type()) {
                            implementations_map
                                .entry(iface.clone())
                                .or_default()
                                .interfaces
                                .push(type_name.clone());
                        }
                    }
                }
                GraphQLNamedType::Object(t) => {
                    for iface in t.interfaces() {
                        if type_map.get(iface).is_some_and(|i| i.is_interface_type()) {
                            implementations_map
                                .entry(iface.clone())
                                .or_default()
                                .objects
                                .push(type_name.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        let validation_errors = OnceLock::new();
        if config.assume_valid {
            let _ = validation_errors.set(Vec::new());
        }
        Ok(GraphQLSchema {
            description: config.description,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
            assume_valid: config.assume_valid,
            query_type: config.query,
            mutation_type: config.mutation,
            subscription_type: config.subscription,
            directives,
            type_map,
            implementations_map,
            validation_errors,
        })
    }

    /// graphql-js `getQueryType()`.
    #[doc(hidden)]
    pub fn get_query_type(&self) -> Option<&GraphQLNamedType> {
        self.query_type.as_deref().and_then(|name| self.get_type(name))
    }

    /// graphql-js `getMutationType()`.
    #[doc(hidden)]
    pub fn get_mutation_type(&self) -> Option<&GraphQLNamedType> {
        self.mutation_type.as_deref().and_then(|name| self.get_type(name))
    }

    /// graphql-js `getSubscriptionType()`.
    #[doc(hidden)]
    pub fn get_subscription_type(&self) -> Option<&GraphQLNamedType> {
        self.subscription_type.as_deref().and_then(|name| self.get_type(name))
    }

    /// graphql-js `getRootType(operation)`.
    #[doc(hidden)]
    pub fn get_root_type(&self, operation: OperationType) -> Option<&GraphQLNamedType> {
        match operation {
            OperationType::Query => self.get_query_type(),
            OperationType::Mutation => self.get_mutation_type(),
            OperationType::Subscription => self.get_subscription_type(),
        }
    }

    /// graphql-js `getTypeMap()`.
    #[doc(hidden)]
    pub fn get_type_map(&self) -> &IndexMap<String, GraphQLNamedType> {
        &self.type_map
    }

    /// graphql-js `getType(name)`.
    #[doc(hidden)]
    pub fn get_type(&self, name: &str) -> Option<&GraphQLNamedType> {
        self.type_map.get(name)
    }

    /// graphql-js `getPossibleTypes(abstractType)`, by name.
    #[doc(hidden)]
    pub fn get_possible_types(&self, abstract_type: &GraphQLNamedType) -> Vec<String> {
        match abstract_type {
            GraphQLNamedType::Union(union) => union.types().to_vec(),
            _ => self.get_implementations(abstract_type.name()).objects,
        }
    }

    /// graphql-js `getImplementations(interfaceType)`, by interface name.
    #[doc(hidden)]
    pub fn get_implementations(&self, interface_type: &str) -> Implementations {
        self.implementations_map
            .get(interface_type)
            .cloned()
            .unwrap_or_default()
    }

    /// graphql-js `isSubType(abstractType, maybeSubType)`.
    #[doc(hidden)]
    pub fn is_sub_type(&self, abstract_type: &GraphQLNamedType, maybe_sub_type: &str) -> bool {
        match abstract_type {
            GraphQLNamedType::Union(union) => union.types().iter().any(|t| t == maybe_sub_type),
            _ => {
                let implementations = self.get_implementations(abstract_type.name());
                implementations
                    .objects
                    .iter()
                    .chain(&implementations.interfaces)
                    .any(|t| t == maybe_sub_type)
            }
        }
    }

    /// graphql-js `getDirectives()`.
    #[doc(hidden)]
    pub fn get_directives(&self) -> &[GraphQLDirective] {
        &self.directives
    }

    /// graphql-js `getDirective(name)`.
    #[doc(hidden)]
    pub fn get_directive(&self, name: &str) -> Option<&GraphQLDirective> {
        self.directives.iter().find(|d| d.name == name)
    }
}

impl TypeLookup for GraphQLSchema {
    fn lookup_type(&self, name: &str) -> Option<&GraphQLNamedType> {
        self.get_type(name)
    }
}

/// graphql-js `collectReferencedTypes`, by name.
///
/// NOTE(port): graphql-js recurses once per type it reaches through a field, an argument, an
/// interface or a union member, so a long chain of types would overflow the stack. This runs the
/// same depth-first search with an explicit stack of the calls that are iterating over the types
/// they reference; the types are added, and the lazy fields, interfaces and members resolved (and
/// their errors returned), in the order of the recursion.
fn collect_referenced_types<'p>(
    type_name: &'p str,
    pool: &'p IndexMap<String, GraphQLNamedType>,
    type_set: &mut IndexSet<String>,
) -> Result<(), GraphQLError> {
    let mut stack: Vec<ReferencedTypes<'p>> = Vec::new();
    stack.extend(start_collecting(type_name, pool, type_set)?);
    while let Some(frame) = stack.last_mut() {
        if let Some(&referenced) = frame.names.get(frame.next) {
            frame.next += 1;
            if let Some(call) = start_collecting(referenced, pool, type_set)? {
                stack.push(call);
            }
            continue;
        }
        // An object or interface type collects its interfaces first, then the types of its
        // fields and their arguments.
        match frame.fields_of.take() {
            Some(GraphQLNamedType::Object(object)) => frame.restart(field_type_names(object.get_fields()?)),
            Some(GraphQLNamedType::Interface(interface)) => frame.restart(field_type_names(interface.get_fields()?)),
            _ => {
                stack.pop();
            }
        }
    }
    Ok(())
}

/// A `collectReferencedTypes` call in [`collect_referenced_types`]'s explicit stack: the names
/// of the types its current loop collects, and the type whose fields it collects after that.
struct ReferencedTypes<'p> {
    names: Vec<&'p str>,
    next: usize,
    fields_of: Option<&'p GraphQLNamedType>,
}

impl<'p> ReferencedTypes<'p> {
    fn restart(&mut self, names: Vec<&'p str>) {
        self.names = names;
        self.next = 0;
    }
}

/// A `collectReferencedTypes(typeName)` call up to its first recursive call: `None` when it
/// makes none.
fn start_collecting<'p>(
    type_name: &str,
    pool: &'p IndexMap<String, GraphQLNamedType>,
    type_set: &mut IndexSet<String>,
) -> Result<Option<ReferencedTypes<'p>>, GraphQLError> {
    if type_set.contains(type_name) {
        return Ok(None);
    }
    type_set.insert(type_name.to_string());
    let Some(named_type) = pool.get(type_name) else {
        return Ok(None);
    };
    let (names, fields_of): (Vec<&'p str>, _) = match named_type {
        GraphQLNamedType::Union(union) => (union.get_types()?.iter().map(String::as_str).collect(), None),
        GraphQLNamedType::Object(object) => (
            object.get_interfaces()?.iter().map(String::as_str).collect(),
            Some(named_type),
        ),
        GraphQLNamedType::Interface(interface) => (
            interface.get_interfaces()?.iter().map(String::as_str).collect(),
            Some(named_type),
        ),
        GraphQLNamedType::InputObject(input_object) => (
            input_object
                .get_fields()?
                .values()
                .map(|field| field.ty.named_type())
                .collect(),
            None,
        ),
        GraphQLNamedType::Scalar(_) | GraphQLNamedType::Enum(_) => return Ok(None),
    };
    Ok(Some(ReferencedTypes {
        names,
        next: 0,
        fields_of,
    }))
}

/// The types of the fields and of their arguments, in the order `collectReferencedTypes` visits
/// them.
fn field_type_names(fields: &IndexMap<String, GraphQLField>) -> Vec<&str> {
    let mut names = Vec::new();
    for field in fields.values() {
        names.push(field.ty.named_type());
        names.extend(field.args.iter().map(|arg| arg.ty.named_type()));
    }
    names
}
