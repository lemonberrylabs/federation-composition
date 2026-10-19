//! Port of `src/subgraph/state.ts`: the state of one subgraph, built while its SDL is visited.
//!
//! Representation (see "Design decisions" in `crates/PORTING.md`):
//! - `SubgraphState` owns everything; types live in `types: IndexMap<String, SubgraphType>`.
//! - `createSubgraphStateBuilder` returns an object of factories closing over `state`; here it
//!   is [`SubgraphStateBuilder`], which owns the state and the captured locals. Each TS factory
//!   (`objectTypeFactory(state, ...)`) is a short-lived borrowing struct obtained from the
//!   builder: `builder.object_type().field().arg().set_type(..)` is
//!   `builder.objectType.field.arg.setType(..)`.
//! - `getOrCreate*` throw when a name is already used by another kind, so every setter returns
//!   `JsResult<()>`.
//! - AST directives stored in the state (`ast.directives`) are owned clones of the document's
//!   nodes.

use graphql_js::{
    Directive as DirectiveNode, DirectiveLocation, Document, NodeRef, ObjectTypeDefinition,
    ObjectTypeExtension, OperationType, SchemaDefinition, Type, VisitAction, VisitInfo, Visitor,
};
use indexmap::{IndexMap, IndexSet};

use crate::specifications::federation::FederationVersion;
use crate::specifications::link::{Link, LinkImport};
use crate::subgraph::validation::validation_context::SubgraphValidationContext;
use crate::utils::js::{JsError, JsResult};

/// `SubgraphType`.
#[derive(Clone, Debug, PartialEq)]
pub enum SubgraphType {
    Object(ObjectType),
    Interface(InterfaceType),
    InputObject(InputObjectType),
    Union(UnionType),
    Enum(EnumType),
    Scalar(ScalarType),
    Directive(Directive),
}

impl SubgraphType {
    /// `type.kind`.
    pub fn kind(&self) -> TypeKind {
        match self {
            SubgraphType::Object(_) => TypeKind::Object,
            SubgraphType::Interface(_) => TypeKind::Interface,
            SubgraphType::InputObject(_) => TypeKind::InputObject,
            SubgraphType::Union(_) => TypeKind::Union,
            SubgraphType::Enum(_) => TypeKind::Enum,
            SubgraphType::Scalar(_) => TypeKind::Scalar,
            SubgraphType::Directive(_) => TypeKind::Directive,
        }
    }

    /// `type.name`.
    pub fn name(&self) -> &str {
        match self {
            SubgraphType::Object(t) => &t.name,
            SubgraphType::Interface(t) => &t.name,
            SubgraphType::InputObject(t) => &t.name,
            SubgraphType::Union(t) => &t.name,
            SubgraphType::Enum(t) => &t.name,
            SubgraphType::Scalar(t) => &t.name,
            SubgraphType::Directive(t) => &t.name,
        }
    }
}

/// `TypeKind`. The TS enum values are the variant names in SCREAMING_CASE (`"OBJECT"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Object,
    Interface,
    Enum,
    Union,
    Scalar,
    InputObject,
    Directive,
}

impl TypeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TypeKind::Object => "OBJECT",
            TypeKind::Interface => "INTERFACE",
            TypeKind::Enum => "ENUM",
            TypeKind::Union => "UNION",
            TypeKind::Scalar => "SCALAR",
            TypeKind::InputObject => "INPUT_OBJECT",
            TypeKind::Directive => "DIRECTIVE",
        }
    }
}

/// `ArgumentKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ArgumentKind {
    #[default]
    Scalar,
    Object,
    Enum,
}

impl ArgumentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ArgumentKind::Scalar => "SCALAR",
            ArgumentKind::Object => "OBJECT",
            ArgumentKind::Enum => "ENUM",
        }
    }
}

/// `{ directives: DirectiveNode[] }`, the `ast` property of every state object.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ast {
    pub directives: Vec<DirectiveNode>,
}

/// `Directive` (`kind: TypeKind.DIRECTIVE`).
#[derive(Clone, Debug, PartialEq)]
pub struct Directive {
    pub name: String,
    /// Marked by @composeDirective directive as a composed directive.
    pub composed: bool,
    pub repeatable: bool,
    pub locations: IndexSet<String>,
    pub args: IndexMap<String, Argument>,
}

impl Directive {
    /// The `isExecutable` getter: whether it is an executable directive that can be used
    /// within GraphQL operation documents.
    pub fn is_executable(&self) -> bool {
        self.locations.iter().any(|l| is_executable_directive_location(l))
    }
}

/// `ScalarType`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarType {
    pub name: String,
    pub inaccessible: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub authenticated: bool,
    pub cost: Option<f64>,
    pub tags: IndexSet<String>,
    pub description: Option<Description>,
    pub specified_by: Option<String>,
    pub ast: Ast,
}

/// `extensionType?: "@extends" | "extend"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExtensionType {
    /// `"@extends"`.
    Extends,
    /// `"extend"`.
    Extend,
}

/// `ObjectType`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectType {
    pub name: String,
    pub fields: IndexMap<String, Field>,
    pub contexts: IndexSet<String>,
    pub extension: bool,
    pub extension_type: Option<ExtensionType>,
    pub external: bool,
    pub keys: Vec<Key>,
    pub fields_used_as_keys: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub shareable: bool,
    pub tags: IndexSet<String>,
    pub interfaces: IndexSet<String>,
    pub is_definition: bool,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
}

/// `InterfaceType`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceType {
    pub name: String,
    pub fields: IndexMap<String, Field>,
    pub contexts: IndexSet<String>,
    pub fields_used_as_keys: IndexSet<String>,
    pub extension: bool,
    pub keys: Vec<Key>,
    pub inaccessible: bool,
    pub tags: IndexSet<String>,
    pub interfaces: IndexSet<String>,
    pub is_definition: bool,
    pub is_interface_object: bool,
    pub description: Option<Description>,
    pub implemented_by: IndexSet<String>,
    pub ast: Ast,
}

/// `InputObjectType`.
#[derive(Clone, Debug, PartialEq)]
pub struct InputObjectType {
    pub name: String,
    pub fields: IndexMap<String, InputField>,
    pub extension: bool,
    pub inaccessible: bool,
    pub tags: IndexSet<String>,
    pub is_definition: bool,
    pub description: Option<Description>,
    pub ast: Ast,
    /// `isOneOf?: boolean`; `None` is `undefined`.
    pub is_one_of: Option<bool>,
}

/// `UnionType`.
#[derive(Clone, Debug, PartialEq)]
pub struct UnionType {
    pub name: String,
    pub members: IndexSet<String>,
    pub contexts: IndexSet<String>,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub is_definition: bool,
    pub description: Option<Description>,
    pub ast: Ast,
}

/// `EnumType`.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumType {
    pub name: String,
    pub values: IndexMap<String, EnumValue>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub tags: IndexSet<String>,
    pub is_definition: bool,
    pub description: Option<Description>,
    pub referenced_by_input_type: bool,
    pub referenced_by_output_type: bool,
    pub input_type_references: IndexSet<String>,
    pub output_type_references: IndexSet<String>,
    pub ast: Ast,
}

/// `Field` (object and interface fields).
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    /// The printed output type (`printOutputType`), `"MISSING"` until set.
    pub r#type: String,
    pub is_leaf: bool,
    pub args: IndexMap<String, Argument>,
    pub external: bool,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub list_size: Option<ListSize>,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub provides: Option<String>,
    pub requires: Option<String>,
    pub extension: bool,
    pub required: bool,
    pub provided: bool,
    pub shareable: bool,
    pub used_as_key: bool,
    pub used: bool,
    pub tags: IndexSet<String>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
}

/// `InputField`.
#[derive(Clone, Debug, PartialEq)]
pub struct InputField {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub inaccessible: bool,
    pub tags: IndexSet<String>,
    pub cost: Option<f64>,
    /// The printed default value (`print(node.defaultValue)`).
    pub default_value: Option<String>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
}

/// `EnumValue`.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub inaccessible: bool,
    pub tags: IndexSet<String>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
}

/// `fromContext?: { context, selection }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FromContext {
    pub context: String,
    pub selection: String,
}

/// `Argument` (field and directive arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub inaccessible: bool,
    pub tags: IndexSet<String>,
    pub cost: Option<f64>,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
}

/// `Key`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub fields: String,
    pub resolvable: bool,
}

/// `Description`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Description {
    pub value: String,
    pub block: bool,
}

/// `Deprecated` (`{ reason?, deprecated: true }`; `deprecated` is always true and implied).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Deprecated {
    pub reason: Option<String>,
}

/// `ListSize`.
#[derive(Clone, Debug, PartialEq)]
pub struct ListSize {
    pub print_require_one_slicing_argument: bool,
    pub assumed_size: Option<f64>,
    pub slicing_arguments: Option<Vec<String>>,
    pub sized_fields: Option<Vec<String>>,
    pub require_one_slicing_argument: bool,
}

/// `SubgraphState["graph"]`: `{ id, name, version, url? }`. Also `Graph` in
/// `supergraph/composition/common.ts`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubgraphGraph {
    pub id: String,
    pub name: String,
    pub version: FederationVersion,
    pub url: Option<String>,
}

/// The `graph` argument of `createSubgraphStateBuilder`: `{ id, name, url? }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphInput {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

/// A key of `SubgraphState["schema"]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RootTypeKind {
    QueryType,
    MutationType,
    SubscriptionType,
}

impl RootTypeKind {
    /// The TS property name (`"queryType"`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            RootTypeKind::QueryType => "queryType",
            RootTypeKind::MutationType => "mutationType",
            RootTypeKind::SubscriptionType => "subscriptionType",
        }
    }
}

/// `SubgraphState["schema"]`: the root types the subgraph provides (renamed to
/// `Query`/`Mutation`/`Subscription`).
/// NOTE(port): the TS object starts empty and gets its keys in the order the visitor assigns
/// them; `validateRootTypes` iterates it with `for..in`, so the insertion order is kept here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubgraphSchema {
    entries: IndexMap<RootTypeKind, String>,
}

impl SubgraphSchema {
    pub fn get(&self, kind: RootTypeKind) -> Option<&str> {
        self.entries.get(&kind).map(String::as_str)
    }

    /// `schema[kind] = name` (an existing key keeps its position).
    pub fn set(&mut self, kind: RootTypeKind, name: String) {
        self.entries.insert(kind, name);
    }

    /// `for (const key in state.schema)`.
    pub fn iter(&self) -> impl Iterator<Item = (RootTypeKind, &str)> {
        self.entries.iter().map(|(k, v)| (*k, v.as_str()))
    }

    pub fn query_type(&self) -> Option<&str> {
        self.get(RootTypeKind::QueryType)
    }

    pub fn mutation_type(&self) -> Option<&str> {
        self.get(RootTypeKind::MutationType)
    }

    pub fn subscription_type(&self) -> Option<&str> {
        self.get(RootTypeKind::SubscriptionType)
    }
}

/// `specs.cost.names`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostSpecNames {
    pub cost: Option<String>,
    pub list_size: Option<String>,
}

/// `specs.cost`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostSpec {
    pub used: bool,
    pub names: CostSpecNames,
}

/// `SubgraphState["specs"]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubgraphSpecs {
    pub tag: bool,
    pub context: bool,
    pub inaccessible: bool,
    pub cost: CostSpec,
    pub link: bool,
    pub policy: bool,
    pub requires_scopes: bool,
    pub authenticated: bool,
}

/// `keyof SubgraphState["specs"]["cost"]["names"]`, the argument of `markCostSpecAsUsed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostSpecDirective {
    Cost,
    ListSize,
}

/// `Exclude<keyof SubgraphState["specs"], "cost">`, the argument of `markSpecAsUsed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecName {
    Tag,
    Context,
    Inaccessible,
    Link,
    Policy,
    RequiresScopes,
    Authenticated,
}

/// `SubgraphState["federation"]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SubgraphFederation {
    pub version: FederationVersion,
    pub imports: Vec<LinkImport>,
}

/// `SubgraphState`.
#[derive(Clone, Debug, PartialEq)]
pub struct SubgraphState {
    pub graph: SubgraphGraph,
    /// `[typeName, Type]`.
    pub types: IndexMap<String, SubgraphType>,
    /// If an operation type is defined, the subgraph provides that type.
    pub schema: SubgraphSchema,
    pub links: Vec<Link>,
    pub specs: SubgraphSpecs,
    pub federation: SubgraphFederation,
}

const MISSING: &str = "MISSING";

/// The expected root type names (`expectedQueryTypeName`, ...) the builder closures use for
/// `renameObjectType`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootTypeNames {
    pub query: String,
    pub mutation: String,
    pub subscription: String,
}

impl RootTypeNames {
    /// `renameObjectType(typeName)`: maps the schema's root type names to
    /// `Query`/`Mutation`/`Subscription`.
    pub fn rename_object_type(&self, type_name: &str) -> String {
        if type_name == self.query {
            return "Query".to_string();
        }
        if type_name == self.mutation {
            return "Mutation".to_string();
        }
        if type_name == self.subscription {
            return "Subscription".to_string();
        }
        type_name.to_string()
    }
}

/// `SubgraphStateBuilder = ReturnType<typeof createSubgraphStateBuilder>`.
/// Owns the state and the locals the TS closures capture.
#[derive(Clone, Debug)]
pub struct SubgraphStateBuilder {
    pub state: SubgraphState,
    /// A set of directive names that are marked as composed directives.
    pub composed_directives: IndexSet<String>,
    pub root_type_names: RootTypeNames,
    /// `specifiedScalarTypes.map((type) => type.name)`.
    specified_scalars: Vec<String>,
    leaf_type_names: IndexSet<String>,
    enum_type_names: IndexSet<String>,
    input_object_type_names: IndexSet<String>,
    /// The names of enum definitions and extensions in `typeDefs`; computed by `visitor()` in
    /// TS (from the same document object, after `validateSubgraph` strips `_Any`, `_Entity`,
    /// `_Service` and `_service`/`_entities`, which does not change enum names).
    enum_types_by_name: IndexSet<String>,
}

/// `createSubgraphStateBuilder(graph, typeDefs, version, links)`.
pub fn create_subgraph_state_builder(
    graph: &GraphInput,
    type_defs: &Document,
    version: FederationVersion,
    links: &[Link],
) -> SubgraphStateBuilder {
    todo!()
}

impl SubgraphStateBuilder {
    /// `isInterfaceObject(typeName)`.
    pub fn is_interface_object(&self, type_name: &str) -> bool {
        is_interface_object(&self.state, type_name)
    }

    /// `resolveArgumentKind(typeName)`.
    fn resolve_argument_kind(&self, type_name: &str) -> ArgumentKind {
        if self.enum_type_names.contains(type_name) {
            return ArgumentKind::Enum;
        }
        if self.input_object_type_names.contains(type_name) {
            return ArgumentKind::Object;
        }
        ArgumentKind::Scalar
    }

    /// `builder.directive`.
    pub fn directive(&mut self) -> DirectiveBuilder<'_> {
        DirectiveBuilder { state: &mut self.state }
    }

    /// `builder.scalarType`.
    pub fn scalar_type(&mut self) -> ScalarTypeBuilder<'_> {
        ScalarTypeBuilder { state: &mut self.state }
    }

    /// `builder.objectType`.
    pub fn object_type(&mut self) -> ObjectTypeBuilder<'_> {
        ObjectTypeBuilder {
            state: &mut self.state,
            roots: &self.root_type_names,
        }
    }

    /// `builder.interfaceType`.
    pub fn interface_type(&mut self) -> InterfaceTypeBuilder<'_> {
        InterfaceTypeBuilder { state: &mut self.state }
    }

    /// `builder.inputObjectType`.
    pub fn input_object_type(&mut self) -> InputObjectTypeBuilder<'_> {
        InputObjectTypeBuilder { state: &mut self.state }
    }

    /// `builder.unionType`.
    pub fn union_type(&mut self) -> UnionTypeBuilder<'_> {
        UnionTypeBuilder { state: &mut self.state }
    }

    /// `builder.enumType`.
    pub fn enum_type(&mut self) -> EnumTypeBuilder<'_> {
        EnumTypeBuilder { state: &mut self.state }
    }

    /// `markCostSpecAsUsed(directive, name)`.
    pub fn mark_cost_spec_as_used(&mut self, directive: CostSpecDirective, name: &str) {
        self.state.specs.cost.used = true;
        match directive {
            CostSpecDirective::Cost => self.state.specs.cost.names.cost = Some(name.to_string()),
            CostSpecDirective::ListSize => {
                self.state.specs.cost.names.list_size = Some(name.to_string())
            }
        }
    }

    /// `markSpecAsUsed(specName)`.
    pub fn mark_spec_as_used(&mut self, spec_name: SpecName) {
        let specs = &mut self.state.specs;
        match spec_name {
            SpecName::Tag => specs.tag = true,
            SpecName::Context => specs.context = true,
            SpecName::Inaccessible => specs.inaccessible = true,
            SpecName::Link => specs.link = true,
            SpecName::Policy => specs.policy = true,
            SpecName::RequiresScopes => specs.requires_scopes = true,
            SpecName::Authenticated => specs.authenticated = true,
        }
    }

    /// `visitor(typeNodeInfo)`: the visitor that fills the state. It reads the
    /// `TypeNodeInfo` and the builder from the validation context it is visited with.
    pub fn visitor(&self) -> SubgraphStateVisitor {
        SubgraphStateVisitor {
            enum_types_by_name: self.enum_types_by_name.clone(),
        }
    }

    /// Moves the state out (validate.ts reads `builder.state` once validation is done).
    pub fn into_state(self) -> SubgraphState {
        self.state
    }
}

/// The free function behind the `isInterfaceObject` closure.
fn is_interface_object(state: &SubgraphState, type_name: &str) -> bool {
    match state.types.get(type_name) {
        Some(SubgraphType::Interface(i)) => i.is_interface_object,
        _ => false,
    }
}

/// The `ASTVisitor` returned by `builder.visitor(typeNodeInfo)`.
/// A TS `throw` inside a callback is reported with `cx.throw(err)`, which records the error
/// and stops the traversal (see `SubgraphValidationContext::throw`).
#[derive(Clone, Debug)]
pub struct SubgraphStateVisitor {
    enum_types_by_name: IndexSet<String>,
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for SubgraphStateVisitor {
    /// The callbacks `FieldDefinition`, `InputValueDefinition`, `ObjectTypeDefinition`,
    /// `ObjectTypeExtension`, `InterfaceTypeDefinition`, `InterfaceTypeExtension`,
    /// `UnionTypeDefinition`, `UnionTypeExtension`, `EnumTypeDefinition`,
    /// `EnumValueDefinition`, `InputObjectTypeDefinition`, `ScalarTypeDefinition`,
    /// `ScalarTypeExtension`, `Directive`, `DirectiveDefinition` and `StringValue(node, _, parent)`
    /// (all are enter callbacks).
    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut SubgraphValidationContext<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        todo!()
    }
}

/// `directiveFactory(state)`.
pub struct DirectiveBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> DirectiveBuilder<'s> {
    pub fn set_composed(self, directive_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_location(self, directive_name: &str, location: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_repeatable(self, directive_name: &str) -> JsResult<()> {
        todo!()
    }
    /// `directive.arg`.
    pub fn arg(self) -> DirectiveArgBuilder<'s> {
        DirectiveArgBuilder { state: self.state }
    }
}

/// `directiveFactory(state).arg`.
pub struct DirectiveArgBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl DirectiveArgBuilder<'_> {
    pub fn set_tag(self, directive_name: &str, arg_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_type(self, directive_name: &str, arg_name: &str, arg_type: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_kind(self, directive_name: &str, arg_name: &str, arg_kind: ArgumentKind) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, arg_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn set_default_value(self, type_name: &str, arg_name: &str, default_value: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str, arg_name: &str) -> JsResult<()> {
        todo!()
    }
}

/// `cleanSubgraphStateFromFederationSpec(state)`: removes types specific to the federation spec.
pub fn clean_subgraph_state_from_federation_spec(mut state: SubgraphState) -> SubgraphState {
    state.types.shift_remove("_FieldSet");
    state.types.shift_remove("federation__FieldSet");
    state.types.shift_remove("federation__Policy");
    state.types.shift_remove("federation__Scope");
    state.types.shift_remove("federation__ContextFieldValue");
    state
}

/// `cleanSubgraphStateFromLinkSpec(state)`: removes types specific to the link spec.
pub fn clean_subgraph_state_from_link_spec(mut state: SubgraphState) -> SubgraphState {
    state.types.shift_remove("link__Import");
    state.types.shift_remove("link__Purpose");
    state.types.shift_remove("link__Import");
    state.types.shift_remove("link");
    state
}

/// `scalarTypeFactory(state)`.
pub struct ScalarTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl ScalarTypeBuilder<'_> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_authenticated(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_policies(self, type_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_scopes(self, type_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_specified_by(self, type_name: &str, url: &str) -> JsResult<()> {
        todo!()
    }
}

/// `objectTypeFactory(state, renameObject, interfaceTypeBuilder, isInterfaceObject)`.
/// The interface builder is `InterfaceTypeBuilder { state }` made on demand; `isInterfaceObject`
/// is [`is_interface_object`] on the same state; `renameObject` is
/// [`RootTypeNames::rename_object_type`].
pub struct ObjectTypeBuilder<'s> {
    state: &'s mut SubgraphState,
    roots: &'s RootTypeNames,
}

impl<'s> ObjectTypeBuilder<'s> {
    fn interface_type_builder(&mut self) -> InterfaceTypeBuilder<'_> {
        InterfaceTypeBuilder { state: self.state }
    }

    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_extension(self, type_name: &str, extension_type: ExtensionType) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_external(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_interface(self, type_name: &str, interface_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_key(
        self,
        type_name: &str,
        fields: &str,
        fields_used_in_key: &IndexSet<String>,
        resolvable: bool,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_authenticated(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_policies(self, type_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_scopes(self, type_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
    pub fn set_shareable(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn add_context(self, type_name: &str, context_name: &str) -> JsResult<()> {
        todo!()
    }
    /// `objectType.field`.
    pub fn field(self) -> ObjectTypeFieldBuilder<'s> {
        ObjectTypeFieldBuilder {
            state: self.state,
            roots: self.roots,
        }
    }
}

/// `objectTypeFactory(..).field`.
pub struct ObjectTypeFieldBuilder<'s> {
    state: &'s mut SubgraphState,
    roots: &'s RootTypeNames,
}

impl<'s> ObjectTypeFieldBuilder<'s> {
    pub fn set_type(self, type_name: &str, field_name: &str, field_type: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_leaf(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_extension(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, field_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, field_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, reason: Option<&str>) -> JsResult<()> {
        todo!()
    }
    pub fn set_authenticated(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_policies(self, type_name: &str, field_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_scopes(self, type_name: &str, field_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
    pub fn set_list_size(self, type_name: &str, field_name: &str, list_size: ListSize) -> JsResult<()> {
        todo!()
    }
    pub fn set_external(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_override(
        self,
        type_name: &str,
        field_name: &str,
        r#override: &str,
        label: Option<&str>,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_provides(self, type_name: &str, field_name: &str, provides: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_requires(self, type_name: &str, field_name: &str, requires: &str) -> JsResult<()> {
        todo!()
    }
    pub fn mark_as_provided(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn marked_as_required(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_shareable(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_used(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    /// `objectType.field.arg`.
    pub fn arg(self) -> ObjectTypeFieldArgBuilder<'s> {
        ObjectTypeFieldArgBuilder {
            state: self.state,
            roots: self.roots,
        }
    }
}

/// `objectTypeFactory(..).field.arg`.
pub struct ObjectTypeFieldArgBuilder<'s> {
    state: &'s mut SubgraphState,
    roots: &'s RootTypeNames,
}

impl ObjectTypeFieldArgBuilder<'_> {
    pub fn set_type(self, type_name: &str, field_name: &str, arg_name: &str, arg_type: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_kind(self, type_name: &str, field_name: &str, arg_name: &str, arg_kind: ArgumentKind) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        description: Description,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, arg_name: &str, reason: Option<&str>) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        directive: &DirectiveNode,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_default_value(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        default_value: &str,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str, arg_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_from_context(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        from_context: FromContext,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, arg_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, arg_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
}

/// `interfaceTypeFactory(state)`.
pub struct InterfaceTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> InterfaceTypeBuilder<'s> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    /// For Federation v1: set as extension only if it uses @extends directive (not for
    /// `extend type` syntax). For Federation v2: set as extension if it's not a definition.
    pub fn set_extension(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_interface(self, type_name: &str, interface_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_interface_object(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_key(
        self,
        type_name: &str,
        fields: &str,
        fields_used_in_key: &IndexSet<String>,
        resolvable: bool,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn add_context(self, type_name: &str, context_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    /// `interfaceType.field`.
    pub fn field(self) -> InterfaceTypeFieldBuilder<'s> {
        InterfaceTypeFieldBuilder { state: self.state }
    }
}

/// `interfaceTypeFactory(state).field`.
pub struct InterfaceTypeFieldBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> InterfaceTypeFieldBuilder<'s> {
    pub fn set_type(self, type_name: &str, field_name: &str, field_type: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_leaf(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_external(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_authenticated(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_policies(self, type_name: &str, field_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_scopes(self, type_name: &str, field_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
    pub fn set_list_size(self, type_name: &str, field_name: &str, list_size: ListSize) -> JsResult<()> {
        todo!()
    }
    pub fn set_override(
        self,
        type_name: &str,
        field_name: &str,
        r#override: &str,
        label: Option<&str>,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_requires(self, type_name: &str, field_name: &str, requires: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_shareable(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_used(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, field_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, field_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, reason: Option<&str>) -> JsResult<()> {
        todo!()
    }
    /// `interfaceType.field.arg`.
    pub fn arg(self) -> InterfaceTypeFieldArgBuilder<'s> {
        InterfaceTypeFieldArgBuilder { state: self.state }
    }
}

/// `interfaceTypeFactory(state).field.arg`.
pub struct InterfaceTypeFieldArgBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl InterfaceTypeFieldArgBuilder<'_> {
    pub fn set_type(self, type_name: &str, field_name: &str, arg_name: &str, arg_type: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_kind(self, type_name: &str, field_name: &str, arg_name: &str, arg_kind: ArgumentKind) -> JsResult<()> {
        todo!()
    }
    pub fn set_default_value(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        default_value: &str,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, arg_name: &str, reason: Option<&str>) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        description: Description,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, arg_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str, arg_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, arg_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        directive: &DirectiveNode,
    ) -> JsResult<()> {
        todo!()
    }
    pub fn set_from_context(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        from_context: FromContext,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `inputObjectTypeFactory(state)`.
pub struct InputObjectTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> InputObjectTypeBuilder<'s> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_extension(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_one_of(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    /// `inputObjectType.field`.
    pub fn field(self) -> InputObjectTypeFieldBuilder<'s> {
        InputObjectTypeFieldBuilder { state: self.state }
    }
}

/// `inputObjectTypeFactory(state).field`.
pub struct InputObjectTypeFieldBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl InputObjectTypeFieldBuilder<'_> {
    pub fn set_type(self, type_name: &str, field_name: &str, field_type: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_kind(self, type_name: &str, field_name: &str, field_kind: ArgumentKind) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, field_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, reason: Option<&str>) -> JsResult<()> {
        todo!()
    }
    pub fn set_default_value(self, type_name: &str, field_name: &str, default_value: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, field_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
}

/// `unionTypeFactory(state)`.
pub struct UnionTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl UnionTypeBuilder<'_> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_member(self, type_name: &str, member: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn add_context(self, type_name: &str, context_name: &str) -> JsResult<()> {
        todo!()
    }
}

/// `enumTypeFactory(state)`.
pub struct EnumTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> EnumTypeBuilder<'s> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_authenticated(self, type_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_policies(self, type_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_scopes(self, type_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        todo!()
    }
    pub fn set_cost(self, type_name: &str, cost: f64) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_referenced_by_input_type(self, type_name: &str, schema_coordinate: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_referenced_by_output_type(self, type_name: &str, schema_coordinate: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    /// `enumType.value`.
    pub fn value(self) -> EnumTypeValueBuilder<'s> {
        EnumTypeValueBuilder { state: self.state }
    }
}

/// `enumTypeFactory(state).value`.
pub struct EnumTypeValueBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl EnumTypeValueBuilder<'_> {
    pub fn set_value(self, type_name: &str, value_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_description(self, type_name: &str, value_name: &str, description: Description) -> JsResult<()> {
        todo!()
    }
    pub fn set_directive(self, type_name: &str, value_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        todo!()
    }
    pub fn set_inaccessible(self, type_name: &str, value_name: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_tag(self, type_name: &str, value_name: &str, tag: &str) -> JsResult<()> {
        todo!()
    }
    pub fn set_deprecated(self, type_name: &str, value_name: &str, reason: Option<&str>) -> JsResult<()> {
        todo!()
    }
}

/// `executableDirectiveLocations`.
const EXECUTABLE_DIRECTIVE_LOCATIONS: [DirectiveLocation; 8] = [
    DirectiveLocation::Field,
    DirectiveLocation::FragmentDefinition,
    DirectiveLocation::InlineFragment,
    DirectiveLocation::FragmentSpread,
    DirectiveLocation::VariableDefinition,
    DirectiveLocation::Query,
    DirectiveLocation::Mutation,
    DirectiveLocation::Subscription,
];

/// `isExecutableDirectiveLocation(location)`.
pub fn is_executable_directive_location(location: &str) -> bool {
    EXECUTABLE_DIRECTIVE_LOCATIONS.iter().any(|l| l.as_str() == location)
}

fn get_or_create_directive<'s>(state: &'s mut SubgraphState, directive_name: &str) -> JsResult<&'s mut Directive> {
    todo!()
}

fn get_or_create_directive_arg<'s>(
    state: &'s mut SubgraphState,
    directive_name: &str,
    arg_name: &str,
) -> JsResult<&'s mut Argument> {
    todo!()
}

fn get_or_create_scalar_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut ScalarType> {
    todo!()
}

/// `getOrCreateObjectType(state, renameObject, typeName)`.
fn get_or_create_object_type<'s>(
    state: &'s mut SubgraphState,
    rename_object: &RootTypeNames,
    type_name: &str,
) -> JsResult<&'s mut ObjectType> {
    todo!()
}

fn get_or_create_interface_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut InterfaceType> {
    todo!()
}

fn get_or_create_input_object_type<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
) -> JsResult<&'s mut InputObjectType> {
    todo!()
}

fn get_or_create_enum_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut EnumType> {
    todo!()
}

fn get_or_create_union_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut UnionType> {
    todo!()
}

fn get_or_create_object_field<'s>(
    state: &'s mut SubgraphState,
    rename_object: &RootTypeNames,
    type_name: &str,
    field_name: &str,
) -> JsResult<&'s mut Field> {
    todo!()
}

fn get_or_create_interface_field<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    field_name: &str,
) -> JsResult<&'s mut Field> {
    todo!()
}

fn get_or_create_input_object_field<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    field_name: &str,
) -> JsResult<&'s mut InputField> {
    todo!()
}

fn get_or_create_enum_value<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    enum_value_name: &str,
) -> JsResult<&'s mut EnumValue> {
    todo!()
}

fn get_or_create_object_field_argument<'s>(
    state: &'s mut SubgraphState,
    rename_object: &RootTypeNames,
    type_name: &str,
    field_name: &str,
    arg_name: &str,
) -> JsResult<&'s mut Argument> {
    todo!()
}

fn get_or_create_interface_field_argument<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    field_name: &str,
    arg_name: &str,
) -> JsResult<&'s mut Argument> {
    todo!()
}

fn is_schema_definition(node: &graphql_js::Definition) -> Option<&SchemaDefinition> {
    match node {
        graphql_js::Definition::SchemaDefinition(s) => Some(s),
        _ => None,
    }
}

/// `resolveTypeName(node)`: the named type inside list/non-null wrappers.
fn resolve_type_name(node: &Type) -> String {
    match node {
        Type::Named(n) => n.name.value.clone(),
        Type::List(l) => resolve_type_name(&l.ty),
        Type::NonNull(n) => resolve_type_name(&n.ty),
    }
}

/// `isEnumType(node)`: an enum type definition or extension.
fn is_enum_type(node: &graphql_js::Definition) -> bool {
    matches!(
        node,
        graphql_js::Definition::EnumTypeDefinition(_) | graphql_js::Definition::EnumTypeExtension(_)
    )
}

/// `decideOnRootTypeName(schemaDef, kind, defaultName)`.
fn decide_on_root_type_name(schema_def: Option<&SchemaDefinition>, kind: OperationType, default_name: &str) -> String {
    schema_def
        .and_then(|s| s.operation_types.iter().find(|op| op.operation == kind))
        .map(|op| op.ty.name.value.clone())
        .unwrap_or_else(|| default_name.to_string())
}

/// `hasInterfaceObjectDirective(node)` for an object type definition.
fn has_interface_object_directive(directives: &[DirectiveNode]) -> bool {
    directives.iter().any(|d| d.name.value == "interfaceObject")
}
