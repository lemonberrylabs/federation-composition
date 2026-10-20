//! Port of `src/contracts/reachable-type-filter.ts`.
//!
//! NOTE(port): the TS builds a `GraphQLSchema` with graphql-js `buildASTSchema` and walks it
//! with `getNamedType`/`isObjectType`/...; the port implements the subset needed here over the
//! document (root types, fields, arguments, interfaces, union members, input fields).
//! `buildASTSchema` first runs `assertValidSDL` (graphql-js `specifiedSDLRules`) and throws
//! `new Error(messages.join("\n\n"))` when the document is invalid; that validation is ported
//! below (`assert_valid_sdl` and the rule structs), with graphql-js's messages and order.

use std::collections::VecDeque;
use std::sync::LazyLock;

use crate::utils::collections::{IndexMap, IndexSet};
use graphql_rs::{
    is_type_definition_node, is_type_extension_node, is_type_system_definition_node, is_type_system_extension_node,
    visit, visit_in_parallel, Ancestor, ConstDirective, Definition, Directive, Document, FieldDefinition,
    InputValueDefinition, Kind, NamedType, NodeRef, Type, VisitAction, VisitInfo, Visitor,
};

use crate::utils::js::{JsError, JsResult};

/// `specifiedScalarNames`.
static SPECIFIED_SCALAR_NAMES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_rs::specified_scalar_types()
        .iter()
        .map(|t| t.name.to_string())
        .collect()
});

/// `getReachableTypes(documentNode)`: the names of all types reachable from the root types.
/// `Err` is the error `buildASTSchema` throws for an invalid SDL document.
pub fn get_reachable_types(document_node: &Document) -> JsResult<IndexSet<String>> {
    let mut reachable_type_names: IndexSet<String> = IndexSet::default();
    let schema = build_ast_schema(document_node)?;
    let mut did_visit_type: IndexSet<String> = IndexSet::default();
    let mut type_queue: VecDeque<String> = VecDeque::new();

    // NOTE(port): `didVisitType` holds type objects; a schema has one object per type name,
    // so the port tracks names.
    fn process_named_type(
        t_type: &str,
        did_visit_type: &mut IndexSet<String>,
        type_queue: &mut VecDeque<String>,
        reachable_type_names: &mut IndexSet<String>,
    ) {
        if did_visit_type.contains(t_type) || SPECIFIED_SCALAR_NAMES.contains(t_type) {
            return;
        }
        did_visit_type.insert(t_type.to_string());
        type_queue.push_back(t_type.to_string());
        reachable_type_names.insert(t_type.to_string());
    }

    if let Some(query_type) = schema.query_type {
        process_named_type(
            query_type,
            &mut did_visit_type,
            &mut type_queue,
            &mut reachable_type_names,
        );
    }
    if let Some(mutation_type) = schema.mutation_type {
        process_named_type(
            mutation_type,
            &mut did_visit_type,
            &mut type_queue,
            &mut reachable_type_names,
        );
    }
    if let Some(subscription_type) = schema.subscription_type {
        process_named_type(
            subscription_type,
            &mut did_visit_type,
            &mut type_queue,
            &mut reachable_type_names,
        );
    }

    // `schema.getDirectives()`: the document's directive definitions, then the specified
    // directives it does not redefine. Specified directives (by name) are skipped.
    for directive in &schema.directives {
        if graphql_rs::is_specified_directive(&directive.name.value) {
            continue;
        }

        for arg in &directive.arguments {
            let arg_type = schema.get_named_type(named_type_name(&arg.ty))?;
            process_named_type(
                arg_type,
                &mut did_visit_type,
                &mut type_queue,
                &mut reachable_type_names,
            );
        }
    }

    while let Some(current_type) = type_queue.pop_front() {
        let current = schema.get_type(&current_type)?;
        match current {
            SchemaType::Introspection(IntrospectionType::Object(referenced)) => {
                // `isObjectType`: field types and argument types, in field order; no interfaces.
                for name in referenced.iter() {
                    process_named_type(name, &mut did_visit_type, &mut type_queue, &mut reachable_type_names);
                }
            }
            SchemaType::Introspection(IntrospectionType::Enum) => {
                reachable_type_names.insert(current_type.clone());
            }
            SchemaType::User(user_type) => match user_type.kind {
                UserTypeKind::Object => {
                    for field in user_type.field_definitions() {
                        let field_type = schema.get_named_type(named_type_name(&field.ty))?;
                        process_named_type(
                            field_type,
                            &mut did_visit_type,
                            &mut type_queue,
                            &mut reachable_type_names,
                        );

                        for arg in &field.arguments {
                            let arg_type = schema.get_named_type(named_type_name(&arg.ty))?;
                            process_named_type(
                                arg_type,
                                &mut did_visit_type,
                                &mut type_queue,
                                &mut reachable_type_names,
                            );
                        }
                    }
                    for interface in user_type.interfaces() {
                        let interface = schema.get_named_type(&interface.name.value)?;
                        process_named_type(
                            interface,
                            &mut did_visit_type,
                            &mut type_queue,
                            &mut reachable_type_names,
                        );
                    }
                }
                UserTypeKind::InputObject => {
                    for field in user_type.input_fields() {
                        let field_type = schema.get_named_type(named_type_name(&field.ty))?;
                        process_named_type(
                            field_type,
                            &mut did_visit_type,
                            &mut type_queue,
                            &mut reachable_type_names,
                        );
                    }
                }
                UserTypeKind::Scalar | UserTypeKind::Enum => {
                    reachable_type_names.insert(current_type.clone());
                }
                UserTypeKind::Interface => {
                    // NOTE(port): the TS does not visit the arguments of interface fields.
                    for field in user_type.field_definitions() {
                        let field_type = schema.get_named_type(named_type_name(&field.ty))?;
                        process_named_type(
                            field_type,
                            &mut did_visit_type,
                            &mut type_queue,
                            &mut reachable_type_names,
                        );
                    }
                    for interface in user_type.interfaces() {
                        let interface = schema.get_named_type(&interface.name.value)?;
                        process_named_type(
                            interface,
                            &mut did_visit_type,
                            &mut type_queue,
                            &mut reachable_type_names,
                        );
                    }
                    for possible_type in schema.get_possible_types(&current_type) {
                        process_named_type(
                            possible_type,
                            &mut did_visit_type,
                            &mut type_queue,
                            &mut reachable_type_names,
                        );
                    }
                }
                UserTypeKind::Union => {
                    for member in user_type.union_types() {
                        let member = schema.get_named_type(&member.name.value)?;
                        process_named_type(member, &mut did_visit_type, &mut type_queue, &mut reachable_type_names);
                    }
                }
            },
        }
    }

    Ok(reachable_type_names)
}

/// `createAddDirectiveIfNotExists(directiveName)` returns this function: `directives` plus a
/// `@directiveName` if not present.
fn add_directive_if_not_exists(directive_name: &str, directives: &[ConstDirective]) -> Vec<ConstDirective> {
    let has_inaccessible_directive = directives
        .iter()
        .any(|directive| directive.name.value == directive_name);

    if !has_inaccessible_directive {
        let mut result = directives.to_vec();
        result.push(Directive::new(directive_name, Vec::new()));
        return result;
    }
    directives.to_vec()
}

/// The argument of `addDirectiveOnTypes`.
#[derive(Clone, Copy, Debug)]
pub struct AddDirectiveOnTypesArgs<'a> {
    /// `documentNode`: the supergraph.
    pub document_node: &'a Document,
    /// A set of excluded types, e.g. from `get_reachable_types`.
    pub excluded_type_names: &'a IndexSet<String>,
    /// Name of the directive to add on the types.
    pub directive_name: &'a str,
}

/// `addDirectiveOnTypes(args)`: adds the directive to every named type definition not in
/// `excluded_type_names`.
/// NOTE(port): the TS `visit`s the document with callbacks on the six type definition kinds;
/// those nodes only occur in `document.definitions`, so the port maps the definitions.
pub fn add_directive_on_types(args: AddDirectiveOnTypesArgs<'_>) -> Document {
    let excluded = args.excluded_type_names;
    let directive_name = args.directive_name;
    let definitions = args
        .document_node
        .definitions
        .iter()
        .map(|definition| {
            let mut definition = definition.clone();
            let directives = match &mut definition {
                Definition::ObjectTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::InterfaceTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::UnionTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::EnumTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::InputObjectTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::ScalarTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                _ => None,
            };
            if let Some((name, directives)) = directives {
                if !excluded.contains(name.as_str()) {
                    *directives = add_directive_if_not_exists(directive_name, directives);
                }
            }
            definition
        })
        .collect();
    Document { definitions }
}

// ---------------------------------------------------------------------------------------------
// The subset of graphql-js `buildASTSchema` used above.
// ---------------------------------------------------------------------------------------------

/// The named type of a type reference (graphql-js `getNamedType` on the built type).
fn named_type_name(ty: &Type) -> &str {
    match ty {
        Type::Named(n) => &n.name.value,
        Type::List(l) => named_type_name(&l.ty),
        Type::NonNull(n) => named_type_name(&n.ty),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UserTypeKind {
    Scalar,
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
}

/// A type built from a definition and its extensions (`[astNode, ...extensionASTNodes]`).
#[derive(Debug)]
struct UserType<'a> {
    kind: UserTypeKind,
    nodes: Vec<&'a Definition>,
}

impl<'a> UserType<'a> {
    /// `getFields()` of an object or interface type: the definition's fields, then each
    /// extension's.
    fn field_definitions(&self) -> impl Iterator<Item = &'a FieldDefinition> + '_ {
        self.nodes.iter().flat_map(|node| -> &'a [FieldDefinition] {
            match node {
                Definition::ObjectTypeDefinition(n) => &n.fields,
                Definition::ObjectTypeExtension(n) => &n.fields,
                Definition::InterfaceTypeDefinition(n) => &n.fields,
                Definition::InterfaceTypeExtension(n) => &n.fields,
                _ => &[],
            }
        })
    }

    /// `getFields()` of an input object type.
    fn input_fields(&self) -> impl Iterator<Item = &'a InputValueDefinition> + '_ {
        self.nodes.iter().flat_map(|node| -> &'a [InputValueDefinition] {
            match node {
                Definition::InputObjectTypeDefinition(n) => &n.fields,
                Definition::InputObjectTypeExtension(n) => &n.fields,
                _ => &[],
            }
        })
    }

    /// `getInterfaces()`.
    fn interfaces(&self) -> impl Iterator<Item = &'a NamedType> + '_ {
        self.nodes.iter().flat_map(|node| -> &'a [NamedType] {
            match node {
                Definition::ObjectTypeDefinition(n) => &n.interfaces,
                Definition::ObjectTypeExtension(n) => &n.interfaces,
                Definition::InterfaceTypeDefinition(n) => &n.interfaces,
                Definition::InterfaceTypeExtension(n) => &n.interfaces,
                _ => &[],
            }
        })
    }

    /// `getTypes()` of a union.
    fn union_types(&self) -> impl Iterator<Item = &'a NamedType> + '_ {
        self.nodes.iter().flat_map(|node| -> &'a [NamedType] {
            match node {
                Definition::UnionTypeDefinition(n) => &n.types,
                Definition::UnionTypeExtension(n) => &n.types,
                _ => &[],
            }
        })
    }
}

/// The graphql-js introspection types, as far as the walk sees them.
#[derive(Clone, Copy, Debug)]
enum IntrospectionType {
    /// An object type: the named types of its fields and their arguments, in field order.
    Object(&'static [&'static str]),
    Enum,
}

/// graphql-js `introspectionTypes` (field and argument types in definition order).
fn introspection_type(name: &str) -> Option<IntrospectionType> {
    use IntrospectionType::*;
    Some(match name {
        "__Schema" => Object(&[
            "String",
            "__Type",
            "__Type",
            "__Type",
            "__Type",
            "__Directive",
            "Boolean",
        ]),
        "__Directive" => Object(&[
            "String",
            "String",
            "Boolean",
            "__DirectiveLocation",
            "__InputValue",
            "Boolean",
            "Boolean",
            "String",
        ]),
        "__DirectiveLocation" => Enum,
        "__Type" => Object(&[
            "__TypeKind",
            "String",
            "String",
            "String",
            "__Field",
            "Boolean",
            "__Type",
            "__Type",
            "__EnumValue",
            "Boolean",
            "__InputValue",
            "Boolean",
            "__Type",
            "Boolean",
        ]),
        "__Field" => Object(&[
            "String",
            "String",
            "__InputValue",
            "Boolean",
            "__Type",
            "Boolean",
            "String",
        ]),
        "__InputValue" => Object(&["String", "String", "__Type", "String", "Boolean", "String"]),
        "__EnumValue" => Object(&["String", "String", "Boolean", "String"]),
        "__TypeKind" => Enum,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug)]
enum SchemaType<'s, 'a> {
    Introspection(IntrospectionType),
    User(&'s UserType<'a>),
}

/// The parts of the `GraphQLSchema` built by `buildASTSchema` that the walk reads.
#[derive(Debug)]
struct AstSchema<'a> {
    query_type: Option<&'a str>,
    mutation_type: Option<&'a str>,
    subscription_type: Option<&'a str>,
    /// `getDirectives()`.
    directives: Vec<&'a graphql_rs::DirectiveDefinition>,
    /// The type map (built from the document's type definitions, in document order).
    types: IndexMap<&'a str, UserType<'a>>,
}

impl<'a> AstSchema<'a> {
    /// The mapSchemaConfig `getNamedType(typeName)`: standard types first, then the document's
    /// types; an unknown name is graphql-js's invariant error.
    fn get_named_type<'n>(&self, type_name: &'n str) -> JsResult<&'n str> {
        if SPECIFIED_SCALAR_NAMES.contains(type_name)
            || introspection_type(type_name).is_some()
            || self.types.contains_key(type_name)
        {
            return Ok(type_name);
        }
        Err(JsError::error(format!("Unknown type: \"{type_name}\".")))
    }

    /// The type object behind a (non specified scalar) type name.
    fn get_type(&self, type_name: &str) -> JsResult<SchemaType<'_, 'a>> {
        if let Some(introspection) = introspection_type(type_name) {
            return Ok(SchemaType::Introspection(introspection));
        }
        match self.types.get(type_name) {
            Some(user_type) => Ok(SchemaType::User(user_type)),
            None => Err(JsError::error(format!("Unknown type: \"{type_name}\"."))),
        }
    }

    /// `getPossibleTypes(interfaceType)`: the object types that implement it.
    /// NOTE(port): graphql-js lists them in its type map order (collected from the root types
    /// first); the port lists them in document order. Only the set of names is observable.
    fn get_possible_types<'s>(&'s self, interface_name: &'s str) -> impl Iterator<Item = &'a str> + 's {
        self.types
            .iter()
            .filter(move |(name, user_type)| {
                user_type.kind == UserTypeKind::Object
                    && !SPECIFIED_SCALAR_NAMES.contains(**name)
                    && introspection_type(name).is_none()
                    && user_type.interfaces().any(|i| i.name.value == interface_name)
            })
            .map(|(name, _)| *name)
    }
}

/// `buildASTSchema(documentAST)` (subset).
fn build_ast_schema<'a>(document_ast: &'a Document) -> JsResult<AstSchema<'a>> {
    assert_valid_sdl(document_ast)?;

    let mut schema_def: Option<&graphql_rs::SchemaDefinition> = None;
    let mut schema_extensions: Vec<&graphql_rs::SchemaExtension> = Vec::new();
    let mut directive_defs: Vec<&graphql_rs::DirectiveDefinition> = Vec::new();
    let mut types: IndexMap<&str, UserType<'_>> = IndexMap::default();
    let mut extensions: Vec<&Definition> = Vec::new();

    for def in &document_ast.definitions {
        let kind = match def {
            Definition::SchemaDefinition(d) => {
                schema_def = Some(d);
                continue;
            }
            Definition::SchemaExtension(d) => {
                schema_extensions.push(d);
                continue;
            }
            Definition::DirectiveDefinition(d) => {
                directive_defs.push(d);
                continue;
            }
            Definition::ScalarTypeDefinition(_) => UserTypeKind::Scalar,
            Definition::ObjectTypeDefinition(_) => UserTypeKind::Object,
            Definition::InterfaceTypeDefinition(_) => UserTypeKind::Interface,
            Definition::UnionTypeDefinition(_) => UserTypeKind::Union,
            Definition::EnumTypeDefinition(_) => UserTypeKind::Enum,
            Definition::InputObjectTypeDefinition(_) => UserTypeKind::InputObject,
            Definition::ScalarTypeExtension(_)
            | Definition::ObjectTypeExtension(_)
            | Definition::InterfaceTypeExtension(_)
            | Definition::UnionTypeExtension(_)
            | Definition::EnumTypeExtension(_)
            | Definition::InputObjectTypeExtension(_) => {
                extensions.push(def);
                continue;
            }
            _ => continue,
        };
        let name = def.as_node_ref().name().map(|n| n.value.as_str()).unwrap_or_default();
        // `typeMap[name] = ...`: unique type names are guaranteed by the SDL validation.
        types.insert(name, UserType { kind, nodes: vec![def] });
    }
    // Extensions are applied to the types they extend (validated to exist with the same kind).
    for ext in extensions {
        let name = ext.as_node_ref().name().map(|n| n.value.as_str()).unwrap_or_default();
        if let Some(user_type) = types.get_mut(name) {
            user_type.nodes.push(ext);
        }
    }

    let mut query_type: Option<&str> = None;
    let mut mutation_type: Option<&str> = None;
    let mut subscription_type: Option<&str> = None;
    {
        let mut set_operation_types = |operation_types: &'a [graphql_rs::OperationTypeDefinition]| {
            for operation_type in operation_types {
                let name = operation_type.ty.name.value.as_str();
                match operation_type.operation {
                    graphql_rs::OperationType::Query => query_type = Some(name),
                    graphql_rs::OperationType::Mutation => mutation_type = Some(name),
                    graphql_rs::OperationType::Subscription => subscription_type = Some(name),
                }
            }
        };
        // `...(schemaDef && getOperationTypes([schemaDef])), ...getOperationTypes(schemaExtensions)`
        if let Some(schema_def) = schema_def {
            set_operation_types(&schema_def.operation_types);
        }
        for ext in &schema_extensions {
            set_operation_types(&ext.operation_types);
        }
    }
    // `if (config.astNode == null)`: without a schema definition, the types named `Query`,
    // `Mutation` and `Subscription` are the root types (overriding `extend schema`).
    if schema_def.is_none() {
        for name in types.keys() {
            match *name {
                "Query" => query_type = Some(name),
                "Mutation" => mutation_type = Some(name),
                "Subscription" => subscription_type = Some(name),
                _ => {}
            }
        }
    }

    let mut directives: Vec<&graphql_rs::DirectiveDefinition> = directive_defs.clone();
    for std_directive in graphql_rs::specified_directives() {
        if directive_defs
            .iter()
            .all(|directive| directive.name.value != std_directive.name.value)
        {
            directives.push(std_directive);
        }
    }

    Ok(AstSchema {
        query_type,
        mutation_type,
        subscription_type,
        directives,
        types,
    })
}

// ---------------------------------------------------------------------------------------------
// graphql-js `assertValidSDL` with `specifiedSDLRules` (no schema to extend).
// ---------------------------------------------------------------------------------------------

/// The `SDLValidationContext` (no schema; suggestions are not hidden).
#[derive(Debug, Default)]
struct SdlValidationContext {
    errors: Vec<String>,
}

impl SdlValidationContext {
    fn report_error(&mut self, message: String) {
        self.errors.push(message);
    }
}

/// `assertValidSDL(documentAST)`.
fn assert_valid_sdl(document_ast: &Document) -> JsResult<()> {
    let errors = validate_sdl(document_ast);
    if !errors.is_empty() {
        return Err(JsError::error(errors.join("\n\n")));
    }
    Ok(())
}

/// `validateSDL(documentAST)`: the messages of the reported errors, in report order.
fn validate_sdl(document_ast: &Document) -> Vec<String> {
    let mut context = SdlValidationContext::default();
    let visitors: Vec<Box<dyn Visitor<'_, SdlValidationContext> + '_>> = vec![
        Box::new(LoneSchemaDefinitionRule::default()),
        Box::new(UniqueOperationTypesRule::default()),
        Box::new(UniqueTypeNamesRule::default()),
        Box::new(UniqueEnumValueNamesRule::default()),
        Box::new(UniqueFieldDefinitionNamesRule::default()),
        Box::new(UniqueArgumentDefinitionNamesRule),
        Box::new(UniqueDirectiveNamesRule::default()),
        Box::new(KnownTypeNamesRule::new(document_ast)),
        Box::new(KnownDirectivesRule::new(document_ast)),
        Box::new(UniqueDirectivesPerLocationRule::new(document_ast)),
        Box::new(PossibleTypeExtensionsRule::new(document_ast)),
        Box::new(KnownArgumentNamesOnDirectivesRule::new(document_ast)),
        Box::new(UniqueArgumentNamesRule),
        Box::new(UniqueInputFieldNamesRule::default()),
        Box::new(ProvidedRequiredArgumentsOnDirectivesRule::new(document_ast)),
    ];
    let mut parallel = visit_in_parallel(visitors);
    visit(document_ast, &mut parallel, &mut context);
    context.errors
}

/// The directives of a node that has a `directives` field.
fn node_directives(node: NodeRef<'_>) -> Option<&[Directive]> {
    Some(match node {
        NodeRef::OperationDefinition(n) => &n.directives,
        NodeRef::VariableDefinition(n) => &n.directives,
        NodeRef::Field(n) => &n.directives,
        NodeRef::FragmentSpread(n) => &n.directives,
        NodeRef::InlineFragment(n) => &n.directives,
        NodeRef::FragmentDefinition(n) => &n.directives,
        NodeRef::SchemaDefinition(n) => &n.directives,
        NodeRef::ScalarTypeDefinition(n) => &n.directives,
        NodeRef::ObjectTypeDefinition(n) => &n.directives,
        NodeRef::FieldDefinition(n) => &n.directives,
        NodeRef::InputValueDefinition(n) => &n.directives,
        NodeRef::InterfaceTypeDefinition(n) => &n.directives,
        NodeRef::UnionTypeDefinition(n) => &n.directives,
        NodeRef::EnumTypeDefinition(n) => &n.directives,
        NodeRef::EnumValueDefinition(n) => &n.directives,
        NodeRef::InputObjectTypeDefinition(n) => &n.directives,
        NodeRef::DirectiveDefinition(n) => &n.directives,
        NodeRef::SchemaExtension(n) => &n.directives,
        NodeRef::DirectiveExtension(n) => &n.directives,
        NodeRef::ScalarTypeExtension(n) => &n.directives,
        NodeRef::ObjectTypeExtension(n) => &n.directives,
        NodeRef::InterfaceTypeExtension(n) => &n.directives,
        NodeRef::UnionTypeExtension(n) => &n.directives,
        NodeRef::EnumTypeExtension(n) => &n.directives,
        NodeRef::InputObjectTypeExtension(n) => &n.directives,
        _ => return None,
    })
}

/// `node.name.value` for a node known to have a name.
fn node_name(node: NodeRef<'_>) -> &str {
    node.name().map(|n| n.value.as_str()).unwrap_or_default()
}

/// graphql-js `groupBy(list, keyFn)` over names, keeping first-seen order.
fn group_by_name<'n>(names: impl Iterator<Item = &'n str>) -> IndexMap<&'n str, usize> {
    let mut result: IndexMap<&str, usize> = IndexMap::default();
    for name in names {
        *result.entry(name).or_insert(0) += 1;
    }
    result
}

/// `LoneSchemaDefinitionRule`.
#[derive(Default)]
struct LoneSchemaDefinitionRule {
    schema_definitions_count: usize,
}

impl<'a> Visitor<'a, SdlValidationContext> for LoneSchemaDefinitionRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::SchemaDefinition(_) = node {
            if self.schema_definitions_count > 0 {
                cx.report_error("Must provide only one schema definition.".to_string());
            }
            self.schema_definitions_count += 1;
        }
        VisitAction::Continue
    }
}

/// `UniqueOperationTypesRule`.
#[derive(Default)]
struct UniqueOperationTypesRule {
    defined_operation_types: IndexSet<&'static str>,
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueOperationTypesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let operation_types_nodes = match node {
            NodeRef::SchemaDefinition(n) => &n.operation_types,
            NodeRef::SchemaExtension(n) => &n.operation_types,
            _ => return VisitAction::Continue,
        };
        for operation_type in operation_types_nodes {
            let operation = operation_type.operation.as_str();
            if self.defined_operation_types.contains(operation) {
                cx.report_error(format!("There can be only one {operation} type in schema."));
            } else {
                self.defined_operation_types.insert(operation);
            }
        }
        VisitAction::Skip
    }
}

/// `UniqueTypeNamesRule`.
#[derive(Default)]
struct UniqueTypeNamesRule {
    known_type_names: IndexSet<String>,
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueTypeNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        if !is_type_definition_node(node) {
            return VisitAction::Continue;
        }
        let type_name = node_name(node);
        if self.known_type_names.contains(type_name) {
            cx.report_error(format!("There can be only one type named \"{type_name}\"."));
        } else {
            self.known_type_names.insert(type_name.to_string());
        }
        VisitAction::Skip
    }
}

/// `UniqueEnumValueNamesRule`.
#[derive(Default)]
struct UniqueEnumValueNamesRule {
    known_value_names: IndexMap<String, IndexSet<String>>,
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueEnumValueNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let value_nodes = match node {
            NodeRef::EnumTypeDefinition(n) => &n.values,
            NodeRef::EnumTypeExtension(n) => &n.values,
            _ => return VisitAction::Continue,
        };
        let type_name = node_name(node);
        let value_names = self.known_value_names.entry(type_name.to_string()).or_default();
        for value_def in value_nodes {
            let value_name = &value_def.name.value;
            if value_names.contains(value_name) {
                cx.report_error(format!(
                    "Enum value \"{type_name}.{value_name}\" can only be defined once."
                ));
            } else {
                value_names.insert(value_name.clone());
            }
        }
        VisitAction::Skip
    }
}

/// `UniqueFieldDefinitionNamesRule`.
#[derive(Default)]
struct UniqueFieldDefinitionNamesRule {
    known_field_names: IndexMap<String, IndexSet<String>>,
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueFieldDefinitionNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let field_names_of_node: Vec<&str> = match node {
            NodeRef::InputObjectTypeDefinition(n) => n.fields.iter().map(|f| f.name.value.as_str()).collect(),
            NodeRef::InputObjectTypeExtension(n) => n.fields.iter().map(|f| f.name.value.as_str()).collect(),
            NodeRef::InterfaceTypeDefinition(n) => n.fields.iter().map(|f| f.name.value.as_str()).collect(),
            NodeRef::InterfaceTypeExtension(n) => n.fields.iter().map(|f| f.name.value.as_str()).collect(),
            NodeRef::ObjectTypeDefinition(n) => n.fields.iter().map(|f| f.name.value.as_str()).collect(),
            NodeRef::ObjectTypeExtension(n) => n.fields.iter().map(|f| f.name.value.as_str()).collect(),
            _ => return VisitAction::Continue,
        };
        let type_name = node_name(node);
        let field_names = self.known_field_names.entry(type_name.to_string()).or_default();
        for field_name in field_names_of_node {
            if field_names.contains(field_name) {
                cx.report_error(format!("Field \"{type_name}.{field_name}\" can only be defined once."));
            } else {
                field_names.insert(field_name.to_string());
            }
        }
        VisitAction::Skip
    }
}

/// `UniqueArgumentDefinitionNamesRule`.
struct UniqueArgumentDefinitionNamesRule;

impl UniqueArgumentDefinitionNamesRule {
    fn check_arg_uniqueness(cx: &mut SdlValidationContext, parent_name: &str, argument_nodes: &[InputValueDefinition]) {
        let seen_args = group_by_name(argument_nodes.iter().map(|arg| arg.name.value.as_str()));
        for (arg_name, count) in seen_args {
            if count > 1 {
                cx.report_error(format!(
                    "Argument \"{parent_name}({arg_name}:)\" can only be defined once."
                ));
            }
        }
    }
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueArgumentDefinitionNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let field_nodes = match node {
            NodeRef::DirectiveDefinition(directive_node) => {
                Self::check_arg_uniqueness(
                    cx,
                    &format!("@{}", directive_node.name.value),
                    &directive_node.arguments,
                );
                return VisitAction::Skip;
            }
            NodeRef::InterfaceTypeDefinition(n) => &n.fields,
            NodeRef::InterfaceTypeExtension(n) => &n.fields,
            NodeRef::ObjectTypeDefinition(n) => &n.fields,
            NodeRef::ObjectTypeExtension(n) => &n.fields,
            _ => return VisitAction::Continue,
        };
        let type_name = node_name(node);
        for field_def in field_nodes {
            let field_name = &field_def.name.value;
            Self::check_arg_uniqueness(cx, &format!("{type_name}.{field_name}"), &field_def.arguments);
        }
        VisitAction::Skip
    }
}

/// `UniqueDirectiveNamesRule`.
#[derive(Default)]
struct UniqueDirectiveNamesRule {
    known_directive_names: IndexSet<String>,
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueDirectiveNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::DirectiveDefinition(node) = node else {
            return VisitAction::Continue;
        };
        let directive_name = &node.name.value;
        if self.known_directive_names.contains(directive_name) {
            cx.report_error(format!("There can be only one directive named \"@{directive_name}\"."));
        } else {
            self.known_directive_names.insert(directive_name.clone());
        }
        VisitAction::Skip
    }
}

/// `standardTypeNames`: the specified scalars and the introspection types.
static STANDARD_TYPE_NAMES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_rs::specified_scalar_types()
        .iter()
        .map(|t| t.name.to_string())
        .chain(graphql_rs::INTROSPECTION_TYPE_NAMES.iter().map(|n| n.to_string()))
        .collect()
});

/// `KnownTypeNamesRule`.
struct KnownTypeNamesRule {
    type_names: IndexSet<String>,
}

impl KnownTypeNamesRule {
    fn new(document: &Document) -> Self {
        KnownTypeNamesRule {
            type_names: document
                .definitions
                .iter()
                .filter(|def| is_type_definition_node(*def))
                .map(|def| node_name(def.as_node_ref()).to_string())
                .collect(),
        }
    }
}

/// `isSDLNode(value)`.
fn is_sdl_node(value: Ancestor<'_>) -> bool {
    match value.as_node() {
        Some(node) => is_type_system_definition_node(node) || is_type_system_extension_node(node),
        None => false,
    }
}

impl<'a> Visitor<'a, SdlValidationContext> for KnownTypeNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::NamedType(node) = node else {
            return VisitAction::Continue;
        };
        let type_name = &node.name.value;
        if !self.type_names.contains(type_name) {
            let definition_node = info.ancestors.get(2).copied().or(info.parent);
            let is_sdl = definition_node.is_some_and(is_sdl_node);
            if is_sdl && STANDARD_TYPE_NAMES.contains(type_name) {
                return VisitAction::Continue;
            }

            let options: Vec<&str> = if is_sdl {
                STANDARD_TYPE_NAMES
                    .iter()
                    .chain(self.type_names.iter())
                    .map(String::as_str)
                    .collect()
            } else {
                self.type_names.iter().map(String::as_str).collect()
            };
            let suggested_types = suggestion_list(type_name, &options);
            cx.report_error(format!(
                "Unknown type \"{type_name}\".{}",
                did_you_mean(&suggested_types)
            ));
        }
        VisitAction::Continue
    }
}

/// `KnownDirectivesRule`.
struct KnownDirectivesRule {
    locations_map: IndexMap<String, Vec<String>>,
}

impl KnownDirectivesRule {
    fn new(document: &Document) -> Self {
        let mut locations_map: IndexMap<String, Vec<String>> = IndexMap::default();
        for directive in graphql_rs::specified_directives() {
            locations_map.insert(
                directive.name.value.clone(),
                directive.locations.iter().map(|l| l.value.clone()).collect(),
            );
        }
        for def in &document.definitions {
            if let Definition::DirectiveDefinition(def) = def {
                locations_map.insert(
                    def.name.value.clone(),
                    def.locations.iter().map(|name| name.value.clone()).collect(),
                );
            }
        }
        KnownDirectivesRule { locations_map }
    }
}

impl<'a> Visitor<'a, SdlValidationContext> for KnownDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::Directive(node) = node else {
            return VisitAction::Continue;
        };
        let name = &node.name.value;
        let Some(locations) = self.locations_map.get(name) else {
            cx.report_error(format!("Unknown directive \"@{name}\"."));
            return VisitAction::Continue;
        };
        let candidate_location = get_directive_location_for_ast_path(info.ancestors);
        if let Some(candidate_location) = candidate_location {
            if !locations.iter().any(|l| l == candidate_location) {
                cx.report_error(format!(
                    "Directive \"@{name}\" may not be used on {candidate_location}."
                ));
            }
        }
        VisitAction::Continue
    }
}

/// `getDirectiveLocationForASTPath(ancestors)`.
/// NOTE(port): graphql-js throws an invariant error for a node kind without a location; every
/// node with `directives` has one, so that branch is `None` here.
fn get_directive_location_for_ast_path(ancestors: &[Ancestor<'_>]) -> Option<&'static str> {
    let applied_to = ancestors.last().and_then(|a| a.as_node())?;
    Some(match applied_to {
        NodeRef::OperationDefinition(n) => match n.operation {
            graphql_rs::OperationType::Query => "QUERY",
            graphql_rs::OperationType::Mutation => "MUTATION",
            graphql_rs::OperationType::Subscription => "SUBSCRIPTION",
        },
        NodeRef::Field(_) => "FIELD",
        NodeRef::FragmentSpread(_) => "FRAGMENT_SPREAD",
        NodeRef::InlineFragment(_) => "INLINE_FRAGMENT",
        NodeRef::FragmentDefinition(_) => "FRAGMENT_DEFINITION",
        NodeRef::VariableDefinition(_) => {
            let parent_node = ancestors
                .len()
                .checked_sub(3)
                .and_then(|i| ancestors.get(i))
                .and_then(|a| a.as_node())?;
            if parent_node.kind() == Kind::OperationDefinition {
                "VARIABLE_DEFINITION"
            } else {
                "FRAGMENT_VARIABLE_DEFINITION"
            }
        }
        NodeRef::SchemaDefinition(_) | NodeRef::SchemaExtension(_) => "SCHEMA",
        NodeRef::ScalarTypeDefinition(_) | NodeRef::ScalarTypeExtension(_) => "SCALAR",
        NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_) => "OBJECT",
        NodeRef::FieldDefinition(_) => "FIELD_DEFINITION",
        NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_) => "INTERFACE",
        NodeRef::UnionTypeDefinition(_) | NodeRef::UnionTypeExtension(_) => "UNION",
        NodeRef::EnumTypeDefinition(_) | NodeRef::EnumTypeExtension(_) => "ENUM",
        NodeRef::EnumValueDefinition(_) => "ENUM_VALUE",
        NodeRef::InputObjectTypeDefinition(_) | NodeRef::InputObjectTypeExtension(_) => "INPUT_OBJECT",
        NodeRef::InputValueDefinition(_) => {
            let parent_node = ancestors
                .len()
                .checked_sub(3)
                .and_then(|i| ancestors.get(i))
                .and_then(|a| a.as_node())?;
            if matches!(
                parent_node.kind(),
                Kind::InputObjectTypeDefinition | Kind::InputObjectTypeExtension
            ) {
                "INPUT_FIELD_DEFINITION"
            } else {
                "ARGUMENT_DEFINITION"
            }
        }
        NodeRef::DirectiveDefinition(_) | NodeRef::DirectiveExtension(_) => "DIRECTIVE_DEFINITION",
        _ => return None,
    })
}

/// `UniqueDirectivesPerLocationRule`.
struct UniqueDirectivesPerLocationRule {
    unique_directive_map: IndexMap<String, bool>,
    schema_directives: IndexSet<String>,
    type_directives_map: IndexMap<String, IndexSet<String>>,
    directive_directives_map: IndexMap<String, IndexSet<String>>,
}

impl UniqueDirectivesPerLocationRule {
    fn new(document: &Document) -> Self {
        let mut unique_directive_map: IndexMap<String, bool> = IndexMap::default();
        for directive in graphql_rs::specified_directives() {
            unique_directive_map.insert(directive.name.value.clone(), !directive.repeatable);
        }
        for def in &document.definitions {
            if let Definition::DirectiveDefinition(def) = def {
                unique_directive_map.insert(def.name.value.clone(), !def.repeatable);
            }
        }
        UniqueDirectivesPerLocationRule {
            unique_directive_map,
            schema_directives: IndexSet::default(),
            type_directives_map: IndexMap::default(),
            directive_directives_map: IndexMap::default(),
        }
    }
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueDirectivesPerLocationRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let Some(directives) = node_directives(node) else {
            return VisitAction::Continue;
        };
        let mut local_seen_directives = IndexSet::default();
        let seen_directives: &mut IndexSet<String> =
            if matches!(node.kind(), Kind::SchemaDefinition | Kind::SchemaExtension) {
                &mut self.schema_directives
            } else if is_type_definition_node(node) || is_type_extension_node(node) {
                self.type_directives_map.entry(node_name(node).to_string()).or_default()
            } else if matches!(node.kind(), Kind::DirectiveDefinition | Kind::DirectiveExtension) {
                self.directive_directives_map
                    .entry(node_name(node).to_string())
                    .or_default()
            } else {
                &mut local_seen_directives
            };

        for directive in directives {
            let directive_name = &directive.name.value;
            if self.unique_directive_map.get(directive_name) == Some(&true) {
                if seen_directives.contains(directive_name) {
                    cx.report_error(format!(
                        "The directive \"@{directive_name}\" can only be used once at this location."
                    ));
                } else {
                    seen_directives.insert(directive_name.clone());
                }
            }
        }
        VisitAction::Continue
    }
}

/// `PossibleTypeExtensionsRule`.
struct PossibleTypeExtensionsRule {
    /// Type name to the kind of its (last) definition.
    defined_types: IndexMap<String, Kind>,
}

impl PossibleTypeExtensionsRule {
    fn new(document: &Document) -> Self {
        let mut defined_types = IndexMap::default();
        for def in &document.definitions {
            if is_type_definition_node(def) {
                defined_types.insert(node_name(def.as_node_ref()).to_string(), def.kind());
            }
        }
        PossibleTypeExtensionsRule { defined_types }
    }
}

/// `defKindToExtKind`.
fn def_kind_to_ext_kind(kind: Kind) -> Option<Kind> {
    Some(match kind {
        Kind::ScalarTypeDefinition => Kind::ScalarTypeExtension,
        Kind::ObjectTypeDefinition => Kind::ObjectTypeExtension,
        Kind::InterfaceTypeDefinition => Kind::InterfaceTypeExtension,
        Kind::UnionTypeDefinition => Kind::UnionTypeExtension,
        Kind::EnumTypeDefinition => Kind::EnumTypeExtension,
        Kind::InputObjectTypeDefinition => Kind::InputObjectTypeExtension,
        _ => return None,
    })
}

/// `extensionKindToTypeName(kind)`.
fn extension_kind_to_type_name(kind: Kind) -> &'static str {
    match kind {
        Kind::ScalarTypeExtension => "scalar",
        Kind::ObjectTypeExtension => "object",
        Kind::InterfaceTypeExtension => "interface",
        Kind::UnionTypeExtension => "union",
        Kind::EnumTypeExtension => "enum",
        Kind::InputObjectTypeExtension => "input object",
        _ => "",
    }
}

impl<'a> Visitor<'a, SdlValidationContext> for PossibleTypeExtensionsRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        if !is_type_extension_node(node) {
            return VisitAction::Continue;
        }
        let type_name = node_name(node);
        let expected_kind = self.defined_types.get(type_name).and_then(|k| def_kind_to_ext_kind(*k));

        if let Some(expected_kind) = expected_kind {
            if expected_kind != node.kind() {
                let kind_str = extension_kind_to_type_name(node.kind());
                cx.report_error(format!("Cannot extend non-{kind_str} type \"{type_name}\"."));
            }
        } else {
            let all_type_names: Vec<&str> = self.defined_types.keys().map(String::as_str).collect();
            cx.report_error(format!(
                "Cannot extend type \"{type_name}\" because it is not defined.{}",
                did_you_mean(&suggestion_list(type_name, &all_type_names))
            ));
        }
        VisitAction::Continue
    }
}

/// `KnownArgumentNamesOnDirectivesRule`.
struct KnownArgumentNamesOnDirectivesRule {
    directive_args: IndexMap<String, Vec<String>>,
}

impl KnownArgumentNamesOnDirectivesRule {
    fn new(document: &Document) -> Self {
        let mut directive_args: IndexMap<String, Vec<String>> = IndexMap::default();
        for directive in graphql_rs::specified_directives() {
            directive_args.insert(
                directive.name.value.clone(),
                directive.arguments.iter().map(|arg| arg.name.value.clone()).collect(),
            );
        }
        for def in &document.definitions {
            if let Definition::DirectiveDefinition(def) = def {
                directive_args.insert(
                    def.name.value.clone(),
                    def.arguments.iter().map(|arg| arg.name.value.clone()).collect(),
                );
            }
        }
        KnownArgumentNamesOnDirectivesRule { directive_args }
    }
}

impl<'a> Visitor<'a, SdlValidationContext> for KnownArgumentNamesOnDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::Directive(directive_node) = node else {
            return VisitAction::Continue;
        };
        let directive_name = &directive_node.name.value;
        if let Some(known_args) = self.directive_args.get(directive_name) {
            for arg_node in &directive_node.arguments {
                let arg_name = &arg_node.name.value;
                if !known_args.contains(arg_name) {
                    let options: Vec<&str> = known_args.iter().map(String::as_str).collect();
                    let suggestions = suggestion_list(arg_name, &options);
                    cx.report_error(format!(
                        "Unknown argument \"{arg_name}\" on directive \"@{directive_name}\".{}",
                        did_you_mean(&suggestions)
                    ));
                }
            }
        }
        VisitAction::Skip
    }
}

/// `UniqueArgumentNamesRule`.
struct UniqueArgumentNamesRule;

impl<'a> Visitor<'a, SdlValidationContext> for UniqueArgumentNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let argument_nodes = match node {
            NodeRef::Field(n) => &n.arguments,
            NodeRef::Directive(n) => &n.arguments,
            _ => return VisitAction::Continue,
        };
        let seen_args = group_by_name(argument_nodes.iter().map(|arg| arg.name.value.as_str()));
        for (arg_name, count) in seen_args {
            if count > 1 {
                cx.report_error(format!("There can be only one argument named \"{arg_name}\"."));
            }
        }
        VisitAction::Continue
    }
}

/// `UniqueInputFieldNamesRule`.
#[derive(Default)]
struct UniqueInputFieldNamesRule {
    known_name_stack: Vec<IndexSet<String>>,
    known_names: IndexSet<String>,
}

impl<'a> Visitor<'a, SdlValidationContext> for UniqueInputFieldNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::ObjectValue(_) => {
                self.known_name_stack.push(std::mem::take(&mut self.known_names));
            }
            NodeRef::ObjectField(node) => {
                let field_name = &node.name.value;
                if self.known_names.contains(field_name) {
                    cx.report_error(format!("There can be only one input field named \"{field_name}\"."));
                } else {
                    self.known_names.insert(field_name.clone());
                }
            }
            _ => {}
        }
        VisitAction::Continue
    }

    fn leave(&mut self, node: NodeRef<'a>, _cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::ObjectValue(_) = node {
            self.known_names = self.known_name_stack.pop().unwrap_or_default();
        }
        VisitAction::Continue
    }
}

/// `ProvidedRequiredArgumentsOnDirectivesRule`.
struct ProvidedRequiredArgumentsOnDirectivesRule {
    /// Directive name to its required arguments (name to printed type).
    required_args_map: IndexMap<String, IndexMap<String, String>>,
}

/// `isRequiredArgumentNode(arg)` (and `isRequiredArgument` for the specified directives).
fn is_required_argument_node(arg: &InputValueDefinition) -> bool {
    matches!(arg.ty, Type::NonNull(_)) && arg.default_value.is_none()
}

impl ProvidedRequiredArgumentsOnDirectivesRule {
    fn new(document: &Document) -> Self {
        let required_args = |args: &[InputValueDefinition]| -> IndexMap<String, String> {
            args.iter()
                .filter(|arg| is_required_argument_node(arg))
                .map(|arg| (arg.name.value.clone(), graphql_rs::print(&arg.ty)))
                .collect()
        };
        let mut required_args_map: IndexMap<String, IndexMap<String, String>> = IndexMap::default();
        for directive in graphql_rs::specified_directives() {
            required_args_map.insert(directive.name.value.clone(), required_args(&directive.arguments));
        }
        for def in &document.definitions {
            if let Definition::DirectiveDefinition(def) = def {
                required_args_map.insert(def.name.value.clone(), required_args(&def.arguments));
            }
        }
        ProvidedRequiredArgumentsOnDirectivesRule { required_args_map }
    }
}

impl<'a> Visitor<'a, SdlValidationContext> for ProvidedRequiredArgumentsOnDirectivesRule {
    fn leave(&mut self, node: NodeRef<'a>, cx: &mut SdlValidationContext, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::Directive(directive_node) = node else {
            return VisitAction::Continue;
        };
        let directive_name = &directive_node.name.value;
        if let Some(required_args) = self.required_args_map.get(directive_name) {
            let arg_node_map: IndexSet<&str> = directive_node
                .arguments
                .iter()
                .map(|arg| arg.name.value.as_str())
                .collect();
            for (arg_name, arg_type) in required_args {
                if !arg_node_map.contains(arg_name.as_str()) {
                    cx.report_error(format!(
                        "Argument \"@{directive_name}({arg_name}:)\" of type \"{arg_type}\" is required, but it was not provided."
                    ));
                }
            }
        }
        VisitAction::Continue
    }
}

// ---------------------------------------------------------------------------------------------
// graphql-js `jsutils`: `suggestionList`, `didYouMean`, `naturalCompare`.
// ---------------------------------------------------------------------------------------------

/// `suggestionList(input, options)`: options similar to `input`, closest first.
/// NOTE(port): inputs are GraphQL names (ASCII), so UTF-16 code units are bytes here; the code
/// still works per UTF-16 unit for other strings.
fn suggestion_list(input: &str, options: &[&str]) -> Vec<String> {
    let mut options_by_distance: IndexMap<&str, usize> = IndexMap::default();
    let mut lexical_distance = LexicalDistance::new(input);
    let input_length = input.encode_utf16().count();
    let threshold = (input_length as f64 * 0.4).floor() as usize + 1;
    for option in options {
        if let Some(distance) = lexical_distance.measure(option, threshold) {
            options_by_distance.insert(option, distance);
        }
    }
    // `Object.keys(optionsByDistance)`: GraphQL names are never integer-like keys.
    let mut keys: Vec<&str> = options_by_distance.keys().copied().collect();
    keys.sort_by(|a, b| {
        let distance_diff = options_by_distance[a] as i64 - options_by_distance[b] as i64;
        if distance_diff != 0 {
            distance_diff.cmp(&0)
        } else {
            natural_compare(a, b)
        }
    });
    keys.into_iter().map(str::to_string).collect()
}

/// JS `String.prototype.toLowerCase` as UTF-16 code units.
fn to_lower_case_units(s: &str) -> Vec<u16> {
    s.to_lowercase().encode_utf16().collect()
}

struct LexicalDistance {
    input: String,
    input_lower_case: String,
    input_array: Vec<u16>,
    rows: [Vec<usize>; 3],
}

impl LexicalDistance {
    fn new(input: &str) -> Self {
        let input_lower_case = input.to_lowercase();
        let len = input.encode_utf16().count() + 1;
        LexicalDistance {
            input: input.to_string(),
            input_array: to_lower_case_units(input),
            input_lower_case,
            rows: [vec![0; len], vec![0; len], vec![0; len]],
        }
    }

    fn measure(&mut self, option: &str, threshold: usize) -> Option<usize> {
        if self.input == option {
            return Some(0);
        }

        let option_lower_case = option.to_lowercase();
        if self.input_lower_case == option_lower_case {
            return Some(1);
        }

        let option_array: Vec<u16> = option_lower_case.encode_utf16().collect();
        let mut a: &[u16] = &option_array;
        let mut b: &[u16] = &self.input_array;
        if a.len() < b.len() {
            std::mem::swap(&mut a, &mut b);
        }
        let a_length = a.len();
        let b_length = b.len();

        if a_length - b_length > threshold {
            return None;
        }

        // NOTE(port): JS arrays grow on write; the rows are sized for the longer string.
        let rows = &mut self.rows;
        for row in rows.iter_mut() {
            if row.len() < b_length + 1 {
                row.resize(b_length + 1, 0);
            }
        }
        for (j, cell) in rows[0].iter_mut().enumerate().take(b_length + 1) {
            *cell = j;
        }

        for i in 1..=a_length {
            let up = (i - 1) % 3;
            let cur = i % 3;
            rows[cur][0] = i;
            let mut smallest_cell = i;
            for j in 1..=b_length {
                let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };

                let mut current_cell = (rows[up][j] + 1).min(rows[cur][j - 1] + 1).min(rows[up][j - 1] + cost);

                if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                    // transposition
                    let double_diagonal_cell = rows[(i - 2) % 3][j - 2];
                    current_cell = current_cell.min(double_diagonal_cell + 1);
                }

                if current_cell < smallest_cell {
                    smallest_cell = current_cell;
                }

                rows[cur][j] = current_cell;
            }

            // Early exit, since distance can't go smaller than smallest element of the previous row.
            if smallest_cell > threshold {
                return None;
            }
        }

        let distance = rows[a_length % 3][b_length];
        if distance <= threshold {
            Some(distance)
        } else {
            None
        }
    }
}

/// `naturalCompare(aStr, bStr)`.
fn natural_compare(a_str: &str, b_str: &str) -> std::cmp::Ordering {
    const DIGIT_0: u32 = 48;
    let is_digit = |code: Option<u16>| code.is_some_and(|c| (48..=57).contains(&c));
    let a: Vec<u16> = a_str.encode_utf16().collect();
    let b: Vec<u16> = b_str.encode_utf16().collect();
    let mut a_index = 0;
    let mut b_index = 0;

    while a_index < a.len() && b_index < b.len() {
        let mut a_char = a.get(a_index).copied();
        let mut b_char = b.get(b_index).copied();

        if is_digit(a_char) && is_digit(b_char) {
            let mut a_num: f64 = 0.0;
            loop {
                a_index += 1;
                a_num = a_num * 10.0 + (a_char.unwrap_or(0) as u32 - DIGIT_0) as f64;
                a_char = a.get(a_index).copied();
                if !(is_digit(a_char) && a_num > 0.0) {
                    break;
                }
            }

            let mut b_num: f64 = 0.0;
            loop {
                b_index += 1;
                b_num = b_num * 10.0 + (b_char.unwrap_or(0) as u32 - DIGIT_0) as f64;
                b_char = b.get(b_index).copied();
                if !(is_digit(b_char) && b_num > 0.0) {
                    break;
                }
            }

            if a_num < b_num {
                return std::cmp::Ordering::Less;
            }

            if a_num > b_num {
                return std::cmp::Ordering::Greater;
            }
        } else {
            if a_char < b_char {
                return std::cmp::Ordering::Less;
            }
            if a_char > b_char {
                return std::cmp::Ordering::Greater;
            }
            a_index += 1;
            b_index += 1;
        }
    }
    a.len().cmp(&b.len())
}

/// `didYouMean(suggestions)` (without a sub-message).
fn did_you_mean(suggestions: &[String]) -> String {
    const MAX_SUGGESTIONS: usize = 5;
    if suggestions.is_empty() {
        return String::new();
    }
    let message = " Did you mean ".to_string();
    let items: Vec<String> = suggestions
        .iter()
        .take(MAX_SUGGESTIONS)
        .map(|x| format!("\"{x}\""))
        .collect();
    message + &or_list(&items) + "?"
}

/// `orList(items)` (`formatList('or', items)`).
fn or_list(items: &[String]) -> String {
    match items.len() {
        1 => return items[0].clone(),
        2 => return format!("{} or {}", items[0], items[1]),
        _ => {}
    }
    let all_but_last = &items[..items.len() - 1];
    let last_item = &items[items.len() - 1];
    format!("{}, or {}", all_but_last.join(", "), last_item)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reachable(sdl: &str) -> IndexSet<String> {
        get_reachable_types(&graphql_rs::parse(sdl).unwrap()).unwrap()
    }

    fn sdl_error(sdl: &str) -> String {
        match get_reachable_types(&graphql_rs::parse(sdl).unwrap()) {
            Err(e) => e.to_js_string(),
            Ok(set) => panic!("expected an error, got {set:?}"),
        }
    }

    fn names(items: &[&str]) -> IndexSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn includes_root_types() {
        assert_eq!(reachable("type Query { hello: String }"), names(&["Query"]));
        assert_eq!(reachable("type Mutation { hello: String }"), names(&["Mutation"]));
        assert_eq!(
            reachable("type Subscription { hello: String }"),
            names(&["Subscription"])
        );
    }

    #[test]
    fn excludes_unused_root_type() {
        let set = reachable(
            "type Query { hello: String }
             type Mutation { hello: String }
             schema { query: Query }",
        );
        assert_eq!(set, names(&["Query"]));
    }

    #[test]
    fn extend_schema_without_schema_definition_is_overridden_by_named_types() {
        // `config.astNode == null`: the type named `Query` wins over `extend schema`.
        let set = reachable(
            "type Query { a: String }
             type Root { b: Foo }
             type Foo { c: String }
             extend schema { query: Root }",
        );
        assert_eq!(set, names(&["Query"]));
        let set = reachable(
            "type Root { b: Foo }
             type Foo { c: String }
             extend schema { query: Root }",
        );
        assert_eq!(set, names(&["Root", "Foo"]));
    }

    #[test]
    fn walks_types() {
        assert_eq!(
            reachable("type Query { hello: Hello } type Hello { world: String }"),
            names(&["Query", "Hello"])
        );
        assert_eq!(
            reachable("type Query { hello: Hello } scalar Hello"),
            names(&["Query", "Hello"])
        );
        assert_eq!(
            reachable("type Query { hello(input: Hello): String } input Hello { world: String }"),
            names(&["Query", "Hello"])
        );
        assert_eq!(
            reachable("type Query { hello: Hello } enum Hello { WORLD }"),
            names(&["Query", "Hello"])
        );
        assert_eq!(
            reachable(
                "type Query { hello: Hello } union Hello = World union Gang = World type World { world: String }"
            ),
            names(&["Query", "Hello", "World"])
        );
        assert_eq!(
            reachable(
                "type Query { hello: Hello } interface Hello { world: String } type World implements Hello { world: String }"
            ),
            names(&["Query", "Hello", "World"])
        );
        assert_eq!(
            reachable(
                "input Hello { world: World } input World { world: String } type Query { hello(world: Hello): String }"
            ),
            names(&["Query", "Hello", "World"])
        );
    }

    #[test]
    fn interface_field_arguments_are_not_visited() {
        let set = reachable(
            "type Query { hello: Hello }
             interface Hello { world(a: In): String }
             input In { x: String }",
        );
        assert_eq!(set, names(&["Query", "Hello"]));
    }

    #[test]
    fn extensions_contribute_fields() {
        let set = reachable(
            "type Query { a: String }
             extend type Query { b: B }
             type B { c: String }
             union U = B
             extend union U = C
             type C { d: String }",
        );
        assert_eq!(set, names(&["Query", "B"]));
    }

    #[test]
    fn includes_types_referenced_by_directive_arguments() {
        let set = reachable(
            "directive @meta(options: MetaOptions) on FIELD_DEFINITION
             input MetaOptions { visibility: Visibility }
             enum Visibility { PUBLIC PRIVATE }
             type Query { hello: String }",
        );
        assert_eq!(set, names(&["Query", "MetaOptions", "Visibility"]));
    }

    #[test]
    fn introspection_types_are_walked() {
        let set = reachable("type Query { t: __Type }");
        assert_eq!(
            set,
            names(&[
                "Query",
                "__Type",
                "__TypeKind",
                "__Field",
                "__EnumValue",
                "__InputValue"
            ])
        );
    }

    #[test]
    fn add_directive_on_unused_types() {
        let document = graphql_rs::parse(
            "type Query { hello: String }
             type Mutation @inaccessible { hello: String }
             type Other { hello: String }
             schema { query: Query }
             directive @inaccessible on OBJECT",
        )
        .unwrap();
        let excluded = get_reachable_types(&document).unwrap();
        let result = add_directive_on_types(AddDirectiveOnTypesArgs {
            document_node: &document,
            excluded_type_names: &excluded,
            directive_name: "inaccessible",
        });
        assert_eq!(
            graphql_rs::print(&result),
            "type Query {\n  hello: String\n}\n\ntype Mutation @inaccessible {\n  hello: String\n}\n\ntype Other @inaccessible {\n  hello: String\n}\n\nschema {\n  query: Query\n}\n\ndirective @inaccessible on OBJECT"
        );
    }

    // Expected messages generated with graphql-js 17.0.2 (`String(e)` of `buildASTSchema(parse(sdl))`).
    #[test]
    fn sdl_validation_errors() {
        let cases: [(&str, &str); 10] = [
            (
                "type Query { a: Strin b: Foo } type Foo { x: Int }",
                "Error: Unknown type \"Strin\". Did you mean \"String\"?",
            ),
            (
                "type Query { a: String } type Query { b: String }",
                "Error: There can be only one type named \"Query\".",
            ),
            (
                "type Query { a: String @foo @deprecated @deprecated }",
                "Error: The directive \"@deprecated\" can only be used once at this location.\n\nUnknown directive \"@foo\".",
            ),
            (
                "type Query { a: String } extend type Querry { b: String } extend scalar Query @specifiedBy(url: \"x\")",
                "Error: Cannot extend type \"Querry\" because it is not defined. Did you mean \"Query\"?\n\nCannot extend non-scalar type \"Query\".",
            ),
            (
                "directive @d(req: Int!, opt: Int) on OBJECT type Query @d(op: 1) { a: String }",
                "Error: Unknown argument \"op\" on directive \"@d\". Did you mean \"opt\"?\n\nArgument \"@d(req:)\" of type \"Int!\" is required, but it was not provided.",
            ),
            (
                "directive @d on FIELD_DEFINITION type Query @d { a(x: Int, x: Int): String @d }",
                "Error: Argument \"Query.a(x:)\" can only be defined once.\n\nDirective \"@d\" may not be used on OBJECT.",
            ),
            (
                "schema { query: Query query: Query } schema { query: Query } type Query { a: String }",
                "Error: There can be only one query type in schema.\n\nMust provide only one schema definition.\n\nThere can be only one query type in schema.",
            ),
            (
                "type Query { a: String a: Int } enum E { A A } input I { x: Int x: Int } directive @x(a: Int a: Int) on OBJECT directive @x on OBJECT",
                "Error: Field \"Query.a\" can only be defined once.\n\nEnum value \"E.A\" can only be defined once.\n\nField \"I.x\" can only be defined once.\n\nArgument \"@x(a:)\" can only be defined once.\n\nThere can be only one directive named \"@x\".",
            ),
            (
                "type Query { a(x: I = {a: 1, a: 2}): String @include(if: true, if: false) @skip } input I { a: Int } scalar S @specifiedBy",
                "Error: There can be only one input field named \"a\".\n\nDirective \"@include\" may not be used on FIELD_DEFINITION.\n\nThere can be only one argument named \"if\".\n\nDirective \"@skip\" may not be used on FIELD_DEFINITION.\n\nArgument \"@skip(if:)\" of type \"Boolean!\" is required, but it was not provided.\n\nArgument \"@specifiedBy(url:)\" of type \"String!\" is required, but it was not provided.",
            ),
            (
                "type Query { a: __Typ }",
                "Error: Unknown type \"__Typ\". Did you mean \"__Type\"?",
            ),
        ];
        for (sdl, expected) in cases {
            assert_eq!(sdl_error(sdl), expected, "{sdl}");
        }
    }

    #[test]
    fn suggestion_list_and_did_you_mean() {
        assert_eq!(
            suggestion_list("Strin", &["String", "Int", "Stri"]),
            vec!["Stri", "String"]
        );
        assert_eq!(
            suggestion_list("abc", &["ABC", "abc", "acb"]),
            vec!["abc", "ABC", "acb"]
        );
        assert_eq!(did_you_mean(&[]), "");
        assert_eq!(
            did_you_mean(&["a", "b", "c", "d", "e", "f"].map(String::from)),
            " Did you mean \"a\", \"b\", \"c\", \"d\", or \"e\"?"
        );
        assert_eq!(
            did_you_mean(&["a", "b"].map(String::from)),
            " Did you mean \"a\" or \"b\"?"
        );
        assert_eq!(natural_compare("a10", "a9"), std::cmp::Ordering::Greater);
        assert_eq!(natural_compare("a01", "a1"), std::cmp::Ordering::Less);
    }
}
