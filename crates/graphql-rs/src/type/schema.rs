//! Port of graphql-js `type/schema`: `GraphQLSchema`.

use std::sync::OnceLock;

use crate::ast::{OperationType, SchemaDefinition, SchemaExtension};
use crate::collections::{IndexMap, IndexSet};
use crate::error::GraphQLError;

use super::definition::{GraphQLNamedType, GraphQLType, TypeLookup};
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

/// graphql-js `GraphQLSchema`.
#[derive(Debug)]
pub struct GraphQLSchema {
    pub description: Option<String>,
    pub ast_node: Option<SchemaDefinition>,
    pub extension_ast_nodes: Vec<SchemaExtension>,
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
    pub fn get_query_type(&self) -> Option<&GraphQLNamedType> {
        self.query_type.as_deref().and_then(|name| self.get_type(name))
    }

    /// graphql-js `getMutationType()`.
    pub fn get_mutation_type(&self) -> Option<&GraphQLNamedType> {
        self.mutation_type.as_deref().and_then(|name| self.get_type(name))
    }

    /// graphql-js `getSubscriptionType()`.
    pub fn get_subscription_type(&self) -> Option<&GraphQLNamedType> {
        self.subscription_type.as_deref().and_then(|name| self.get_type(name))
    }

    /// graphql-js `getRootType(operation)`.
    pub fn get_root_type(&self, operation: OperationType) -> Option<&GraphQLNamedType> {
        match operation {
            OperationType::Query => self.get_query_type(),
            OperationType::Mutation => self.get_mutation_type(),
            OperationType::Subscription => self.get_subscription_type(),
        }
    }

    /// graphql-js `getTypeMap()`.
    pub fn get_type_map(&self) -> &IndexMap<String, GraphQLNamedType> {
        &self.type_map
    }

    /// graphql-js `getType(name)`.
    pub fn get_type(&self, name: &str) -> Option<&GraphQLNamedType> {
        self.type_map.get(name)
    }

    /// graphql-js `getPossibleTypes(abstractType)`, by name.
    pub fn get_possible_types(&self, abstract_type: &GraphQLNamedType) -> Vec<String> {
        match abstract_type {
            GraphQLNamedType::Union(union) => union.types().to_vec(),
            _ => self.get_implementations(abstract_type.name()).objects,
        }
    }

    /// graphql-js `getImplementations(interfaceType)`, by interface name.
    pub fn get_implementations(&self, interface_type: &str) -> Implementations {
        self.implementations_map
            .get(interface_type)
            .cloned()
            .unwrap_or_default()
    }

    /// graphql-js `isSubType(abstractType, maybeSubType)`.
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
    pub fn get_directives(&self) -> &[GraphQLDirective] {
        &self.directives
    }

    /// graphql-js `getDirective(name)`.
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
fn collect_referenced_types(
    type_name: &str,
    pool: &IndexMap<String, GraphQLNamedType>,
    type_set: &mut IndexSet<String>,
) -> Result<(), GraphQLError> {
    if type_set.contains(type_name) {
        return Ok(());
    }
    type_set.insert(type_name.to_string());
    let Some(named_type) = pool.get(type_name) else {
        return Ok(());
    };
    let collect_type =
        |ty: &GraphQLType, type_set: &mut IndexSet<String>| collect_referenced_types(ty.named_type(), pool, type_set);
    match named_type {
        GraphQLNamedType::Union(union) => {
            for member_type in union.get_types()? {
                collect_referenced_types(member_type, pool, type_set)?;
            }
        }
        GraphQLNamedType::Object(object) => {
            for interface_type in object.get_interfaces()? {
                collect_referenced_types(interface_type, pool, type_set)?;
            }
            for field in object.get_fields()?.values() {
                collect_type(&field.ty, type_set)?;
                for arg in &field.args {
                    collect_type(&arg.ty, type_set)?;
                }
            }
        }
        GraphQLNamedType::Interface(interface) => {
            for interface_type in interface.get_interfaces()? {
                collect_referenced_types(interface_type, pool, type_set)?;
            }
            for field in interface.get_fields()?.values() {
                collect_type(&field.ty, type_set)?;
                for arg in &field.args {
                    collect_type(&arg.ty, type_set)?;
                }
            }
        }
        GraphQLNamedType::InputObject(input_object) => {
            for field in input_object.get_fields()?.values() {
                collect_type(&field.ty, type_set)?;
            }
        }
        GraphQLNamedType::Scalar(_) | GraphQLNamedType::Enum(_) => {}
    }
    Ok(())
}
