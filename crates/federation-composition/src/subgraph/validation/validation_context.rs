//! Port of `src/subgraph/validation/validation-context.ts`.
//!
//! Both TS contexts are objects of closures over locals; here they are structs whose fields are
//! those locals and whose methods are the closures. They are the `C` (`cx`) of
//! `graphql_core::Visitor<'a, C>` for the validation rules, so a rule receives the context in
//! every callback instead of capturing it.

use std::rc::Rc;
use std::sync::LazyLock;

use crate::utils::collections::{IndexMap, IndexSet};
use graphql_core::{
    parse, Ancestor, Definition, DirectiveDefinition, Document, GraphQLError, Kind, Name, TypeDefinition,
};

use crate::graphql::type_node_info::{HasTypeNodeInfo, TypeNodeInfo};
use crate::specifications::federation::{DetectedFederation, FederationVersion, SpecSchema};
use crate::specifications::link::LinkImport;
use crate::subgraph::helpers::ObjectOrInterface;
use crate::subgraph::state::{SubgraphStateBuilder, SubgraphType, TypeKind};
use crate::utils::js::{JsError, JsResult};
use crate::utils::state::strip_type_modifiers;

/// `linkSpec = parse(linkSpecSdl)`.
static LINK_SPEC: LazyLock<Document> =
    LazyLock::new(|| parse(crate::specifications::link::SDL).expect("valid link spec SDL"));

/// `linkSpecDirectives`.
static LINK_SPEC_DIRECTIVES: LazyLock<Vec<DirectiveDefinition>> = LazyLock::new(|| {
    LINK_SPEC
        .definitions
        .iter()
        .filter_map(|d| match d {
            Definition::DirectiveDefinition(d) => Some(d.clone()),
            _ => None,
        })
        .collect()
});

/// `linkSpecTypes = linkSpec.definitions.filter(isTypeDefinitionNode)`.
static LINK_SPEC_TYPES: LazyLock<Vec<TypeDefinition>> = LazyLock::new(|| {
    LINK_SPEC
        .definitions
        .iter()
        .filter_map(|d| TypeDefinition::try_from(d.clone()).ok())
        .collect()
});

/// The subgraph passed to `createSubgraphValidationContext`: `{ name, id, typeDefs }`.
#[derive(Clone, Copy, Debug)]
pub struct SubgraphRef<'a> {
    pub name: &'a str,
    // TS: the subgraph's `id`; only `getSubgraphId` reads it.
    #[allow(dead_code)]
    pub id: &'a str,
    pub type_defs: &'a Document,
}

/// An entry of `typeDefinitionMap`: `Pick<TypeDefinitionNode | TypeExtensionNode, "name" | "kind">`.
#[derive(Clone, Copy, Debug)]
pub struct KnownTypeDefinition<'a> {
    // TS: part of `Pick<TypeDefinitionNode, "name" | "kind">`; nothing in `src/` reads the name.
    #[allow(dead_code)]
    pub name: &'a Name,
    pub kind: Kind,
}

/// `SimpleValidationContext = ReturnType<typeof createSimpleValidationContext>`, the context of
/// the graphql-js-style rules run over the full type definitions.
#[derive(Debug)]
pub struct SimpleValidationContext<'a> {
    type_defs: &'a Document,
    reported_errors: Vec<GraphQLError>,
    directive_definition_map: IndexMap<String, &'a DirectiveDefinition>,
    type_definition_map: IndexMap<String, KnownTypeDefinition<'a>>,
    /// The error a rule threw (see [`SimpleValidationContext::throw`]).
    thrown: Option<JsError>,
}

/// `createSimpleValidationContext(typeDefs, typeNodeInfo)`.
/// NOTE(port): the TS takes the `TypeNodeInfo` but the returned object never uses it, so it is
/// not a parameter here.
pub fn create_simple_validation_context(type_defs: &Document) -> SimpleValidationContext<'_> {
    let mut directive_definition_map: IndexMap<String, &DirectiveDefinition> = IndexMap::default();
    let mut type_definition_map: IndexMap<String, KnownTypeDefinition<'_>> = IndexMap::default();

    for definition in &type_defs.definitions {
        if let Definition::DirectiveDefinition(d) = definition {
            directive_definition_map.insert(d.name.value.clone(), d);
        } else if let Some(name) = definition.name() {
            // TODO: What if we have a type extension (or many) and a type definition with the same name?
            type_definition_map.insert(
                name.value.clone(),
                KnownTypeDefinition {
                    name,
                    kind: definition.kind(),
                },
            );
        }
    }

    SimpleValidationContext {
        type_defs,
        reported_errors: Vec::new(),
        directive_definition_map,
        type_definition_map,
        thrown: None,
    }
}

impl<'a> SimpleValidationContext<'a> {
    /// Port mechanism for a TS `throw` inside a rule's callback, which aborts the `visit` and
    /// `validateSubgraph`: records the first error and returns `VisitAction::Break`. The visit of
    /// the rules stops once an error is recorded, and `validate_subgraph` returns it.
    pub fn throw(&mut self, error: JsError) -> graphql_core::VisitAction {
        if self.thrown.is_none() {
            self.thrown = Some(error);
        }
        graphql_core::VisitAction::Break
    }

    /// Whether a rule threw (see [`Self::throw`]).
    pub fn has_thrown(&self) -> bool {
        self.thrown.is_some()
    }

    /// Takes the error recorded by [`Self::throw`].
    pub fn take_thrown(&mut self) -> Option<JsError> {
        self.thrown.take()
    }

    pub fn get_document(&self) -> &'a Document {
        self.type_defs
    }

    pub fn get_known_directive_definition(&self, name: &str) -> Option<&'a DirectiveDefinition> {
        self.directive_definition_map.get(name).copied()
    }

    pub fn get_known_type_definition(&self, name: &str) -> Option<KnownTypeDefinition<'a>> {
        self.type_definition_map.get(name).copied()
    }

    /// `getSchemaCoordinate(ancestors)`.
    pub fn get_schema_coordinate(&self, ancestors: &[Ancestor<'a>]) -> String {
        let mut coordinate = String::new();
        for ancestor in ancestors {
            // `"kind" in ancestor`: lists have no kind.
            let Ancestor::Node(ancestor) = ancestor else {
                continue;
            };
            if ancestor.kind() == Kind::Document {
                continue;
            }
            let name = match ancestor.kind() {
                Kind::SchemaDefinition | Kind::SchemaExtension => "schema",
                _ => ancestor.name().map(|n| n.value.as_str()).unwrap_or(""),
            };
            if !coordinate.is_empty() {
                coordinate = coordinate + "." + name;
            } else {
                coordinate = name.to_string();
            }
        }

        coordinate
    }

    pub fn report_error(&mut self, error: GraphQLError) {
        self.reported_errors.push(error);
    }

    pub fn collect_reported_errors(&mut self) -> Vec<GraphQLError> {
        std::mem::take(&mut self.reported_errors)
    }
}

/// The `reason` of `markAsUsed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkAsUsedReason {
    /// `"fields"`.
    Fields,
    /// `"@extends"`.
    Extends,
    /// `"references @shareable"`.
    // TS: a member of the `reason` union that nothing in `src/` produces.
    #[allow(dead_code)]
    ReferencesShareable,
}

/// An entry of `getFieldsToMarkAsShareable()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldToMarkAsShareable {
    pub type_name: String,
    pub field_name: String,
}

/// `SubgraphValidationContext = ReturnType<typeof createSubgraphValidationContext>`.
///
/// `'a` is the lifetime of the visited subgraph document; the context borrows the state
/// builder for the same duration. `type_node_info` is maintained by
/// `visit_with_type_node_info` during the federation-rules visit.
pub struct SubgraphValidationContext<'a> {
    /// `stateBuilder`.
    pub state_builder: &'a mut SubgraphStateBuilder,
    /// `federationImports`.
    // TS: the context's `federationImports`; nothing in `src/` reads it.
    #[allow(dead_code)]
    pub federation_imports: Vec<LinkImport>,
    /// `typeNodeInfo`.
    pub type_node_info: TypeNodeInfo<'a>,
    subgraph: SubgraphRef<'a>,
    version: FederationVersion,
    imports: Vec<LinkImport>,
    /// Directives and types available to a subgraph.
    /// NOTE(port): `getAvailableFederationTypeAndDirectiveDefinitions` renames these nodes in
    /// place (aliases), and later readers see the renamed nodes; keep that.
    available_spec: SpecSchema,
    /// Directives and types available in the spec (some parts may not be available to a subgraph).
    known_spec: &'static SpecSchema,
    /// Object and interface types defined in a subgraph (merged definitions and extensions).
    known_object_and_interface_subgraph_entities: IndexMap<String, Rc<ObjectOrInterface>>,
    known_union_types_members: IndexMap<String, IndexSet<String>>,
    /// `Map<string, TypeKind.OBJECT | TypeKind.INTERFACE>`.
    typename_to_kind: IndexMap<String, TypeKind>,
    /// Directive definitions defined in a subgraph.
    known_subgraph_directive_definitions: IndexMap<String, &'a DirectiveDefinition>,
    leaf_type_names: IndexSet<String>,
    reported_errors: Vec<GraphQLError>,
    marked_as_external: IndexSet<String>,
    marked_as_used: IndexSet<String>,
    marked_as_key_field: IndexSet<String>,
    overwritten_federation_definition_names: IndexSet<String>,
    /// Normalized directive name (e.g. `key`) to the names it is available under
    /// (`key`, `federation__key`, an import alias).
    directive_alternative_names_map: IndexMap<String, IndexSet<String>>,
    /// Same as `directive_alternative_names_map`, for types.
    type_alternative_names_map: IndexMap<String, IndexSet<String>>,
    /// The first error thrown by a visitor callback (see [`Self::throw`]).
    thrown: Option<JsError>,
}

/// `createSubgraphValidationContext(subgraph, federation, typeNodeInfo, stateBuilder)`.
/// The `TypeNodeInfo` is created inside and owned by the context.
pub fn create_subgraph_validation_context<'a>(
    subgraph: SubgraphRef<'a>,
    federation: &DetectedFederation,
    state_builder: &'a mut SubgraphStateBuilder,
) -> SubgraphValidationContext<'a> {
    let version = federation.version;
    let imports = &federation.imports;

    // Directives and types available to a subgraph
    let available_spec = crate::specifications::federation::create_spec_schema(version.as_str(), Some(imports))
        .expect("createSpecSchema accepts every FederationVersion");
    // Directives and types available in the spec (some parts may not be available to a subgraph)
    // PERF: shared, read-only copy (see `known_spec_schema`).
    let known_spec = crate::specifications::federation::known_spec_schema(version.as_str())
        .expect("createSpecSchema accepts every FederationVersion");
    // Object and interface types defined in a subgraph
    let mut known_object_and_interface_subgraph_entities: IndexMap<String, ObjectOrInterface> = IndexMap::default();
    let mut known_union_types_members: IndexMap<String, IndexSet<String>> = IndexMap::default();

    let mut typename_to_kind: IndexMap<String, TypeKind> = IndexMap::default();

    for def in &subgraph.type_defs.definitions {
        if is_union_node(def) {
            let (name, types) = match def {
                Definition::UnionTypeDefinition(d) => (&d.name.value, &d.types),
                Definition::UnionTypeExtension(d) => (&d.name.value, &d.types),
                _ => unreachable!("is_union_node"),
            };
            match known_union_types_members.get_mut(name) {
                None => {
                    known_union_types_members
                        .insert(name.clone(), types.iter().map(|t| t.name.value.clone()).collect());
                }
                Some(found) => {
                    for named_type in types {
                        found.insert(named_type.name.value.clone());
                    }
                }
            }

            continue;
        }

        if !is_object_or_interface_node(def) {
            continue;
        }

        // `{ ...def }`.
        let copy = match def {
            Definition::ObjectTypeDefinition(d) => ObjectOrInterface {
                kind: Kind::ObjectTypeDefinition,
                description: d.description.clone(),
                name: d.name.clone(),
                interfaces: d.interfaces.clone(),
                directives: d.directives.clone(),
                fields: d.fields.clone(),
            },
            Definition::ObjectTypeExtension(d) => ObjectOrInterface {
                kind: Kind::ObjectTypeExtension,
                description: None,
                name: d.name.clone(),
                interfaces: d.interfaces.clone(),
                directives: d.directives.clone(),
                fields: d.fields.clone(),
            },
            Definition::InterfaceTypeDefinition(d) => ObjectOrInterface {
                kind: Kind::InterfaceTypeDefinition,
                description: d.description.clone(),
                name: d.name.clone(),
                interfaces: d.interfaces.clone(),
                directives: d.directives.clone(),
                fields: d.fields.clone(),
            },
            Definition::InterfaceTypeExtension(d) => ObjectOrInterface {
                kind: Kind::InterfaceTypeExtension,
                description: None,
                name: d.name.clone(),
                interfaces: d.interfaces.clone(),
                directives: d.directives.clone(),
                fields: d.fields.clone(),
            },
            _ => unreachable!("is_object_or_interface_node"),
        };
        let def_kind = copy.kind;

        typename_to_kind.insert(
            copy.name.value.clone(),
            if def_kind == Kind::ObjectTypeDefinition || def_kind == Kind::ObjectTypeExtension {
                TypeKind::Object
            } else {
                TypeKind::Interface
            },
        );

        let Some(found) = known_object_and_interface_subgraph_entities.get_mut(&copy.name.value) else {
            known_object_and_interface_subgraph_entities.insert(copy.name.value.clone(), copy);
            continue;
        };

        found.fields.extend(copy.fields);
        found.interfaces.extend(copy.interfaces);
        found.directives.extend(copy.directives);

        if def_kind == Kind::ObjectTypeDefinition && found.kind == Kind::ObjectTypeExtension {
            found.kind = Kind::ObjectTypeDefinition;
        } else if def_kind == Kind::InterfaceTypeDefinition && found.kind == Kind::InterfaceTypeExtension {
            found.kind = Kind::InterfaceTypeDefinition;
        }
    }

    // Directive definitions defined in a subgraph
    let mut known_subgraph_directive_definitions: IndexMap<String, &'a DirectiveDefinition> = IndexMap::default();
    for def in &subgraph.type_defs.definitions {
        if let Definition::DirectiveDefinition(d) = def {
            known_subgraph_directive_definitions.insert(d.name.value.clone(), d);
        }
    }

    let mut leaf_type_names: IndexSet<String> = graphql_core::specified_scalar_types()
        .iter()
        .map(|t| t.name.to_string())
        .collect();

    for def in &subgraph.type_defs.definitions {
        match def {
            Definition::ScalarTypeDefinition(d) => {
                leaf_type_names.insert(d.name.value.clone());
            }
            Definition::ScalarTypeExtension(d) => {
                leaf_type_names.insert(d.name.value.clone());
            }
            Definition::EnumTypeDefinition(d) => {
                leaf_type_names.insert(d.name.value.clone());
            }
            Definition::EnumTypeExtension(d) => {
                leaf_type_names.insert(d.name.value.clone());
            }
            _ => {}
        }
    }

    let mut overwritten_federation_definition_names: IndexSet<String> = IndexSet::default();

    // Build a map where the original name points to alternative names.
    //
    // For example, the `key` is the original name of @key directive
    // but it can be available under different names:
    //  - federation__key might be available (federation__* directives are available by default in v2)
    //  - primaryKey when -> @link(url: "https://specs.apollo.dev/federation/v2.0" import: [{ name: "@key", as: "@primaryKey" }])
    //
    // Whenever we want to detect or get the @key directive, we use the "normalized" form instead of looking for @federation__key or @key or @whatever.
    // That's why we need this map, to take the normalized name and find the available one.
    let mut directive_alternative_names_map: IndexMap<String, IndexSet<String>> = IndexMap::default();
    for spec_directive in &available_spec.directives {
        let spec_name = &spec_directive.name.value;
        if let Some(normalized_name) = spec_name.strip_prefix("federation__") {
            // `name.replace("federation__", "")` on a prefixed name removes the prefix.
            if !directive_alternative_names_map.contains_key(normalized_name) {
                directive_alternative_names_map
                    .insert(normalized_name.to_string(), IndexSet::from_iter([spec_name.clone()]));
            }
        } else {
            // TODO: get rid of `@` prefix in directive names of link.imports
            let alias = imports
                .iter()
                .find(|i| strip_at(&i.name) == spec_name.as_str())
                .and_then(|i| i.alias.as_deref());

            let set_of_names = directive_alternative_names_map.entry(spec_name.clone()).or_default();

            set_of_names.insert(match alias {
                Some(alias) if !alias.is_empty() => strip_at(alias).to_string(),
                _ => spec_name.clone(),
            });
            set_of_names.insert(format!("federation__{spec_name}"));
        }
    }

    // Same story as in `directiveAlternativeNamesMap`.
    let mut type_alternative_names_map: IndexMap<String, IndexSet<String>> = IndexMap::default();
    for spec_type in &available_spec.types {
        let spec_name = &spec_type.name().value;
        if let Some(normalized_name) = spec_name.strip_prefix("federation__") {
            if !type_alternative_names_map.contains_key(normalized_name) {
                type_alternative_names_map
                    .insert(normalized_name.to_string(), IndexSet::from_iter([spec_name.clone()]));
            }
        } else {
            let alias = imports
                .iter()
                .find(|i| &i.name == spec_name)
                .and_then(|i| i.alias.as_deref());

            let set_of_names = type_alternative_names_map.entry(spec_name.clone()).or_default();

            set_of_names.insert(match alias {
                Some(alias) if !alias.is_empty() => alias.to_string(),
                _ => spec_name.clone(),
            });
            set_of_names.insert(format!("federation__{spec_name}"));
        }
    }

    let imported_types_set: IndexSet<&str> = available_spec.types.iter().map(|t| t.name().value.as_str()).collect();
    if !imported_types_set.is_empty() {
        for def in &subgraph.type_defs.definitions {
            if let Some(name) = def.name() {
                if imported_types_set.contains(name.value.as_str()) {
                    overwritten_federation_definition_names.insert(name.value.clone());
                }
            }
        }
    }

    SubgraphValidationContext {
        state_builder,
        federation_imports: imports.clone(),
        type_node_info: TypeNodeInfo::new(),
        subgraph,
        version,
        imports: imports.clone(),
        available_spec,
        known_spec,
        known_object_and_interface_subgraph_entities: known_object_and_interface_subgraph_entities
            .into_iter()
            .map(|(k, v)| (k, Rc::new(v)))
            .collect(),
        known_union_types_members,
        typename_to_kind,
        known_subgraph_directive_definitions,
        leaf_type_names,
        reported_errors: Vec::new(),
        marked_as_external: IndexSet::default(),
        marked_as_used: IndexSet::default(),
        marked_as_key_field: IndexSet::default(),
        overwritten_federation_definition_names,
        directive_alternative_names_map,
        type_alternative_names_map,
        thrown: None,
    }
}

/// `(t.name as any).value = ...` for a spec type definition.
fn type_definition_name_mut(t: &mut TypeDefinition) -> &mut Name {
    match t {
        TypeDefinition::ScalarTypeDefinition(d) => &mut d.name,
        TypeDefinition::ObjectTypeDefinition(d) => &mut d.name,
        TypeDefinition::InterfaceTypeDefinition(d) => &mut d.name,
        TypeDefinition::UnionTypeDefinition(d) => &mut d.name,
        TypeDefinition::EnumTypeDefinition(d) => &mut d.name,
        TypeDefinition::InputObjectTypeDefinition(d) => &mut d.name,
    }
}

/// `name.replace(/^@/, "")`.
fn strip_at(name: &str) -> &str {
    name.strip_prefix('@').unwrap_or(name)
}

fn is_object_or_interface_node(node: &Definition) -> bool {
    matches!(
        node,
        Definition::ObjectTypeDefinition(_)
            | Definition::ObjectTypeExtension(_)
            | Definition::InterfaceTypeDefinition(_)
            | Definition::InterfaceTypeExtension(_)
    )
}

fn is_union_node(node: &Definition) -> bool {
    matches!(
        node,
        Definition::UnionTypeExtension(_) | Definition::UnionTypeDefinition(_)
    )
}

impl<'a> SubgraphValidationContext<'a> {
    pub fn is_link_spec_directive(&self, name: &str) -> bool {
        LINK_SPEC_DIRECTIVES.iter().any(|d| d.name.value == name)
    }

    pub fn is_link_spec_type(&self, name: &str) -> bool {
        LINK_SPEC_TYPES.iter().any(|t| t.name().value == name)
    }

    /// Check if a type is available to the subgraph (either imported directly or available
    /// out of the box).
    pub fn is_available_federation_type(&self, name: &str) -> bool {
        self.type_alternative_names_map
            .get(name)
            .is_some_and(|names| names.contains(name))
    }

    /// Check if a directive is available to the subgraph (either imported directly or
    /// available out of the box). The TS takes the directive node (or anything with a `name`,
    /// as a string or a `NameNode`); this takes that name.
    pub fn is_available_federation_directive(&self, spec_directive_name: &str, directive_name: &str) -> bool {
        self.directive_alternative_names_map
            .get(spec_directive_name)
            .is_some_and(|names| names.contains(directive_name))
    }

    /// `satisfiesVersionRange(range)` for the subgraph's federation version.
    pub fn satisfies_version_range(&self, range: &str) -> bool {
        crate::utils::version::satisfies_version_range(self.version, range)
    }

    /// Directives defined by the spec. These may or may not be available to a subgraph.
    pub fn get_known_federation_directives(&self) -> &[DirectiveDefinition] {
        &self.known_spec.directives
    }

    /// Directives defined by the spec and available to a subgraph.
    pub fn get_available_federation_directives(&self) -> &[DirectiveDefinition] {
        &self.available_spec.directives
    }

    pub fn is_leaf_type(&self, type_name: &str) -> bool {
        self.leaf_type_names.contains(type_name)
    }

    /// Object and interface types defined by a subgraph.
    pub fn get_subgraph_object_or_interface_types(&self) -> &IndexMap<String, Rc<ObjectOrInterface>> {
        &self.known_object_and_interface_subgraph_entities
    }

    /// Union types defined within the subgraph, with their members.
    pub fn get_subgraph_union_types(&self) -> &IndexMap<String, IndexSet<String>> {
        &self.known_union_types_members
    }

    /// Directives defined by a subgraph.
    pub fn get_subgraph_directive_definitions(&self) -> &IndexMap<String, &'a DirectiveDefinition> {
        &self.known_subgraph_directive_definitions
    }

    /// Federation directives and type definitions available to a subgraph, with import aliases
    /// applied. NOTE(port): the TS renames the nodes of `availableSpec` in place; do the same
    /// (hence `&mut self`) and return clones.
    pub fn get_available_federation_type_and_directive_definitions(&mut self) -> Vec<Definition> {
        let mut result: Vec<Definition> = Vec::new();
        for d in self.available_spec.directives.iter_mut() {
            let alias = self
                .imports
                .iter()
                .find(|i| strip_at(&i.name) == d.name.value)
                .and_then(|i| i.alias.as_deref());

            if let Some(alias) = alias.filter(|a| !a.is_empty()) {
                d.name.value = strip_at(alias).to_string();
            }

            result.push(Definition::DirectiveDefinition(d.clone()));
        }
        for t in self.available_spec.types.iter_mut() {
            let alias = self
                .imports
                .iter()
                .find(|i| i.name == t.name().value)
                .and_then(|i| i.alias.as_deref());

            if let Some(alias) = alias.filter(|a| !a.is_empty()) {
                type_definition_name_mut(t).value = alias.to_string();
            }

            result.push(Definition::from(t.clone()));
        }
        result
    }

    pub fn get_document(&self) -> &'a Document {
        self.subgraph.type_defs
    }

    pub fn get_subgraph_name(&self) -> &'a str {
        self.subgraph.name
    }

    // TS: defined here and never called in `src/` either.
    #[allow(dead_code)]
    pub fn get_subgraph_id(&self) -> &'a str {
        self.subgraph.id
    }

    pub fn mark_as_external(&mut self, coordinate: &str) {
        self.marked_as_external.insert(coordinate.to_string());
    }

    pub fn get_fields_to_mark_as_shareable(&self) -> Vec<FieldToMarkAsShareable> {
        let mut shareable_fields: Vec<FieldToMarkAsShareable> = Vec::new();

        for coordinate in &self.marked_as_key_field {
            // `const [typeName, fieldName] = coordinate.split(".")`.
            // NOTE(port): coordinates are always `Type.field`; a missing part would be
            // `undefined` in TS and is `""` here.
            let mut parts = coordinate.split('.');
            let type_name = parts.next().unwrap_or("");
            let field_name = parts.next().unwrap_or("");

            if self.typename_to_kind.get(type_name) == Some(&TypeKind::Object) {
                shareable_fields.push(FieldToMarkAsShareable {
                    type_name: type_name.to_string(),
                    field_name: field_name.to_string(),
                });
            }
        }

        shareable_fields
    }

    /// `markAsUsed(reason, kind, typeName, fieldName)`. `kind` is one of the object/interface
    /// definition/extension kinds. Fallible because the state builder setters are.
    pub fn mark_as_used(
        &mut self,
        reason: MarkAsUsedReason,
        kind: Kind,
        type_name: &str,
        field_name: &str,
    ) -> JsResult<()> {
        if !field_name.starts_with("__") && !type_name.starts_with("__") && reason == MarkAsUsedReason::Fields {
            match kind {
                Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
                    self.state_builder
                        .object_type()
                        .field()
                        .set_used(type_name, field_name)?;
                }
                Kind::InterfaceTypeDefinition | Kind::InterfaceTypeExtension => {
                    self.state_builder
                        .interface_type()
                        .field()
                        .set_used(type_name, field_name)?;
                }
                _ => {}
            }
        }

        self.marked_as_used.insert(format!("{type_name}.{field_name}"));
        Ok(())
    }

    pub fn mark_as_key_field(&mut self, coordinate: &str) {
        self.marked_as_key_field.insert(coordinate.to_string());
    }

    /// Let the system know that there's a correct replacement of a Federation's directive or
    /// scalar. A subgraph can define its own @key directive or any other Federation v2 bit.
    pub fn mark_as_federation_definition_replacement(&mut self, name: &str) {
        self.overwritten_federation_definition_names.insert(name.to_string());
    }

    /// Names of federation directives and scalars the subgraph defined itself.
    pub fn collect_federation_definition_replacements(&self) -> &IndexSet<String> {
        &self.overwritten_federation_definition_names
    }

    pub fn collect_unused_external(&self) -> Vec<String> {
        if self.version == FederationVersion::V1_0 {
            return self
                .marked_as_external
                .iter()
                .filter(|c| !self.marked_as_used.contains(*c) && self.marked_as_key_field.contains(*c))
                .cloned()
                .collect();
        }

        let unused = self
            .marked_as_external
            .iter()
            .filter(|c| !self.marked_as_used.contains(*c));

        let state = &self.state_builder.state;
        unused
            .filter(|coordinate| {
                let mut parts = coordinate.split('.');
                let type_name = parts.next().unwrap_or("");
                let field_name = parts.next().unwrap_or("");

                let Some(type_def) = state.types.get(type_name) else {
                    return true;
                };

                if let SubgraphType::Object(type_def) = type_def {
                    if !self.state_builder.is_interface_object(type_name) {
                        if let Some(field_def) = type_def.fields.get(field_name) {
                            let output_type_name = strip_type_modifiers(&field_def.r#type);
                            let output_type = state.types.get(&output_type_name);

                            if let Some(SubgraphType::Object(output_type)) = output_type {
                                if output_type.shareable {
                                    // If a field is marked as @external but the output type is @shareable, it should not be marked as unused.
                                    return false;
                                }
                            }
                        }

                        for interface_name in &type_def.interfaces {
                            let Some(i_def) = state.types.get(interface_name) else {
                                continue;
                            };

                            if let SubgraphType::Interface(i_def) = i_def {
                                if i_def.fields.contains_key(field_name) {
                                    return false;
                                }
                            }
                        }
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    pub fn report_error(&mut self, error: GraphQLError) {
        self.reported_errors.push(error);
    }

    pub fn collect_reported_errors(&mut self) -> Vec<GraphQLError> {
        std::mem::take(&mut self.reported_errors)
    }

    /// Port mechanism for a TS `throw` inside a visitor callback (which aborts `visit`): records
    /// the first error and returns `VisitAction::Break`. `visit_with_type_node_info` stops the
    /// whole traversal once an error is recorded, and `validate_subgraph` returns it.
    pub fn throw(&mut self, error: JsError) -> graphql_core::VisitAction {
        if self.thrown.is_none() {
            self.thrown = Some(error);
        }
        graphql_core::VisitAction::Break
    }

    /// Takes the error recorded by [`Self::throw`].
    pub fn take_thrown(&mut self) -> Option<JsError> {
        self.thrown.take()
    }
}

impl<'a> HasTypeNodeInfo<'a> for SubgraphValidationContext<'a> {
    fn type_node_info_mut(&mut self) -> &mut TypeNodeInfo<'a> {
        &mut self.type_node_info
    }

    fn has_thrown(&self) -> bool {
        self.thrown.is_some()
    }
}

/// A federation validation rule: TS `(context: SubgraphValidationContext) => ASTVisitor`.
///
/// Each rule is a struct holding the locals of the TS factory, built by `new` (the factory body
/// before `return { ... }`, which may already report errors), and a
/// `graphql_core::Visitor<'a, SubgraphValidationContext<'a>>` whose `enter`/`leave` are the
/// returned visitor object. `NAME` is the TS function name, matched against
/// `__internal.disableValidationRules`.
pub trait SubgraphRule<'a>: graphql_core::Visitor<'a, SubgraphValidationContext<'a>> + Sized + 'a {
    const NAME: &'static str;
    fn new(context: &mut SubgraphValidationContext<'a>) -> Self;
}

/// A graphql-js-style rule run with the [`SimpleValidationContext`]; see [`SubgraphRule`].
/// `new` is the TS rule function and returns its visitor object: `Self` for a rule whose
/// visitor keeps nothing borrowed from the document, a separate type for one that does (so
/// the rule type itself implements `SimpleRule<'a>` for every `'a`).
pub trait SimpleRule<'a> {
    const NAME: &'static str;
    type Visitor: graphql_core::Visitor<'a, SimpleValidationContext<'a>> + 'a;
    fn new(context: &mut SimpleValidationContext<'a>) -> Self::Visitor;
}
