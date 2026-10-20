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

use graphql_rs::{
    specified_scalar_types, Ancestor, Definition, Directive as DirectiveNode, DirectiveDefinition,
    DirectiveLocation, Document, EnumValueDefinition, FieldDefinition, InputValueDefinition,
    InterfaceTypeDefinition, InterfaceTypeExtension, NodeRef, ObjectTypeDefinition,
    ObjectTypeExtension, OperationType, ScalarTypeDefinition, SchemaDefinition, StringValue, Type,
    UnionTypeDefinition, UnionTypeExtension, Value, VisitAction, VisitInfo, Visitor,
};
use crate::utils::collections::{IndexMap, IndexSet};

use crate::graphql::printer::print;
use crate::graphql::type_node_info::TypeNodeInfo;
use crate::specifications::federation::{is_federation_link, FederationVersion};
use crate::specifications::link::{Link, LinkImport, LinkImportKind};
use crate::subgraph::helpers::print_output_type;
use crate::subgraph::validation::validation_context::SubgraphValidationContext;
use crate::utils::auth::merge_scope_policies;
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
    /// PERF: returns a borrowed name instead of a new string.
    pub fn rename_object_type<'n>(&self, type_name: &'n str) -> &'n str {
        if type_name == self.query {
            return "Query";
        }
        if type_name == self.mutation {
            return "Mutation";
        }
        if type_name == self.subscription {
            return "Subscription";
        }
        type_name
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
    let federation_link = links.iter().find(|link| is_federation_link(link));
    let links_with_directive: Vec<Link> = links
        .iter()
        // Reject federation links and links without directives
        .filter(|link| {
            !is_federation_link(link)
                && link
                    .imports
                    .iter()
                    .any(|im| im.kind == LinkImportKind::Directive)
        })
        .cloned()
        .collect();
    let is_link_spec_manually_provided = type_defs.definitions.iter().any(|def| match def {
        Definition::DirectiveDefinition(def) => {
            def.name.value == "link" && def.locations.iter().all(|loc| loc.value == "SCHEMA")
        }
        _ => false,
    });
    let specified_scalars: Vec<String> = specified_scalar_types()
        .iter()
        .map(|ty| ty.name.to_string())
        .collect();

    let state = SubgraphState {
        graph: SubgraphGraph {
            id: graph.id.clone(),
            name: graph.name.clone(),
            version,
            url: graph.url.clone(),
        },
        types: IndexMap::default(),
        schema: SubgraphSchema::default(),
        links: links_with_directive,
        specs: SubgraphSpecs {
            tag: false,
            context: false,
            cost: CostSpec {
                used: false,
                names: CostSpecNames {
                    cost: None,
                    list_size: None,
                },
            },
            inaccessible: false,
            authenticated: false,
            requires_scopes: false,
            policy: false,
            link: is_link_spec_manually_provided,
        },
        federation: SubgraphFederation {
            version,
            imports: federation_link
                .map(|link| link.imports.clone())
                .unwrap_or_default(),
        },
    };

    let schema_def = type_defs.definitions.iter().find_map(is_schema_definition);

    let mut leaf_type_names: IndexSet<String> = specified_scalars.iter().cloned().collect();
    let mut enum_type_names: IndexSet<String> = IndexSet::default();
    let mut input_object_type_names: IndexSet<String> = IndexSet::default();

    for type_def in &type_defs.definitions {
        match type_def {
            Definition::EnumTypeDefinition(def) => {
                enum_type_names.insert(def.name.value.clone());
                leaf_type_names.insert(def.name.value.clone());
            }
            Definition::EnumTypeExtension(def) => {
                enum_type_names.insert(def.name.value.clone());
                leaf_type_names.insert(def.name.value.clone());
            }
            Definition::ScalarTypeDefinition(def) => {
                leaf_type_names.insert(def.name.value.clone());
            }
            Definition::ScalarTypeExtension(def) => {
                leaf_type_names.insert(def.name.value.clone());
            }
            Definition::InputObjectTypeDefinition(def) => {
                input_object_type_names.insert(def.name.value.clone());
            }
            Definition::InputObjectTypeExtension(def) => {
                input_object_type_names.insert(def.name.value.clone());
            }
            _ => {}
        }
    }

    let root_type_names = RootTypeNames {
        query: decide_on_root_type_name(schema_def, OperationType::Query, "Query"),
        mutation: decide_on_root_type_name(schema_def, OperationType::Mutation, "Mutation"),
        subscription: decide_on_root_type_name(
            schema_def,
            OperationType::Subscription,
            "Subscription",
        ),
    };

    // NOTE(port): the TS computes `enumTypesByName` in `visitor()`, from the same `typeDefs`
    // object; the enum definitions it reads do not change in between, so it is computed here.
    let enum_types_by_name: IndexSet<String> = type_defs
        .definitions
        .iter()
        .filter(|def| is_enum_type(def))
        .filter_map(|def| match def {
            Definition::EnumTypeDefinition(def) => Some(def.name.value.clone()),
            Definition::EnumTypeExtension(def) => Some(def.name.value.clone()),
            _ => None,
        })
        .collect();

    SubgraphStateBuilder {
        state,
        composed_directives: IndexSet::default(),
        root_type_names,
        specified_scalars,
        leaf_type_names,
        enum_type_names,
        input_object_type_names,
        enum_types_by_name,
    }
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
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::FieldDefinition, Kind::InputValueDefinition, Kind::ObjectTypeDefinition, Kind::ObjectTypeExtension, Kind::InterfaceTypeDefinition, Kind::InterfaceTypeExtension, Kind::UnionTypeDefinition, Kind::UnionTypeExtension, Kind::EnumTypeDefinition, Kind::EnumValueDefinition, Kind::InputObjectTypeDefinition, Kind::ScalarTypeDefinition, Kind::ScalarTypeExtension, Kind::Directive, Kind::DirectiveDefinition, Kind::StringValue])
    }

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
        let type_node_info = cx.type_node_info;
        let result = self.enter_node(node, &mut *cx.state_builder, &type_node_info, info);
        match result {
            Ok(()) => VisitAction::Continue,
            Err(error) => cx.throw(error),
        }
    }
}

/// The name of a node kept by `TypeNodeInfo` (type definitions and extensions, directive
/// definitions, field and input value definitions all have one).
fn node_name<'a>(node: NodeRef<'a>) -> &'a str {
    node.name().map(|name| name.value.as_str()).unwrap_or_default()
}

/// `fieldDef!.name.value` where the TS asserts `fieldDef` is defined. If it were not, JS would
/// throw a `TypeError` reading `name` of `undefined`.
fn asserted_field_name<'a>(field_def: Option<NodeRef<'a>>) -> JsResult<&'a str> {
    match field_def {
        Some(field_def) => Ok(node_name(field_def)),
        None => Err(JsError::type_error(
            "Cannot read properties of undefined (reading 'name')",
        )),
    }
}

/// `node.arguments?.find((arg) => arg.name.value === name)?.value` when it is a `StringValue`.
fn find_string_argument<'a>(node: &'a DirectiveNode, name: &str) -> Option<&'a str> {
    match node.arguments.iter().find(|arg| arg.name.value == name) {
        Some(arg) => match &arg.value {
            Value::String(s) => Some(s.value.as_str()),
            _ => None,
        },
        None => None,
    }
}

impl SubgraphStateVisitor {
    fn enter_node<'a>(
        &self,
        node: NodeRef<'a>,
        b: &mut SubgraphStateBuilder,
        type_node_info: &TypeNodeInfo<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> JsResult<()> {
        match node {
            NodeRef::FieldDefinition(node) => self.field_definition(node, b, type_node_info),
            NodeRef::InputValueDefinition(node) => {
                self.input_value_definition(node, b, type_node_info)
            }
            NodeRef::ObjectTypeDefinition(node) => object_type_definition(node, b),
            NodeRef::ObjectTypeExtension(node) => object_type_extension(node, b),
            NodeRef::InterfaceTypeDefinition(node) => interface_type_definition(node, b),
            NodeRef::InterfaceTypeExtension(node) => interface_type_extension(node, b),
            NodeRef::UnionTypeDefinition(node) => union_type_definition(node, b),
            NodeRef::UnionTypeExtension(node) => union_type_extension(node, b),
            NodeRef::EnumTypeDefinition(node) => b.enum_type().set_definition(&node.name.value),
            NodeRef::EnumValueDefinition(node) => enum_value_definition(node, b, type_node_info),
            NodeRef::InputObjectTypeDefinition(node) => {
                b.input_object_type().set_definition(&node.name.value)
            }
            NodeRef::ScalarTypeDefinition(node) => scalar_type_definition(node, b),
            NodeRef::ScalarTypeExtension(_) => {
                // TODO: T06 implement scalar type extension
                Ok(())
            }
            NodeRef::Directive(node) => directive(node, b, type_node_info),
            NodeRef::DirectiveDefinition(node) => directive_definition(node, b),
            NodeRef::StringValue(node) => string_value(node, b, type_node_info, info),
            _ => Ok(()),
        }
    }

    /// `FieldDefinition(node)`.
    fn field_definition<'a>(
        &self,
        node: &'a FieldDefinition,
        b: &mut SubgraphStateBuilder,
        type_node_info: &TypeNodeInfo<'a>,
    ) -> JsResult<()> {
        let Some(type_def) = type_node_info.get_type_def() else {
            return Err(JsError::error(
                "Expected to find a type definition or extension",
            ));
        };

        let is_interface_type = matches!(
            type_def,
            NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_)
        );
        let is_object_type = matches!(
            type_def,
            NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_)
        );
        let type_name = node_name(type_def);
        let field_name = node.name.value.as_str();

        let output_type_name = resolve_type_name(&node.ty);
        let is_leaf = b.leaf_type_names.contains(output_type_name);
        let references_enum_type = self.enum_types_by_name.contains(output_type_name);

        if references_enum_type {
            b.enum_type().set_referenced_by_output_type(
                &output_type_name,
                &format!("{type_name}.{field_name}"),
            )?;
        }

        if is_interface_type || b.is_interface_object(type_name) {
            b.interface_type()
                .field()
                .set_type(type_name, field_name, &print_output_type(&node.ty))?;
            b.interface_type().field().set_used(type_name, field_name)?;

            if is_leaf {
                b.interface_type().field().set_leaf(type_name, field_name)?;
            }

            return Ok(());
        }

        if !is_object_type {
            return Err(JsError::error("Expected to find an object type"));
        }

        b.object_type()
            .field()
            .set_type(type_name, field_name, &print_output_type(&node.ty))?;

        if is_leaf {
            b.object_type().field().set_leaf(type_name, field_name)?;
        }

        if matches!(type_def, NodeRef::ObjectTypeExtension(_)) {
            b.object_type().field().set_extension(type_name, field_name)?;
        }

        // In Federation v1, all fields are shareable by default, but only in object type definition.
        // Extensions are not shareable.
        // The exception are root types, all their fields are shareable.
        if b.state.graph.version == FederationVersion::V1_0
            && (matches!(type_def, NodeRef::ObjectTypeDefinition(_))
                || type_name == b.root_type_names.query
                || type_name == b.root_type_names.mutation
                || type_name == b.root_type_names.subscription)
        {
            b.object_type().field().set_shareable(type_name, field_name)?;
        }

        Ok(())
    }

    /// `InputValueDefinition(node)`.
    fn input_value_definition<'a>(
        &self,
        node: &'a InputValueDefinition,
        b: &mut SubgraphStateBuilder,
        type_node_info: &TypeNodeInfo<'a>,
    ) -> JsResult<()> {
        let type_def = type_node_info.get_type_def();
        let field_def = type_node_info.get_field_def();

        let Some(type_def) = type_def else {
            return Ok(());
        };

        let is_input_object_type = matches!(
            type_def,
            NodeRef::InputObjectTypeDefinition(_) | NodeRef::InputObjectTypeExtension(_)
        );
        let is_object_type = matches!(
            type_def,
            NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_)
        );
        let is_interface_type = matches!(
            type_def,
            NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_)
        );
        let type_name = node_name(type_def);
        let name = node.name.value.as_str();

        let output_type_name = resolve_type_name(&node.ty);
        let references_enum_type = self.enum_types_by_name.contains(output_type_name);

        if is_input_object_type {
            b.input_object_type()
                .field()
                .set_type(type_name, name, &print_output_type(&node.ty))?;

            let kind = b.resolve_argument_kind(output_type_name);
            b.input_object_type().field().set_kind(type_name, name, kind)?;

            if references_enum_type {
                b.enum_type().set_referenced_by_input_type(
                    &output_type_name,
                    &format!("{type_name}.{name}"),
                )?;
            }

            if let Some(default_value) = &node.default_value {
                b.input_object_type()
                    .field()
                    .set_default_value(type_name, name, &print(default_value))?;
            }
        }

        if is_object_type {
            if let Some(field_def) = field_def {
                let field_name = node_name(field_def);
                // It's a field argument
                b.object_type().field().arg().set_type(
                    type_name,
                    field_name,
                    name,
                    &print_output_type(&node.ty),
                )?;

                let kind = b.resolve_argument_kind(output_type_name);
                b.object_type()
                    .field()
                    .arg()
                    .set_kind(type_name, field_name, name, kind)?;

                if let Some(default_value) = &node.default_value {
                    b.object_type().field().arg().set_default_value(
                        type_name,
                        field_name,
                        name,
                        &print(default_value),
                    )?;
                }

                if references_enum_type {
                    b.enum_type().set_referenced_by_input_type(
                        &output_type_name,
                        &format!("{type_name}.{field_name}({name}:)"),
                    )?;
                }
            }
        }

        if is_interface_type {
            if let Some(field_def) = field_def {
                let field_name = node_name(field_def);
                // It's a field argument
                b.interface_type().field().arg().set_type(
                    type_name,
                    field_name,
                    name,
                    &print_output_type(&node.ty),
                )?;

                let kind = b.resolve_argument_kind(output_type_name);
                b.interface_type()
                    .field()
                    .arg()
                    .set_kind(type_name, field_name, name, kind)?;

                if let Some(default_value) = &node.default_value {
                    b.interface_type().field().arg().set_default_value(
                        type_name,
                        field_name,
                        name,
                        &print(default_value),
                    )?;
                }
            }
        }

        Ok(())
    }
}

/// Sets `state.schema.{query,mutation,subscription}Type` when `type_name` is a root type
/// (shared by `ObjectTypeDefinition` and `ObjectTypeExtension`).
fn set_schema_root_type(b: &mut SubgraphStateBuilder, type_name: &str) {
    // Set query, mutation or subscription types as found in the schema
    if type_name == b.root_type_names.query {
        let renamed = b.root_type_names.rename_object_type(type_name);
        b.state.schema.set(RootTypeKind::QueryType, renamed.to_string());
    } else if type_name == b.root_type_names.mutation {
        let renamed = b.root_type_names.rename_object_type(type_name);
        b.state.schema.set(RootTypeKind::MutationType, renamed.to_string());
    } else if type_name == b.root_type_names.subscription {
        let renamed = b.root_type_names.rename_object_type(type_name);
        b.state.schema.set(RootTypeKind::SubscriptionType, renamed.to_string());
    }
}

/// `ObjectTypeDefinition(node)`.
fn object_type_definition(node: &ObjectTypeDefinition, b: &mut SubgraphStateBuilder) -> JsResult<()> {
    let type_name = node.name.value.as_str();

    if has_interface_object_directive(&node.directives) {
        b.interface_type().set_definition(type_name)?;
        b.interface_type().set_interface_object(type_name)?;

        for interface_node in &node.interfaces {
            b.interface_type()
                .set_interface(type_name, &interface_node.name.value)?;
        }
        return Ok(());
    }

    b.object_type().set_definition(type_name)?;

    set_schema_root_type(b, type_name);

    for interface_node in &node.interfaces {
        b.object_type()
            .set_interface(type_name, &interface_node.name.value)?;
    }

    Ok(())
}

/// `ObjectTypeExtension(node)`.
fn object_type_extension(node: &ObjectTypeExtension, b: &mut SubgraphStateBuilder) -> JsResult<()> {
    let type_name = node.name.value.as_str();

    // NOTE(port): the TS has an empty `if (hasInterfaceObjectDirective(node)) {}` here.

    set_schema_root_type(b, type_name);

    b.object_type().set_extension(type_name, ExtensionType::Extend)?;

    for interface_node in &node.interfaces {
        b.object_type()
            .set_interface(type_name, &interface_node.name.value)?;
    }

    Ok(())
}

/// `InterfaceTypeDefinition(node)`.
fn interface_type_definition(
    node: &InterfaceTypeDefinition,
    b: &mut SubgraphStateBuilder,
) -> JsResult<()> {
    let type_name = node.name.value.as_str();
    b.interface_type().set_definition(type_name)?;

    for interface_node in &node.interfaces {
        b.interface_type()
            .set_interface(type_name, &interface_node.name.value)?;
    }

    Ok(())
}

/// `InterfaceTypeExtension(node)`.
fn interface_type_extension(
    node: &InterfaceTypeExtension,
    b: &mut SubgraphStateBuilder,
) -> JsResult<()> {
    let type_name = node.name.value.as_str();
    if b.state.graph.version != FederationVersion::V1_0 {
        b.interface_type().set_extension(type_name)?;
    }

    for interface_node in &node.interfaces {
        b.interface_type()
            .set_interface(type_name, &interface_node.name.value)?;
    }

    Ok(())
}

/// `UnionTypeDefinition(node)`.
fn union_type_definition(node: &UnionTypeDefinition, b: &mut SubgraphStateBuilder) -> JsResult<()> {
    let type_name = node.name.value.as_str();
    b.union_type().set_definition(type_name)?;
    for member in &node.types {
        b.union_type().set_member(type_name, &member.name.value)?;
    }
    Ok(())
}

/// `UnionTypeExtension(node)`.
fn union_type_extension(node: &UnionTypeExtension, b: &mut SubgraphStateBuilder) -> JsResult<()> {
    let type_name = node.name.value.as_str();
    for member in &node.types {
        b.union_type().set_member(type_name, &member.name.value)?;
    }
    Ok(())
}

/// `EnumValueDefinition(node)`.
fn enum_value_definition(
    node: &EnumValueDefinition,
    b: &mut SubgraphStateBuilder,
    type_node_info: &TypeNodeInfo<'_>,
) -> JsResult<()> {
    let Some(type_def) = type_node_info.get_type_def() else {
        return Ok(());
    };

    b.enum_type()
        .value()
        .set_value(node_name(type_def), &node.name.value)
}

/// `ScalarTypeDefinition(node)`.
fn scalar_type_definition(node: &ScalarTypeDefinition, b: &mut SubgraphStateBuilder) -> JsResult<()> {
    if !b.specified_scalars.iter().any(|s| *s == node.name.value) {
        b.scalar_type().set_definition(&node.name.value)?;
    }
    Ok(())
}

/// `Directive(node)`.
fn directive(
    node: &DirectiveNode,
    b: &mut SubgraphStateBuilder,
    type_node_info: &TypeNodeInfo<'_>,
) -> JsResult<()> {
    if b.composed_directives.contains(&node.name.value) {
        let type_def = type_node_info.get_type_def();
        let field_def = type_node_info.get_field_def();
        let enum_value_def = type_node_info.get_value_def();
        let arg_def = type_node_info.get_argument_def();

        let Some(type_def) = type_def else {
            return Ok(());
        };
        let type_name = node_name(type_def);

        match type_def {
            NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_) => {
                if let Some(arg_def) = arg_def {
                    let field_name = asserted_field_name(field_def)?;
                    b.object_type().field().arg().set_directive(
                        type_name,
                        field_name,
                        &arg_def.name.value,
                        node,
                    )?;
                } else if let Some(field_def) = field_def {
                    b.object_type()
                        .field()
                        .set_directive(type_name, node_name(field_def), node)?;
                } else {
                    b.object_type().set_directive(type_name, node)?;
                }
            }
            NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_) => {
                if let Some(arg_def) = arg_def {
                    let field_name = asserted_field_name(field_def)?;
                    b.interface_type().field().arg().set_directive(
                        type_name,
                        field_name,
                        &arg_def.name.value,
                        node,
                    )?;
                } else if let Some(field_def) = field_def {
                    b.interface_type()
                        .field()
                        .set_directive(type_name, node_name(field_def), node)?;
                } else {
                    b.interface_type().set_directive(type_name, node)?;
                }
            }
            NodeRef::ScalarTypeDefinition(_) | NodeRef::ScalarTypeExtension(_) => {
                b.scalar_type().set_directive(type_name, node)?;
            }
            NodeRef::DirectiveDefinition(_) => {
                if let Some(field_def) = field_def {
                    b.directive()
                        .arg()
                        .set_directive(type_name, node_name(field_def), node)?;
                }
            }
            NodeRef::InputObjectTypeDefinition(_) | NodeRef::InputObjectTypeExtension(_) => {
                if let Some(field_def) = field_def {
                    b.input_object_type()
                        .field()
                        .set_directive(type_name, node_name(field_def), node)?;
                } else {
                    b.input_object_type().set_directive(type_name, node)?;
                }
            }
            NodeRef::EnumTypeDefinition(_) | NodeRef::EnumTypeExtension(_) => {
                if let Some(enum_value_def) = enum_value_def {
                    b.enum_type().value().set_directive(
                        type_name,
                        &enum_value_def.name.value,
                        node,
                    )?;
                } else {
                    b.enum_type().set_directive(type_name, node)?;
                }
            }
            NodeRef::UnionTypeDefinition(_) | NodeRef::UnionTypeExtension(_) => {
                b.union_type().set_directive(type_name, node)?;
            }
            other => {
                return Err(JsError::error(format!(
                    "Directives on \"{}\" types are not supported yet",
                    other.kind().as_str()
                )));
            }
        }
    } else if node.name.value == "specifiedBy" {
        let type_def = type_node_info.get_type_def();
        if let Some(
            type_def @ (NodeRef::ScalarTypeDefinition(_) | NodeRef::ScalarTypeExtension(_)),
        ) = type_def
        {
            if let Some(url) = find_string_argument(node, "url") {
                b.scalar_type().set_specified_by(node_name(type_def), url)?;
            }
        }
    } else if node.name.value == "deprecated" {
        let type_def = type_node_info.get_type_def();
        let field_def = type_node_info.get_field_def();
        let arg_def = type_node_info.get_argument_def();

        let Some(type_def) = type_def else {
            return Ok(());
        };
        let type_name = node_name(type_def);

        let reason = find_string_argument(node, "reason");

        match type_def {
            NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_) => {
                let Some(field_def) = field_def else {
                    return Ok(());
                };

                if let Some(arg_def) = arg_def {
                    b.object_type().field().arg().set_deprecated(
                        type_name,
                        node_name(field_def),
                        &arg_def.name.value,
                        reason,
                    )?;
                } else {
                    b.object_type()
                        .field()
                        .set_deprecated(type_name, node_name(field_def), reason)?;
                }
            }
            NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_) => {
                let Some(field_def) = field_def else {
                    return Ok(());
                };

                if let Some(arg_def) = arg_def {
                    b.interface_type().field().arg().set_deprecated(
                        type_name,
                        node_name(field_def),
                        &arg_def.name.value,
                        reason,
                    )?;
                } else {
                    b.interface_type()
                        .field()
                        .set_deprecated(type_name, node_name(field_def), reason)?;
                }
            }
            NodeRef::EnumTypeDefinition(_) | NodeRef::EnumTypeExtension(_) => {
                let Some(value_def) = type_node_info.get_value_def() else {
                    return Ok(());
                };

                b.enum_type()
                    .value()
                    .set_deprecated(type_name, &value_def.name.value, reason)?;
            }
            NodeRef::InputObjectTypeDefinition(_) | NodeRef::InputObjectTypeExtension(_) => {
                let Some(field_def) = field_def else {
                    return Ok(());
                };

                b.input_object_type()
                    .field()
                    .set_deprecated(type_name, node_name(field_def), reason)?;
            }
            _ => {}
        }
    } else if node.name.value == "oneOf" {
        let type_def = type_node_info.get_type_def();
        if let Some(
            type_def @ (NodeRef::InputObjectTypeDefinition(_)
            | NodeRef::InputObjectTypeExtension(_)),
        ) = type_def
        {
            b.input_object_type().set_one_of(node_name(type_def))?;
        }
    }

    Ok(())
}

/// `DirectiveDefinition(node)`.
fn directive_definition(node: &DirectiveDefinition, b: &mut SubgraphStateBuilder) -> JsResult<()> {
    let directive_name = node.name.value.as_str();

    if node.repeatable {
        b.directive().set_repeatable(directive_name)?;
    }

    for location in &node.locations {
        b.directive().set_location(directive_name, &location.value)?;
    }

    for arg in &node.arguments {
        b.directive()
            .arg()
            .set_type(directive_name, &arg.name.value, &print_output_type(&arg.ty))?;

        let output_type_name = resolve_type_name(&arg.ty);

        let kind = b.resolve_argument_kind(output_type_name);
        b.directive()
            .arg()
            .set_kind(directive_name, &arg.name.value, kind)?;

        if let Some(default_value) = &arg.default_value {
            b.directive().arg().set_default_value(
                directive_name,
                &arg.name.value,
                &print(default_value),
            )?;
        }
    }

    Ok(())
}

/// `StringValue(node, _, parent)`.
fn string_value<'a>(
    node: &'a StringValue,
    b: &mut SubgraphStateBuilder,
    type_node_info: &TypeNodeInfo<'a>,
    info: &VisitInfo<'a, '_>,
) -> JsResult<()> {
    let type_def = type_node_info.get_type_def();
    let field_def = type_node_info.get_field_def();
    let arg_def = type_node_info.get_argument_def();

    let description = Description {
        value: node.value.clone(),
        block: node.block,
    };

    // `parent && "kind" in parent`: the parent is a node, not a list.
    let parent: Option<NodeRef<'a>> = info.parent.and_then(Ancestor::as_node);

    if let Some(parent) = parent {
        if let NodeRef::ScalarTypeDefinition(_) | NodeRef::ScalarTypeExtension(_) = parent {
            b.scalar_type()
                .set_description(node_name(parent), description.clone())?;
        }

        if let NodeRef::InputValueDefinition(parent) = parent {
            if let Some(Value::String(default_value)) = &parent.default_value {
                if std::ptr::eq(default_value, node) {
                    // It seems like StringValue comes from a default value. Ignore it.
                    return Ok(());
                }
            }
        }
    }

    let Some(type_def) = type_def else {
        return Ok(());
    };
    let type_name = node_name(type_def);

    // `node === parent`.
    let matches_parent = |node: NodeRef<'_>| parent.is_some_and(|parent| parent.ptr_eq(node));

    match type_def {
        NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_) => {
            if let Some(arg_def) = arg_def.filter(|a| matches_parent(NodeRef::InputValueDefinition(a))) {
                let field_name = asserted_field_name(field_def)?;
                b.object_type().field().arg().set_description(
                    type_name,
                    field_name,
                    &arg_def.name.value,
                    description,
                )?;
            } else if let Some(field_def) = field_def.filter(|f| matches_parent(*f)) {
                b.object_type()
                    .field()
                    .set_description(type_name, node_name(field_def), description)?;
            } else if matches_parent(type_def) {
                b.object_type().set_description(type_name, description)?;
            }
        }
        NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_) => {
            if let Some(arg_def) = arg_def.filter(|a| matches_parent(NodeRef::InputValueDefinition(a))) {
                let field_name = asserted_field_name(field_def)?;
                b.interface_type().field().arg().set_description(
                    type_name,
                    field_name,
                    &arg_def.name.value,
                    description,
                )?;
            } else if let Some(field_def) = field_def.filter(|f| matches_parent(*f)) {
                b.interface_type()
                    .field()
                    .set_description(type_name, node_name(field_def), description)?;
            } else if matches_parent(type_def) {
                b.interface_type().set_description(type_name, description)?;
            }
        }
        NodeRef::InputObjectTypeDefinition(_) | NodeRef::InputObjectTypeExtension(_) => {
            if let Some(field_def) = field_def.filter(|f| matches_parent(*f)) {
                b.input_object_type()
                    .field()
                    .set_description(type_name, node_name(field_def), description)?;
            } else if matches_parent(type_def) {
                b.input_object_type().set_description(type_name, description)?;
            }
        }
        NodeRef::EnumTypeDefinition(_) | NodeRef::EnumTypeExtension(_) => {
            if let Some(NodeRef::EnumValueDefinition(parent)) = parent {
                b.enum_type()
                    .value()
                    .set_description(type_name, &parent.name.value, description)?;
            } else if matches_parent(type_def) {
                b.enum_type().set_description(type_name, description)?;
            }
        }
        NodeRef::UnionTypeDefinition(_) | NodeRef::UnionTypeExtension(_) => {
            if matches_parent(type_def) {
                b.union_type().set_description(type_name, description)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// `directiveFactory(state)`.
pub struct DirectiveBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> DirectiveBuilder<'s> {
    pub fn set_composed(self, directive_name: &str) -> JsResult<()> {
        get_or_create_directive(self.state, directive_name)?.composed = true;
        Ok(())
    }
    pub fn set_location(self, directive_name: &str, location: &str) -> JsResult<()> {
        get_or_create_directive(self.state, directive_name)?
            .locations
            .insert(location.to_string());
        Ok(())
    }
    pub fn set_repeatable(self, directive_name: &str) -> JsResult<()> {
        get_or_create_directive(self.state, directive_name)?.repeatable = true;
        Ok(())
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
        get_or_create_directive_arg(self.state, directive_name, arg_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_type(self, directive_name: &str, arg_name: &str, arg_type: &str) -> JsResult<()> {
        get_or_create_directive_arg(self.state, directive_name, arg_name)?.r#type =
            arg_type.to_string();
        Ok(())
    }
    pub fn set_kind(self, directive_name: &str, arg_name: &str, arg_kind: ArgumentKind) -> JsResult<()> {
        get_or_create_directive_arg(self.state, directive_name, arg_name)?.kind = arg_kind;
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, arg_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        get_or_create_directive_arg(self.state, type_name, arg_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_default_value(self, type_name: &str, arg_name: &str, default_value: &str) -> JsResult<()> {
        get_or_create_directive_arg(self.state, type_name, arg_name)?.default_value =
            Some(default_value.to_string());
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str, arg_name: &str) -> JsResult<()> {
        get_or_create_directive_arg(self.state, type_name, arg_name)?.inaccessible = true;
        Ok(())
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
        get_or_create_scalar_type(self.state, type_name)?;
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        get_or_create_scalar_type(self.state, type_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_authenticated(self, type_name: &str) -> JsResult<()> {
        get_or_create_scalar_type(self.state, type_name)?.authenticated = true;
        Ok(())
    }
    pub fn set_policies(self, type_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        let existing = get_or_create_scalar_type(self.state, type_name)?;
        existing.policies = merge_scope_policies(&existing.policies, policies);
        Ok(())
    }
    pub fn set_scopes(self, type_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        let existing = get_or_create_scalar_type(self.state, type_name)?;
        existing.scopes = merge_scope_policies(&existing.scopes, scopes);
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, cost: f64) -> JsResult<()> {
        get_or_create_scalar_type(self.state, type_name)?.cost = Some(cost);
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        get_or_create_scalar_type(self.state, type_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        get_or_create_scalar_type(self.state, type_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        get_or_create_scalar_type(self.state, type_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_specified_by(self, type_name: &str, url: &str) -> JsResult<()> {
        get_or_create_scalar_type(self.state, type_name)?.specified_by = Some(url.to_string());
        Ok(())
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

    fn is_interface_object(&self, type_name: &str) -> bool {
        is_interface_object(self.state, type_name)
    }

    pub fn set_definition(mut self, type_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_type_builder().set_definition(type_name);
        }

        get_or_create_object_type(self.state, self.roots, type_name)?.is_definition = true;
        Ok(())
    }
    pub fn set_extension(mut self, type_name: &str, extension_type: ExtensionType) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_type_builder().set_extension(type_name);
        }

        let object_type = get_or_create_object_type(self.state, self.roots, type_name)?;
        object_type.extension = true;
        if object_type.extension_type != Some(ExtensionType::Extends) {
            object_type.extension_type = Some(extension_type);
        }
        Ok(())
    }
    pub fn set_description(mut self, type_name: &str, description: Description) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_type_builder()
                .set_description(type_name, description);
        }
        get_or_create_object_type(self.state, self.roots, type_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_external(self, type_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return Ok(());
        }

        let object_type = get_or_create_object_type(self.state, self.roots, type_name)?;
        object_type.external = true;

        for field in object_type.fields.values_mut() {
            field.external = true;
        }
        Ok(())
    }
    pub fn set_interface(mut self, type_name: &str, interface_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_type_builder()
                .set_interface(type_name, interface_name);
        }

        get_or_create_object_type(self.state, self.roots, type_name)?
            .interfaces
            .insert(interface_name.to_string());
        get_or_create_interface_type(self.state, interface_name)?
            .implemented_by
            .insert(type_name.to_string());
        Ok(())
    }
    pub fn set_key(
        mut self,
        type_name: &str,
        fields: &str,
        fields_used_in_key: &IndexSet<String>,
        resolvable: bool,
    ) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_type_builder().set_key(
                type_name,
                fields,
                fields_used_in_key,
                resolvable,
            );
        }
        let object_type = get_or_create_object_type(self.state, self.roots, type_name)?;
        object_type.keys.push(Key {
            fields: fields.to_string(),
            resolvable,
        });

        for field in fields_used_in_key {
            object_type.fields_used_as_keys.insert(field.clone());
        }
        Ok(())
    }
    pub fn set_inaccessible(mut self, type_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_type_builder().set_inaccessible(type_name);
        }

        let object_type = get_or_create_object_type(self.state, self.roots, type_name)?;
        object_type.inaccessible = true;
        Ok(())
    }
    pub fn set_authenticated(self, type_name: &str) -> JsResult<()> {
        // NOTE(port): the TS has the interface-object redirect commented out here.
        let object_type = get_or_create_object_type(self.state, self.roots, type_name)?;
        object_type.authenticated = true;
        Ok(())
    }
    pub fn set_policies(self, type_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        let existing = get_or_create_object_type(self.state, self.roots, type_name)?;
        existing.policies = merge_scope_policies(&existing.policies, policies);
        Ok(())
    }
    pub fn set_scopes(self, type_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        let existing = get_or_create_object_type(self.state, self.roots, type_name)?;
        existing.scopes = merge_scope_policies(&existing.scopes, scopes);
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, cost: f64) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            // type Foo @cost(weight: 420) @interfaceObject @key(fields: "id")
            // is not allowed.
            // The validation rules are detecting this as an error.
            return Ok(());
        }

        get_or_create_object_type(self.state, self.roots, type_name)?.cost = Some(cost);
        Ok(())
    }
    pub fn set_shareable(self, type_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return Ok(());
        }

        get_or_create_object_type(self.state, self.roots, type_name)?.shareable = true;
        Ok(())
    }
    pub fn set_tag(mut self, type_name: &str, tag: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_type_builder().set_tag(type_name, tag);
        }

        get_or_create_object_type(self.state, self.roots, type_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_directive(mut self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_type_builder()
                .set_directive(type_name, directive);
        }

        get_or_create_object_type(self.state, self.roots, type_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn add_context(mut self, type_name: &str, context_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_type_builder()
                .add_context(type_name, context_name);
        }

        get_or_create_object_type(self.state, self.roots, type_name)?
            .contexts
            .insert(context_name.to_string());
        Ok(())
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
    fn is_interface_object(&self, type_name: &str) -> bool {
        is_interface_object(self.state, type_name)
    }

    /// `interfaceTypeBuilder.field`.
    fn interface_field(self) -> InterfaceTypeFieldBuilder<'s> {
        InterfaceTypeFieldBuilder { state: self.state }
    }

    fn field(self, type_name: &str, field_name: &str) -> JsResult<&'s mut Field> {
        get_or_create_object_field(self.state, self.roots, type_name, field_name)
    }

    pub fn set_type(self, type_name: &str, field_name: &str, field_type: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_type(type_name, field_name, field_type);
        }

        self.field(type_name, field_name)?.r#type = field_type.to_string();
        Ok(())
    }
    pub fn set_leaf(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_field().set_leaf(type_name, field_name);
        }

        self.field(type_name, field_name)?.is_leaf = true;
        Ok(())
    }
    pub fn set_extension(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return Ok(());
        }

        self.field(type_name, field_name)?.extension = true;
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, field_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_directive(type_name, field_name, directive);
        }

        self.field(type_name, field_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_description(self, type_name: &str, field_name: &str, description: Description) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_description(type_name, field_name, description);
        }

        self.field(type_name, field_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, reason: Option<&str>) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_deprecated(type_name, field_name, reason);
        }

        self.field(type_name, field_name)?.deprecated = Some(Deprecated {
            reason: reason.map(str::to_string),
        });
        Ok(())
    }
    pub fn set_authenticated(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_authenticated(type_name, field_name);
        }

        self.field(type_name, field_name)?.authenticated = true;
        Ok(())
    }
    pub fn set_policies(self, type_name: &str, field_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_policies(type_name, field_name, policies);
        }

        let existing = self.field(type_name, field_name)?;

        existing.policies = merge_scope_policies(&existing.policies, policies);
        Ok(())
    }
    pub fn set_scopes(self, type_name: &str, field_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_scopes(type_name, field_name, scopes);
        }

        let existing = self.field(type_name, field_name)?;

        existing.scopes = merge_scope_policies(&existing.scopes, scopes);
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, cost: f64) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_field().set_cost(type_name, field_name, cost);
        }

        self.field(type_name, field_name)?.cost = Some(cost);
        Ok(())
    }
    pub fn set_list_size(self, type_name: &str, field_name: &str, list_size: ListSize) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_list_size(type_name, field_name, list_size);
        }

        self.field(type_name, field_name)?.list_size = Some(list_size);
        Ok(())
    }
    pub fn set_external(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_field().set_external(type_name, field_name);
        }

        self.field(type_name, field_name)?.external = true;
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_inaccessible(type_name, field_name);
        }

        self.field(type_name, field_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_override(
        self,
        type_name: &str,
        field_name: &str,
        r#override: &str,
        label: Option<&str>,
    ) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_override(type_name, field_name, r#override, label);
        }

        let field = self.field(type_name, field_name)?;
        field.r#override = Some(r#override.to_string());
        field.override_label = label.map(str::to_string);
        Ok(())
    }
    pub fn set_provides(self, type_name: &str, field_name: &str, provides: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return Ok(());
        }

        self.field(type_name, field_name)?.provides = Some(provides.to_string());
        Ok(())
    }
    pub fn set_requires(self, type_name: &str, field_name: &str, requires: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_field()
                .set_requires(type_name, field_name, requires);
        }

        self.field(type_name, field_name)?.requires = Some(requires.to_string());
        Ok(())
    }
    pub fn mark_as_provided(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return Ok(());
        }

        self.field(type_name, field_name)?.provided = true;
        Ok(())
    }
    pub fn marked_as_required(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return Ok(());
        }

        self.field(type_name, field_name)?.required = true;
        Ok(())
    }
    pub fn set_shareable(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_field().set_shareable(type_name, field_name);
        }

        self.field(type_name, field_name)?.shareable = true;
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, tag: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_field().set_tag(type_name, field_name, tag);
        }

        self.field(type_name, field_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_used(self, type_name: &str, field_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self.interface_field().set_used(type_name, field_name);
        }

        self.field(type_name, field_name)?.used = true;
        Ok(())
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

impl<'s> ObjectTypeFieldArgBuilder<'s> {
    fn is_interface_object(&self, type_name: &str) -> bool {
        is_interface_object(self.state, type_name)
    }

    /// `interfaceTypeBuilder.field.arg`.
    fn interface_arg(self) -> InterfaceTypeFieldArgBuilder<'s> {
        InterfaceTypeFieldArgBuilder { state: self.state }
    }

    fn arg(self, type_name: &str, field_name: &str, arg_name: &str) -> JsResult<&'s mut Argument> {
        get_or_create_object_field_argument(self.state, self.roots, type_name, field_name, arg_name)
    }

    pub fn set_type(self, type_name: &str, field_name: &str, arg_name: &str, arg_type: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_type(type_name, field_name, arg_name, arg_type);
        }

        self.arg(type_name, field_name, arg_name)?.r#type = arg_type.to_string();
        Ok(())
    }
    pub fn set_kind(self, type_name: &str, field_name: &str, arg_name: &str, arg_kind: ArgumentKind) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_kind(type_name, field_name, arg_name, arg_kind);
        }

        self.arg(type_name, field_name, arg_name)?.kind = arg_kind;
        Ok(())
    }
    pub fn set_description(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        description: Description,
    ) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_description(type_name, field_name, arg_name, description);
        }

        self.arg(type_name, field_name, arg_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, arg_name: &str, reason: Option<&str>) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_deprecated(type_name, field_name, arg_name, reason);
        }

        self.arg(type_name, field_name, arg_name)?.deprecated = Some(Deprecated {
            reason: reason.map(str::to_string),
        });
        Ok(())
    }
    pub fn set_directive(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        directive: &DirectiveNode,
    ) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_directive(type_name, field_name, arg_name, directive);
        }

        self.arg(type_name, field_name, arg_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_default_value(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        default_value: &str,
    ) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_default_value(type_name, field_name, arg_name, default_value);
        }

        self.arg(type_name, field_name, arg_name)?.default_value = Some(default_value.to_string());
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str, arg_name: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_inaccessible(type_name, field_name, arg_name);
        }

        self.arg(type_name, field_name, arg_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_from_context(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        from_context: FromContext,
    ) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_from_context(type_name, field_name, arg_name, from_context);
        }

        self.arg(type_name, field_name, arg_name)?.from_context = Some(from_context);
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, arg_name: &str, tag: &str) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_tag(type_name, field_name, arg_name, tag);
        }

        self.arg(type_name, field_name, arg_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, arg_name: &str, cost: f64) -> JsResult<()> {
        if self.is_interface_object(type_name) {
            return self
                .interface_arg()
                .set_cost(type_name, field_name, arg_name, cost);
        }

        self.arg(type_name, field_name, arg_name)?.cost = Some(cost);
        Ok(())
    }
}

/// `interfaceTypeFactory(state)`.
pub struct InterfaceTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> InterfaceTypeBuilder<'s> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?.is_definition = true;
        Ok(())
    }
    /// For Federation v1: set as extension only if it uses @extends directive (not for
    /// `extend type` syntax). For Federation v2: set as extension if it's not a definition.
    pub fn set_extension(self, type_name: &str) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?.extension = true;
        Ok(())
    }
    pub fn set_interface(self, type_name: &str, interface_name: &str) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?
            .interfaces
            .insert(interface_name.to_string());
        Ok(())
    }
    pub fn set_interface_object(self, type_name: &str) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?.is_interface_object = true;
        Ok(())
    }
    pub fn set_key(
        self,
        type_name: &str,
        fields: &str,
        fields_used_in_key: &IndexSet<String>,
        resolvable: bool,
    ) -> JsResult<()> {
        let interface_type = get_or_create_interface_type(self.state, type_name)?;
        interface_type.keys.push(Key {
            fields: fields.to_string(),
            resolvable,
        });

        for field in fields_used_in_key {
            interface_type.fields_used_as_keys.insert(field.clone());
        }
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        let object_type = get_or_create_interface_type(self.state, type_name)?;
        object_type.inaccessible = true;
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn add_context(self, type_name: &str, context_name: &str) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?
            .contexts
            .insert(context_name.to_string());
        Ok(())
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        get_or_create_interface_type(self.state, type_name)?.description = Some(description);
        Ok(())
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
    fn field(self, type_name: &str, field_name: &str) -> JsResult<&'s mut Field> {
        get_or_create_interface_field(self.state, type_name, field_name)
    }

    pub fn set_type(self, type_name: &str, field_name: &str, field_type: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.r#type = field_type.to_string();
        Ok(())
    }
    pub fn set_leaf(self, type_name: &str, field_name: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.is_leaf = true;
        Ok(())
    }
    pub fn set_external(self, type_name: &str, field_name: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.external = true;
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_authenticated(self, type_name: &str, field_name: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.authenticated = true;
        Ok(())
    }
    pub fn set_policies(self, type_name: &str, field_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        let existing = self.field(type_name, field_name)?;
        existing.policies = merge_scope_policies(&existing.policies, policies);
        Ok(())
    }
    pub fn set_scopes(self, type_name: &str, field_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        let existing = self.field(type_name, field_name)?;
        existing.scopes = merge_scope_policies(&existing.scopes, scopes);
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, cost: f64) -> JsResult<()> {
        self.field(type_name, field_name)?.cost = Some(cost);
        Ok(())
    }
    pub fn set_list_size(self, type_name: &str, field_name: &str, list_size: ListSize) -> JsResult<()> {
        self.field(type_name, field_name)?.list_size = Some(list_size);
        Ok(())
    }
    pub fn set_override(
        self,
        type_name: &str,
        field_name: &str,
        r#override: &str,
        label: Option<&str>,
    ) -> JsResult<()> {
        let field = self.field(type_name, field_name)?;
        field.r#override = Some(r#override.to_string());
        field.override_label = label.map(str::to_string);
        Ok(())
    }
    pub fn set_requires(self, type_name: &str, field_name: &str, requires: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.requires = Some(requires.to_string());
        Ok(())
    }
    pub fn set_shareable(self, type_name: &str, field_name: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.shareable = true;
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, tag: &str) -> JsResult<()> {
        self.field(type_name, field_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_used(self, type_name: &str, field_name: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.used = true;
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, field_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        self.field(type_name, field_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_description(self, type_name: &str, field_name: &str, description: Description) -> JsResult<()> {
        self.field(type_name, field_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, reason: Option<&str>) -> JsResult<()> {
        self.field(type_name, field_name)?.deprecated = Some(Deprecated {
            reason: reason.map(str::to_string),
        });
        Ok(())
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

impl<'s> InterfaceTypeFieldArgBuilder<'s> {
    fn arg(self, type_name: &str, field_name: &str, arg_name: &str) -> JsResult<&'s mut Argument> {
        get_or_create_interface_field_argument(self.state, type_name, field_name, arg_name)
    }

    pub fn set_type(self, type_name: &str, field_name: &str, arg_name: &str, arg_type: &str) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.r#type = arg_type.to_string();
        Ok(())
    }
    pub fn set_kind(self, type_name: &str, field_name: &str, arg_name: &str, arg_kind: ArgumentKind) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.kind = arg_kind;
        Ok(())
    }
    pub fn set_default_value(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        default_value: &str,
    ) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.default_value = Some(default_value.to_string());
        Ok(())
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, arg_name: &str, reason: Option<&str>) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.deprecated = Some(Deprecated {
            reason: reason.map(str::to_string),
        });
        Ok(())
    }
    pub fn set_description(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        description: Description,
    ) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, arg_name: &str, tag: &str) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str, arg_name: &str) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, arg_name: &str, cost: f64) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.cost = Some(cost);
        Ok(())
    }
    pub fn set_directive(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        directive: &DirectiveNode,
    ) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_from_context(
        self,
        type_name: &str,
        field_name: &str,
        arg_name: &str,
        from_context: FromContext,
    ) -> JsResult<()> {
        self.arg(type_name, field_name, arg_name)?.from_context = Some(from_context);
        Ok(())
    }
}

/// `inputObjectTypeFactory(state)`.
pub struct InputObjectTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> InputObjectTypeBuilder<'s> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        get_or_create_input_object_type(self.state, type_name)?.is_definition = true;
        Ok(())
    }
    pub fn set_extension(self, type_name: &str) -> JsResult<()> {
        get_or_create_input_object_type(self.state, type_name)?.extension = true;
        Ok(())
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        get_or_create_input_object_type(self.state, type_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        let input_object_type = get_or_create_input_object_type(self.state, type_name)?;
        input_object_type.inaccessible = true;

        for field in input_object_type.fields.values_mut() {
            field.inaccessible = true;
        }
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        get_or_create_input_object_type(self.state, type_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        get_or_create_input_object_type(self.state, type_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_one_of(self, type_name: &str) -> JsResult<()> {
        let input_object = DirectiveLocation::InputObject.as_str();

        match self.state.types.get_mut("oneOf") {
            // if definition matches the built-in, flag the type as isOneOf
            // and ensure the directive is composed
            Some(SubgraphType::Directive(existing_one_of))
                if existing_one_of.locations.len() == 1
                    && existing_one_of.locations.contains(input_object)
                    && !existing_one_of.repeatable
                    && existing_one_of.args.is_empty() =>
            {
                existing_one_of.composed = true; // make sure it's composed
                get_or_create_input_object_type(self.state, type_name)?.is_one_of = Some(true);
            }
            // If there is no existing definition, then this must be referencing the built-in.
            // Create the definition and flag the type as isOneOf
            None => {
                let one_of_directive = get_or_create_directive(self.state, "oneOf")?;
                one_of_directive.locations.insert(input_object.to_string());
                one_of_directive.composed = true; // force including in the composed output

                get_or_create_input_object_type(self.state, type_name)?.is_one_of = Some(true);
            }
            // otherwise let composeDirective figure it out
            _ => {}
        }
        Ok(())
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

impl<'s> InputObjectTypeFieldBuilder<'s> {
    fn field(self, type_name: &str, field_name: &str) -> JsResult<&'s mut InputField> {
        get_or_create_input_object_field(self.state, type_name, field_name)
    }

    pub fn set_type(self, type_name: &str, field_name: &str, field_type: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.r#type = field_type.to_string();
        Ok(())
    }
    pub fn set_kind(self, type_name: &str, field_name: &str, field_kind: ArgumentKind) -> JsResult<()> {
        self.field(type_name, field_name)?.kind = field_kind;
        Ok(())
    }
    pub fn set_description(self, type_name: &str, field_name: &str, description: Description) -> JsResult<()> {
        self.field(type_name, field_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_deprecated(self, type_name: &str, field_name: &str, reason: Option<&str>) -> JsResult<()> {
        self.field(type_name, field_name)?.deprecated = Some(Deprecated {
            reason: reason.map(str::to_string),
        });
        Ok(())
    }
    pub fn set_default_value(self, type_name: &str, field_name: &str, default_value: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.default_value = Some(default_value.to_string());
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str, field_name: &str) -> JsResult<()> {
        self.field(type_name, field_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, field_name: &str, tag: &str) -> JsResult<()> {
        self.field(type_name, field_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, field_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        self.field(type_name, field_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, field_name: &str, cost: f64) -> JsResult<()> {
        self.field(type_name, field_name)?.cost = Some(cost);
        Ok(())
    }
}

/// `unionTypeFactory(state)`.
pub struct UnionTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl UnionTypeBuilder<'_> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        get_or_create_union_type(self.state, type_name)?.is_definition = true;
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        get_or_create_union_type(self.state, type_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        get_or_create_union_type(self.state, type_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        get_or_create_union_type(self.state, type_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_member(self, type_name: &str, member: &str) -> JsResult<()> {
        get_or_create_union_type(self.state, type_name)?
            .members
            .insert(member.to_string());
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        get_or_create_union_type(self.state, type_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn add_context(self, type_name: &str, context_name: &str) -> JsResult<()> {
        get_or_create_union_type(self.state, type_name)?
            .contexts
            .insert(context_name.to_string());
        Ok(())
    }
}

/// `enumTypeFactory(state)`.
pub struct EnumTypeBuilder<'s> {
    state: &'s mut SubgraphState,
}

impl<'s> EnumTypeBuilder<'s> {
    pub fn set_definition(self, type_name: &str) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?.is_definition = true;
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_authenticated(self, type_name: &str) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?.authenticated = true;
        Ok(())
    }
    pub fn set_policies(self, type_name: &str, policies: &[Vec<String>]) -> JsResult<()> {
        let existing = get_or_create_enum_type(self.state, type_name)?;
        existing.policies = merge_scope_policies(&existing.policies, policies);
        Ok(())
    }
    pub fn set_scopes(self, type_name: &str, scopes: &[Vec<String>]) -> JsResult<()> {
        let existing = get_or_create_enum_type(self.state, type_name)?;
        existing.scopes = merge_scope_policies(&existing.scopes, scopes);
        Ok(())
    }
    pub fn set_cost(self, type_name: &str, cost: f64) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?.cost = Some(cost);
        Ok(())
    }
    pub fn set_description(self, type_name: &str, description: Description) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, tag: &str) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_referenced_by_input_type(self, type_name: &str, schema_coordinate: &str) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?.referenced_by_input_type = true;
        get_or_create_enum_type(self.state, type_name)?
            .input_type_references
            .insert(schema_coordinate.to_string());
        Ok(())
    }
    pub fn set_referenced_by_output_type(self, type_name: &str, schema_coordinate: &str) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?.referenced_by_output_type = true;
        get_or_create_enum_type(self.state, type_name)?
            .output_type_references
            .insert(schema_coordinate.to_string());
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        get_or_create_enum_type(self.state, type_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
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

impl<'s> EnumTypeValueBuilder<'s> {
    fn value(self, type_name: &str, value_name: &str) -> JsResult<&'s mut EnumValue> {
        get_or_create_enum_value(self.state, type_name, value_name)
    }

    pub fn set_value(self, type_name: &str, value_name: &str) -> JsResult<()> {
        self.value(type_name, value_name)?;
        Ok(())
    }
    pub fn set_description(self, type_name: &str, value_name: &str, description: Description) -> JsResult<()> {
        self.value(type_name, value_name)?.description = Some(description);
        Ok(())
    }
    pub fn set_directive(self, type_name: &str, value_name: &str, directive: &DirectiveNode) -> JsResult<()> {
        self.value(type_name, value_name)?
            .ast
            .directives
            .push(directive.clone());
        Ok(())
    }
    pub fn set_inaccessible(self, type_name: &str, value_name: &str) -> JsResult<()> {
        self.value(type_name, value_name)?.inaccessible = true;
        Ok(())
    }
    pub fn set_tag(self, type_name: &str, value_name: &str, tag: &str) -> JsResult<()> {
        self.value(type_name, value_name)?
            .tags
            .insert(tag.to_string());
        Ok(())
    }
    pub fn set_deprecated(self, type_name: &str, value_name: &str, reason: Option<&str>) -> JsResult<()> {
        self.value(type_name, value_name)?.deprecated = Some(Deprecated {
            reason: reason.map(str::to_string),
        });
        Ok(())
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

/// The shared shape of the `getOrCreate*Type` helpers: an existing entry of another kind throws
/// `message`; a missing one is created with `create` (appended, like `Map.set`).
fn get_or_create_type<'s, T>(
    state: &'s mut SubgraphState,
    type_name: &str,
    create: impl FnOnce() -> SubgraphType,
    as_kind: impl FnOnce(&'s mut SubgraphType) -> Option<&'s mut T>,
    message: impl FnOnce() -> String,
) -> JsResult<&'s mut T> {
    let existing = get_or_insert_with(&mut state.types, type_name, create);
    as_kind(existing).ok_or_else(|| JsError::error(message()))
}

/// `map.get(key)`, inserting `create()` under `key` first when it is missing.
/// PERF: the TS `if (!map.has(key)) map.set(key, ...); map.get(key)`, and the port's
/// `entry(key.to_string()).or_insert_with(..)`, done with one lookup when the key is present
/// and without allocating the key then. Insertion order is the same.
fn get_or_insert_with<'m, V>(map: &'m mut IndexMap<String, V>, key: &str, create: impl FnOnce() -> V) -> &'m mut V {
    let index = match map.get_index_of(key) {
        Some(index) => index,
        None => map.insert_full(key.to_string(), create()).0,
    };
    &mut map[index]
}

fn get_or_create_directive<'s>(state: &'s mut SubgraphState, directive_name: &str) -> JsResult<&'s mut Directive> {
    get_or_create_type(
        state,
        directive_name,
        || {
            SubgraphType::Directive(Directive {
                name: directive_name.to_string(),
                locations: IndexSet::default(),
                repeatable: false,
                composed: false,
                args: IndexMap::default(),
            })
        },
        |existing| match existing {
            SubgraphType::Directive(d) => Some(d),
            _ => None,
        },
        || format!("Expected {directive_name} to be a directive"),
    )
}

/// A new `Argument` (field and directive arguments share the defaults).
fn new_argument(arg_name: &str) -> Argument {
    Argument {
        name: arg_name.to_string(),
        r#type: MISSING.to_string(),
        kind: ArgumentKind::Scalar,
        inaccessible: false,
        tags: IndexSet::default(),
        cost: None,
        default_value: None,
        from_context: None,
        description: None,
        deprecated: None,
        ast: Ast::default(),
    }
}

/// A new `Field` (object and interface fields share the defaults).
fn new_field(field_name: &str) -> Field {
    Field {
        name: field_name.to_string(),
        r#type: MISSING.to_string(),
        used_as_key: false,
        is_leaf: false,
        external: false,
        inaccessible: false,
        authenticated: false,
        extension: false,
        policies: Vec::new(),
        scopes: Vec::new(),
        cost: None,
        list_size: None,
        used: false,
        required: false,
        provided: false,
        r#override: None,
        override_label: None,
        provides: None,
        requires: None,
        shareable: false,
        // shareable: state.version === 'v1.0' ? true : false, // In Fed v1 - it should be true by default, as all fields are shareable by default, even though there's not @shareable directive
        tags: IndexSet::default(),
        args: IndexMap::default(),
        description: None,
        deprecated: None,
        ast: Ast::default(),
    }
}

fn get_or_create_directive_arg<'s>(
    state: &'s mut SubgraphState,
    directive_name: &str,
    arg_name: &str,
) -> JsResult<&'s mut Argument> {
    let directive = get_or_create_directive(state, directive_name)?;

    Ok(get_or_insert_with(&mut directive.args, arg_name, || new_argument(arg_name)))
}

fn get_or_create_scalar_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut ScalarType> {
    get_or_create_type(
        state,
        type_name,
        || {
            SubgraphType::Scalar(ScalarType {
                name: type_name.to_string(),
                tags: IndexSet::default(),
                inaccessible: false,
                authenticated: false,
                policies: Vec::new(),
                scopes: Vec::new(),
                cost: None,
                description: None,
                specified_by: None,
                ast: Ast::default(),
            })
        },
        |existing| match existing {
            SubgraphType::Scalar(t) => Some(t),
            _ => None,
        },
        || format!("Expected {type_name} to be a scalar type"),
    )
}

/// `getOrCreateObjectType(state, renameObject, typeName)`.
fn get_or_create_object_type<'s>(
    state: &'s mut SubgraphState,
    rename_object: &RootTypeNames,
    type_name: &str,
) -> JsResult<&'s mut ObjectType> {
    let type_name = rename_object.rename_object_type(type_name);
    get_or_create_type(
        state,
        type_name,
        || {
            SubgraphType::Object(ObjectType {
                name: type_name.to_string(),
                fields: IndexMap::default(),
                contexts: IndexSet::default(),
                fields_used_as_keys: IndexSet::default(),
                extension: false,
                extension_type: None,
                external: false,
                keys: Vec::new(),
                inaccessible: false,
                authenticated: false,
                policies: Vec::new(),
                scopes: Vec::new(),
                cost: None,
                shareable: false,
                tags: IndexSet::default(),
                interfaces: IndexSet::default(),
                is_definition: false,
                description: None,
                deprecated: None,
                ast: Ast::default(),
            })
        },
        |existing| match existing {
            SubgraphType::Object(t) => Some(t),
            _ => None,
        },
        || format!("Expected {type_name} to be an object type"),
    )
}

fn get_or_create_interface_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut InterfaceType> {
    get_or_create_type(
        state,
        type_name,
        || {
            SubgraphType::Interface(InterfaceType {
                name: type_name.to_string(),
                fields: IndexMap::default(),
                contexts: IndexSet::default(),
                fields_used_as_keys: IndexSet::default(),
                extension: false,
                keys: Vec::new(),
                inaccessible: false,
                tags: IndexSet::default(),
                interfaces: IndexSet::default(),
                implemented_by: IndexSet::default(),
                is_definition: false,
                is_interface_object: false,
                description: None,
                ast: Ast::default(),
            })
        },
        |existing| match existing {
            SubgraphType::Interface(t) => Some(t),
            _ => None,
        },
        || format!("Expected {type_name} to be an interface type"),
    )
}

fn get_or_create_input_object_type<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
) -> JsResult<&'s mut InputObjectType> {
    get_or_create_type(
        state,
        type_name,
        || {
            SubgraphType::InputObject(InputObjectType {
                name: type_name.to_string(),
                fields: IndexMap::default(),
                extension: false,
                inaccessible: false,
                tags: IndexSet::default(),
                is_definition: false,
                description: None,
                ast: Ast::default(),
                is_one_of: None,
            })
        },
        |existing| match existing {
            SubgraphType::InputObject(t) => Some(t),
            _ => None,
        },
        || format!("Expected {type_name} to be an input object type"),
    )
}

fn get_or_create_enum_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut EnumType> {
    get_or_create_type(
        state,
        type_name,
        || {
            SubgraphType::Enum(EnumType {
                name: type_name.to_string(),
                values: IndexMap::default(),
                inaccessible: false,
                authenticated: false,
                policies: Vec::new(),
                scopes: Vec::new(),
                cost: None,
                tags: IndexSet::default(),
                is_definition: false,
                description: None,
                referenced_by_input_type: false,
                referenced_by_output_type: false,
                input_type_references: IndexSet::default(),
                output_type_references: IndexSet::default(),
                ast: Ast::default(),
            })
        },
        |existing| match existing {
            SubgraphType::Enum(t) => Some(t),
            _ => None,
        },
        || format!("Expected {type_name} to be an enum type"),
    )
}

fn get_or_create_union_type<'s>(state: &'s mut SubgraphState, type_name: &str) -> JsResult<&'s mut UnionType> {
    get_or_create_type(
        state,
        type_name,
        || {
            SubgraphType::Union(UnionType {
                name: type_name.to_string(),
                members: IndexSet::default(),
                contexts: IndexSet::default(),
                inaccessible: false,
                tags: IndexSet::default(),
                is_definition: false,
                description: None,
                ast: Ast::default(),
            })
        },
        |existing| match existing {
            SubgraphType::Union(t) => Some(t),
            _ => None,
        },
        || format!("Expected {type_name} to be a union type"),
    )
}

fn get_or_create_object_field<'s>(
    state: &'s mut SubgraphState,
    rename_object: &RootTypeNames,
    type_name: &str,
    field_name: &str,
) -> JsResult<&'s mut Field> {
    let object_type = get_or_create_object_type(state, rename_object, type_name)?;

    Ok(get_or_insert_with(&mut object_type.fields, field_name, || new_field(field_name)))
}

fn get_or_create_interface_field<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    field_name: &str,
) -> JsResult<&'s mut Field> {
    let interface_type = get_or_create_interface_type(state, type_name)?;

    Ok(get_or_insert_with(&mut interface_type.fields, field_name, || new_field(field_name)))
}

fn get_or_create_input_object_field<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    field_name: &str,
) -> JsResult<&'s mut InputField> {
    let input_object_type = get_or_create_input_object_type(state, type_name)?;

    Ok(get_or_insert_with(&mut input_object_type.fields, field_name, || InputField {
            name: field_name.to_string(),
            r#type: MISSING.to_string(),
            kind: ArgumentKind::Scalar,
            inaccessible: false,
            cost: None,
            tags: IndexSet::default(),
            default_value: None,
            description: None,
            deprecated: None,
            ast: Ast::default(),
        }))
}

fn get_or_create_enum_value<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    enum_value_name: &str,
) -> JsResult<&'s mut EnumValue> {
    let enum_type = get_or_create_enum_type(state, type_name)?;

    Ok(get_or_insert_with(&mut enum_type.values, enum_value_name, || EnumValue {
            name: enum_value_name.to_string(),
            inaccessible: false,
            tags: IndexSet::default(),
            description: None,
            deprecated: None,
            ast: Ast::default(),
        }))
}

fn get_or_create_object_field_argument<'s>(
    state: &'s mut SubgraphState,
    rename_object: &RootTypeNames,
    type_name: &str,
    field_name: &str,
    arg_name: &str,
) -> JsResult<&'s mut Argument> {
    let field_state = get_or_create_object_field(state, rename_object, type_name, field_name)?;

    Ok(get_or_insert_with(&mut field_state.args, arg_name, || new_argument(arg_name)))
}

fn get_or_create_interface_field_argument<'s>(
    state: &'s mut SubgraphState,
    type_name: &str,
    field_name: &str,
    arg_name: &str,
) -> JsResult<&'s mut Argument> {
    let field_state = get_or_create_interface_field(state, type_name, field_name)?;

    Ok(get_or_insert_with(&mut field_state.args, arg_name, || new_argument(arg_name)))
}

fn is_schema_definition(node: &graphql_rs::Definition) -> Option<&SchemaDefinition> {
    match node {
        graphql_rs::Definition::SchemaDefinition(s) => Some(s),
        _ => None,
    }
}

/// `resolveTypeName(node)`: the named type inside list/non-null wrappers.
/// PERF: borrows the name instead of copying it.
fn resolve_type_name(node: &Type) -> &str {
    match node {
        Type::Named(n) => &n.name.value,
        Type::List(l) => resolve_type_name(&l.ty),
        Type::NonNull(n) => resolve_type_name(&n.ty),
    }
}

/// `isEnumType(node)`: an enum type definition or extension.
fn is_enum_type(node: &graphql_rs::Definition) -> bool {
    matches!(
        node,
        graphql_rs::Definition::EnumTypeDefinition(_) | graphql_rs::Definition::EnumTypeExtension(_)
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

#[cfg(test)]
mod tests {
    use super::*;
    use graphql_rs::parse;

    fn graph() -> GraphInput {
        GraphInput {
            id: "a".to_string(),
            name: "A".to_string(),
            url: None,
        }
    }

    fn builder(sdl: &str) -> SubgraphStateBuilder {
        builder_with_links(sdl, &[])
    }

    fn builder_with_links(sdl: &str, links: &[Link]) -> SubgraphStateBuilder {
        let doc = parse(sdl).expect("valid SDL");
        create_subgraph_state_builder(&graph(), &doc, FederationVersion::V2_3, links)
    }

    fn link(identity: &str, imports: Vec<LinkImport>) -> Link {
        Link {
            name: None,
            version: Some("v1.0".to_string()),
            identity: identity.to_string(),
            imports,
        }
    }

    fn import(kind: LinkImportKind, name: &str) -> LinkImport {
        LinkImport {
            kind,
            name: name.to_string(),
            alias: None,
        }
    }

    #[test]
    fn root_type_names_come_from_the_schema_definition() {
        let b = builder("schema { query: RootQuery mutation: M } type RootQuery { a: String }");
        assert_eq!(b.root_type_names.query, "RootQuery");
        assert_eq!(b.root_type_names.mutation, "M");
        assert_eq!(b.root_type_names.subscription, "Subscription");
        assert_eq!(b.root_type_names.rename_object_type("RootQuery"), "Query");
        assert_eq!(b.root_type_names.rename_object_type("M"), "Mutation");
        assert_eq!(b.root_type_names.rename_object_type("Subscription"), "Subscription");
        assert_eq!(b.root_type_names.rename_object_type("Query"), "Query");
        assert_eq!(b.root_type_names.rename_object_type("Other"), "Other");
    }

    #[test]
    fn link_spec_is_manually_provided_only_when_every_location_is_schema() {
        let b = builder("directive @link(url: String) repeatable on SCHEMA");
        assert!(b.state.specs.link);
        let b = builder("directive @link(url: String) on SCHEMA | OBJECT");
        assert!(!b.state.specs.link);
        let b = builder("directive @other on SCHEMA");
        assert!(!b.state.specs.link);
    }

    #[test]
    fn links_without_directives_and_federation_links_are_dropped() {
        let fed = link(
            "https://specs.apollo.dev/federation",
            vec![import(LinkImportKind::Directive, "@key")],
        );
        let types_only = link(
            "https://example.com/types",
            vec![import(LinkImportKind::Type, "Foo")],
        );
        let with_directive = link(
            "https://example.com/dirs",
            vec![
                import(LinkImportKind::Type, "Foo"),
                import(LinkImportKind::Directive, "@bar"),
            ],
        );
        let b = builder_with_links(
            "type Query { a: String }",
            &[fed.clone(), types_only, with_directive.clone()],
        );
        assert_eq!(b.state.links, vec![with_directive]);
        assert_eq!(b.state.federation.imports, fed.imports);
        assert_eq!(b.state.federation.version, FederationVersion::V2_3);

        let b = builder("type Query { a: String }");
        assert!(b.state.federation.imports.is_empty());
    }

    #[test]
    fn type_name_sets_are_collected_from_definitions_and_extensions() {
        let b = builder(
            "enum E { A } extend enum E2 { B } scalar S extend scalar S2 @d \
             input I { a: Int } extend input I2 { b: Int } type Query { e: E }",
        );
        assert!(b.leaf_type_names.contains("String"));
        assert!(b.leaf_type_names.contains("ID"));
        for name in ["E", "E2", "S", "S2"] {
            assert!(b.leaf_type_names.contains(name), "{name}");
        }
        assert_eq!(b.resolve_argument_kind("E"), ArgumentKind::Enum);
        assert_eq!(b.resolve_argument_kind("E2"), ArgumentKind::Enum);
        assert_eq!(b.resolve_argument_kind("I"), ArgumentKind::Object);
        assert_eq!(b.resolve_argument_kind("I2"), ArgumentKind::Object);
        assert_eq!(b.resolve_argument_kind("S"), ArgumentKind::Scalar);
        assert_eq!(
            b.enum_types_by_name.iter().cloned().collect::<Vec<_>>(),
            vec!["E".to_string(), "E2".to_string()]
        );
        assert_eq!(
            b.specified_scalars,
            vec!["String", "Int", "Float", "Boolean", "ID"]
        );
    }

    #[test]
    fn get_or_create_throws_on_a_kind_clash() {
        let mut b = builder("type Query { a: String }");
        b.scalar_type().set_definition("Foo").unwrap();
        let cases: Vec<(JsResult<()>, &str)> = vec![
            (b.enum_type().set_definition("Foo"), "Expected Foo to be an enum type"),
            (b.union_type().set_definition("Foo"), "Expected Foo to be a union type"),
            (
                b.interface_type().set_definition("Foo"),
                "Expected Foo to be an interface type",
            ),
            (
                b.input_object_type().set_definition("Foo"),
                "Expected Foo to be an input object type",
            ),
            (b.object_type().set_definition("Foo"), "Expected Foo to be an object type"),
            (b.directive().set_repeatable("Foo"), "Expected Foo to be a directive"),
        ];
        for (result, message) in cases {
            assert_eq!(result, Err(JsError::error(message)));
        }
        b.enum_type().set_definition("Bar").unwrap();
        assert_eq!(
            b.scalar_type().set_definition("Bar"),
            Err(JsError::error("Expected Bar to be a scalar type"))
        );
        // A failed lookup does not create or replace anything.
        assert_eq!(b.state.types.len(), 2);
    }

    #[test]
    fn object_types_are_stored_under_the_renamed_root_name() {
        let mut b = builder("schema { query: RootQuery } type RootQuery { a: String }");
        b.object_type().set_definition("RootQuery").unwrap();
        b.object_type().field().set_type("RootQuery", "a", "String").unwrap();
        let Some(SubgraphType::Object(query)) = b.state.types.get("Query") else {
            panic!("Query missing");
        };
        assert!(query.is_definition);
        assert_eq!(query.name, "Query");
        assert_eq!(query.fields["a"].r#type, "String");
        assert!(!b.state.types.contains_key("RootQuery"));

        // The error message names the renamed type.
        let mut b = builder("schema { mutation: M } type M { a: String }");
        b.scalar_type().set_definition("Mutation").unwrap();
        assert_eq!(
            b.object_type().set_definition("M"),
            Err(JsError::error("Expected Mutation to be an object type"))
        );
    }

    #[test]
    fn new_entries_keep_insertion_order_and_defaults() {
        let mut b = builder("type Query { a: String }");
        b.object_type().field().set_type("T", "z", "Int").unwrap();
        b.object_type().field().set_used("T", "a").unwrap();
        b.object_type().field().set_type("T", "z", "String").unwrap();
        b.object_type().field().arg().set_type("T", "a", "x", "Int").unwrap();
        let Some(SubgraphType::Object(t)) = b.state.types.get("T") else {
            panic!("T missing");
        };
        assert!(!t.is_definition);
        assert_eq!(t.fields.keys().collect::<Vec<_>>(), vec!["z", "a"]);
        assert_eq!(t.fields["z"].r#type, "String");
        let a = &t.fields["a"];
        assert_eq!(a.r#type, MISSING);
        assert!(a.used);
        assert_eq!(a.args["x"].r#type, "Int");
        assert_eq!(a.args["x"].kind, ArgumentKind::Scalar);
        assert_eq!(a.cost, None);
    }

    #[test]
    fn interface_object_calls_are_redirected_to_the_interface() {
        let mut b = builder("type Query { a: String }");
        b.interface_type().set_definition("Foo").unwrap();
        b.interface_type().set_interface_object("Foo").unwrap();
        assert!(b.is_interface_object("Foo"));
        assert!(!b.is_interface_object("Query"));
        assert!(!b.is_interface_object("Missing"));

        b.object_type().set_definition("Foo").unwrap();
        b.object_type().field().set_type("Foo", "id", "ID!").unwrap();
        b.object_type().field().set_shareable("Foo", "id").unwrap();
        b.object_type().field().set_provides("Foo", "id", "x").unwrap();
        b.object_type().field().arg().set_tag("Foo", "id", "arg", "t").unwrap();
        b.object_type().set_shareable("Foo").unwrap();
        b.object_type().set_cost("Foo", 5.0).unwrap();
        b.object_type()
            .set_key("Foo", "id", &IndexSet::from_iter(["id".to_string()]), true)
            .unwrap();

        let Some(SubgraphType::Interface(foo)) = b.state.types.get("Foo") else {
            panic!("Foo is not an interface");
        };
        assert!(foo.is_definition);
        let id = &foo.fields["id"];
        assert_eq!(id.r#type, "ID!");
        assert!(id.shareable);
        // setProvides is ignored for interface objects.
        assert_eq!(id.provides, None);
        assert!(id.args["arg"].tags.contains("t"));
        assert_eq!(
            foo.keys,
            vec![Key {
                fields: "id".to_string(),
                resolvable: true
            }]
        );
        assert!(foo.fields_used_as_keys.contains("id"));
    }

    #[test]
    fn object_set_interface_records_implemented_by() {
        let mut b = builder("type Query { a: String }");
        b.object_type().set_interface("Book", "Node").unwrap();
        let Some(SubgraphType::Interface(node)) = b.state.types.get("Node") else {
            panic!("Node missing");
        };
        assert!(node.implemented_by.contains("Book"));
        assert!(!node.is_definition);
        let Some(SubgraphType::Object(book)) = b.state.types.get("Book") else {
            panic!("Book missing");
        };
        assert!(book.interfaces.contains("Node"));
    }

    #[test]
    fn object_extension_type_keeps_extends() {
        let mut b = builder("type Query { a: String }");
        b.object_type().set_extension("A", ExtensionType::Extend).unwrap();
        b.object_type().set_extension("A", ExtensionType::Extends).unwrap();
        b.object_type().set_extension("A", ExtensionType::Extend).unwrap();
        b.object_type().set_extension("B", ExtensionType::Extends).unwrap();
        b.object_type().set_extension("B", ExtensionType::Extend).unwrap();
        let ext = |name: &str| match b.state.types.get(name) {
            Some(SubgraphType::Object(o)) => (o.extension, o.extension_type),
            _ => panic!("{name} missing"),
        };
        assert_eq!(ext("A"), (true, Some(ExtensionType::Extends)));
        assert_eq!(ext("B"), (true, Some(ExtensionType::Extends)));
    }

    #[test]
    fn object_set_external_marks_existing_fields_only() {
        let mut b = builder("type Query { a: String }");
        b.object_type().field().set_type("A", "x", "Int").unwrap();
        b.object_type().set_external("A").unwrap();
        b.object_type().field().set_type("A", "y", "Int").unwrap();
        let Some(SubgraphType::Object(a)) = b.state.types.get("A") else {
            panic!("A missing");
        };
        assert!(a.external);
        assert!(a.fields["x"].external);
        assert!(!a.fields["y"].external);
    }

    #[test]
    fn input_object_set_inaccessible_marks_existing_fields() {
        let mut b = builder("type Query { a: String }");
        b.input_object_type().field().set_type("I", "x", "Int").unwrap();
        b.input_object_type().set_inaccessible("I").unwrap();
        b.input_object_type().field().set_type("I", "y", "Int").unwrap();
        let Some(SubgraphType::InputObject(i)) = b.state.types.get("I") else {
            panic!("I missing");
        };
        assert!(i.inaccessible);
        assert!(i.fields["x"].inaccessible);
        assert!(!i.fields["y"].inaccessible);
    }

    fn one_of(b: &SubgraphStateBuilder) -> Option<&Directive> {
        match b.state.types.get("oneOf") {
            Some(SubgraphType::Directive(d)) => Some(d),
            _ => None,
        }
    }

    fn is_one_of(b: &SubgraphStateBuilder, name: &str) -> Option<Option<bool>> {
        match b.state.types.get(name) {
            Some(SubgraphType::InputObject(i)) => Some(i.is_one_of),
            _ => None,
        }
    }

    #[test]
    fn set_one_of_without_a_definition_creates_the_built_in() {
        let mut b = builder("type Query { a: String }");
        b.input_object_type().set_one_of("I").unwrap();
        let d = one_of(&b).expect("oneOf created");
        assert!(d.composed);
        assert_eq!(d.locations.iter().collect::<Vec<_>>(), vec!["INPUT_OBJECT"]);
        assert_eq!(is_one_of(&b, "I"), Some(Some(true)));
    }

    #[test]
    fn set_one_of_with_a_matching_definition_composes_it() {
        let mut b = builder("type Query { a: String }");
        b.directive().set_location("oneOf", "INPUT_OBJECT").unwrap();
        b.input_object_type().set_one_of("I").unwrap();
        assert!(one_of(&b).unwrap().composed);
        assert_eq!(is_one_of(&b, "I"), Some(Some(true)));
    }

    #[test]
    fn set_one_of_with_a_different_definition_does_nothing() {
        // Repeatable.
        let mut b = builder("type Query { a: String }");
        b.directive().set_location("oneOf", "INPUT_OBJECT").unwrap();
        b.directive().set_repeatable("oneOf").unwrap();
        b.input_object_type().set_one_of("I").unwrap();
        assert!(!one_of(&b).unwrap().composed);
        assert_eq!(is_one_of(&b, "I"), None);

        // Extra location.
        let mut b = builder("type Query { a: String }");
        b.directive().set_location("oneOf", "INPUT_OBJECT").unwrap();
        b.directive().set_location("oneOf", "OBJECT").unwrap();
        b.input_object_type().set_one_of("I").unwrap();
        assert!(!one_of(&b).unwrap().composed);

        // With an argument.
        let mut b = builder("type Query { a: String }");
        b.directive().set_location("oneOf", "INPUT_OBJECT").unwrap();
        b.directive().arg().set_type("oneOf", "x", "Int").unwrap();
        b.input_object_type().set_one_of("I").unwrap();
        assert!(!one_of(&b).unwrap().composed);

        // A type named oneOf.
        let mut b = builder("type Query { a: String }");
        b.scalar_type().set_definition("oneOf").unwrap();
        b.input_object_type().set_one_of("I").unwrap();
        assert_eq!(is_one_of(&b, "I"), None);
    }

    #[test]
    fn enum_references_are_recorded() {
        let mut b = builder("type Query { a: String }");
        b.enum_type()
            .set_referenced_by_output_type("E", "Query.a")
            .unwrap();
        b.enum_type()
            .set_referenced_by_input_type("E", "Query.a(x:)")
            .unwrap();
        let Some(SubgraphType::Enum(e)) = b.state.types.get("E") else {
            panic!("E missing");
        };
        assert!(e.referenced_by_output_type && e.referenced_by_input_type);
        assert!(!e.is_definition);
        assert!(e.output_type_references.contains("Query.a"));
        assert!(e.input_type_references.contains("Query.a(x:)"));
    }

    #[test]
    fn deprecated_and_override_values() {
        let mut b = builder("type Query { a: String }");
        b.object_type().field().set_deprecated("A", "x", None).unwrap();
        b.object_type().field().set_override("A", "x", "other", None).unwrap();
        b.interface_type().field().set_deprecated("I", "x", Some("why")).unwrap();
        b.interface_type()
            .field()
            .set_override("I", "x", "other", Some("percent(5)"))
            .unwrap();
        let Some(SubgraphType::Object(a)) = b.state.types.get("A") else {
            panic!("A missing");
        };
        assert_eq!(a.fields["x"].deprecated, Some(Deprecated { reason: None }));
        assert_eq!(a.fields["x"].r#override.as_deref(), Some("other"));
        assert_eq!(a.fields["x"].override_label, None);
        let Some(SubgraphType::Interface(i)) = b.state.types.get("I") else {
            panic!("I missing");
        };
        assert_eq!(
            i.fields["x"].deprecated,
            Some(Deprecated {
                reason: Some("why".to_string())
            })
        );
        assert_eq!(i.fields["x"].override_label.as_deref(), Some("percent(5)"));
    }

    #[test]
    fn directive_is_executable_by_location() {
        let mut b = builder("type Query { a: String }");
        b.directive().set_location("d", "OBJECT").unwrap();
        let exec = |b: &SubgraphStateBuilder| match b.state.types.get("d") {
            Some(SubgraphType::Directive(d)) => d.is_executable(),
            _ => panic!("d missing"),
        };
        assert!(!exec(&b));
        b.directive().set_location("d", "FIELD").unwrap();
        assert!(exec(&b));
        assert!(is_executable_directive_location("VARIABLE_DEFINITION"));
        assert!(is_executable_directive_location("QUERY"));
        assert!(!is_executable_directive_location("SCHEMA"));
        assert!(!is_executable_directive_location("field"));
    }

    #[test]
    fn clean_helpers_remove_spec_types_and_keep_order() {
        let mut b = builder("type Query { a: String }");
        for name in ["A", "_FieldSet", "link__Import", "B", "federation__Scope", "link", "C"] {
            b.scalar_type().set_definition(name).unwrap();
        }
        let state = clean_subgraph_state_from_link_spec(clean_subgraph_state_from_federation_spec(
            b.into_state(),
        ));
        assert_eq!(state.types.keys().collect::<Vec<_>>(), vec!["A", "B", "C"]);
    }

    #[test]
    fn small_helpers() {
        let doc = parse("type Q { a(x: [String!]! = \"d\"): Int }").unwrap();
        let Definition::ObjectTypeDefinition(q) = &doc.definitions[0] else {
            panic!("expected an object type");
        };
        assert_eq!(resolve_type_name(&q.fields[0].arguments[0].ty), "String");
        assert_eq!(print_output_type(&q.fields[0].arguments[0].ty), "[String!]!");

        let doc = parse("type T @d(reason: \"r\", url: 1) @interfaceObject { a: Int }").unwrap();
        let Definition::ObjectTypeDefinition(t) = &doc.definitions[0] else {
            panic!("expected an object type");
        };
        assert_eq!(find_string_argument(&t.directives[0], "reason"), Some("r"));
        // Not a string value.
        assert_eq!(find_string_argument(&t.directives[0], "url"), None);
        assert_eq!(find_string_argument(&t.directives[0], "missing"), None);
        assert!(has_interface_object_directive(&t.directives));
        assert!(!has_interface_object_directive(&[]));

        assert_eq!(
            asserted_field_name(None),
            Err(JsError::type_error(
                "Cannot read properties of undefined (reading 'name')"
            ))
        );
    }

    #[test]
    fn mark_spec_helpers() {
        let mut b = builder("type Query { a: String }");
        b.mark_cost_spec_as_used(CostSpecDirective::ListSize, "listSize");
        b.mark_spec_as_used(SpecName::RequiresScopes);
        assert!(b.state.specs.cost.used);
        assert_eq!(b.state.specs.cost.names.list_size.as_deref(), Some("listSize"));
        assert_eq!(b.state.specs.cost.names.cost, None);
        assert!(b.state.specs.requires_scopes);
        assert!(!b.state.specs.policy);
    }
}
