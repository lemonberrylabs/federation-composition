//! Port of `src/subgraph/validation/validation-context.ts`.
//!
//! Both TS contexts are objects of closures over locals; here they are structs whose fields are
//! those locals and whose methods are the closures. They are the `C` (`cx`) of
//! `graphql_js::Visitor<'a, C>` for the validation rules, so a rule receives the context in
//! every callback instead of capturing it.

use std::rc::Rc;
use std::sync::LazyLock;

use graphql_js::{
    parse, Ancestor, Definition, DirectiveDefinition, Document, GraphQLError, Kind, Name, TypeDefinition,
};
use indexmap::{IndexMap, IndexSet};

use crate::graphql::type_node_info::{HasTypeNodeInfo, TypeNodeInfo};
use crate::specifications::federation::{DetectedFederation, FederationVersion, SpecSchema};
use crate::specifications::link::LinkImport;
use crate::subgraph::helpers::ObjectOrInterface;
use crate::subgraph::state::{SubgraphStateBuilder, TypeKind};
use crate::utils::js::{JsError, JsResult};

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
    pub id: &'a str,
    pub type_defs: &'a Document,
}

/// An entry of `typeDefinitionMap`: `Pick<TypeDefinitionNode | TypeExtensionNode, "name" | "kind">`.
#[derive(Clone, Copy, Debug)]
pub struct KnownTypeDefinition<'a> {
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
}

/// `createSimpleValidationContext(typeDefs, typeNodeInfo)`.
/// NOTE(port): the TS takes the `TypeNodeInfo` but the returned object never uses it, so it is
/// not a parameter here.
pub fn create_simple_validation_context(type_defs: &Document) -> SimpleValidationContext<'_> {
    todo!()
}

impl<'a> SimpleValidationContext<'a> {
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
        todo!()
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
    known_spec: SpecSchema,
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
    todo!()
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
    matches!(node, Definition::UnionTypeExtension(_) | Definition::UnionTypeDefinition(_))
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
        todo!()
    }

    pub fn get_document(&self) -> &'a Document {
        self.subgraph.type_defs
    }

    pub fn get_subgraph_name(&self) -> &'a str {
        self.subgraph.name
    }

    pub fn get_subgraph_id(&self) -> &'a str {
        self.subgraph.id
    }

    pub fn mark_as_external(&mut self, coordinate: &str) {
        self.marked_as_external.insert(coordinate.to_string());
    }

    pub fn get_fields_to_mark_as_shareable(&self) -> Vec<FieldToMarkAsShareable> {
        todo!()
    }

    /// `markAsUsed(reason, kind, typeName, fieldName)`. `kind` is one of the object/interface
    /// definition/extension kinds. Fallible because the state builder setters are.
    pub fn mark_as_used(&mut self, reason: MarkAsUsedReason, kind: Kind, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
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
        todo!()
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
    pub fn throw(&mut self, error: JsError) -> graphql_js::VisitAction {
        if self.thrown.is_none() {
            self.thrown = Some(error);
        }
        graphql_js::VisitAction::Break
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
/// `graphql_js::Visitor<'a, SubgraphValidationContext<'a>>` whose `enter`/`leave` are the
/// returned visitor object. `NAME` is the TS function name, matched against
/// `__internal.disableValidationRules`.
pub trait SubgraphRule<'a>: graphql_js::Visitor<'a, SubgraphValidationContext<'a>> + Sized + 'a {
    const NAME: &'static str;
    fn new(context: &mut SubgraphValidationContext<'a>) -> Self;
}

/// A graphql-js-style rule run with the [`SimpleValidationContext`]; see [`SubgraphRule`].
pub trait SimpleRule<'a>: graphql_js::Visitor<'a, SimpleValidationContext<'a>> + Sized + 'a {
    const NAME: &'static str;
    fn new(context: &mut SimpleValidationContext<'a>) -> Self;
}
