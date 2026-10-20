//! JSON forms of the crate's values, in the shapes of the TypeScript objects.
//!
//! Inputs: `serde` mirrors of the TS parameter objects, converted into the
//! crate's structs. Every field defaults, since the TS reads missing
//! properties as `undefined` (an absent `cost` is `null`-like, an absent flag
//! is falsy, an absent array is empty).
//! Outputs: `serde_json::Value`s built in the key order of the TS objects.

use federation_composition_rs::__internal::specifications::link::{Link, LinkImport, LinkUrl};
use federation_composition_rs::__internal::subgraph::context::ContextReference;
use federation_composition_rs::__internal::subgraph::state::{ArgumentKind, Deprecated, Description};
use federation_composition_rs::__internal::supergraph::composition::ast as core;
use federation_composition_rs::__internal::supergraph::state::SupergraphSpecs;
use federation_composition_rs::{FederatedLink, FederatedLinkImport, FederatedLinkUrl};
use graphql_rs::ConstDirective;
use serde::Deserialize;
use serde_json::{json, Map, Value};

// Inputs.

/// `ast?: { directives?: ConstDirectiveNode[] }`.
#[derive(Deserialize, Default)]
#[serde(default)]
struct AstIn {
    directives: Option<Vec<ConstDirective>>,
}

fn ast(a: Option<AstIn>) -> Vec<ConstDirective> {
    a.and_then(|a| a.directives).unwrap_or_default()
}

#[derive(Deserialize)]
pub(crate) struct DescriptionIn {
    value: String,
    #[serde(default)]
    block: bool,
}

impl From<DescriptionIn> for Description {
    fn from(d: DescriptionIn) -> Self {
        Description {
            value: d.value,
            block: d.block,
        }
    }
}

#[derive(Deserialize)]
struct DeprecatedIn {
    #[serde(default)]
    reason: Option<String>,
}

impl From<DeprecatedIn> for Deprecated {
    fn from(d: DeprecatedIn) -> Self {
        Deprecated { reason: d.reason }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CostIn {
    cost: f64,
    directive_name: String,
}

impl From<CostIn> for core::CostAst {
    fn from(c: CostIn) -> Self {
        core::CostAst {
            cost: c.cost,
            directive_name: c.directive_name,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListSizeIn {
    #[serde(default)]
    assumed_size: Option<f64>,
    #[serde(default)]
    slicing_arguments: Option<Vec<String>>,
    #[serde(default)]
    sized_fields: Option<Vec<String>>,
    #[serde(default)]
    require_one_slicing_argument: bool,
    #[serde(default)]
    print_require_one_slicing_argument: bool,
    directive_name: String,
}

impl From<ListSizeIn> for core::ListSizeAst {
    fn from(l: ListSizeIn) -> Self {
        core::ListSizeAst {
            assumed_size: l.assumed_size,
            slicing_arguments: l.slicing_arguments,
            sized_fields: l.sized_fields,
            require_one_slicing_argument: l.require_one_slicing_argument,
            print_require_one_slicing_argument: l.print_require_one_slicing_argument,
            directive_name: l.directive_name,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinTypeIn {
    graph: String,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    is_interface_object: Option<bool>,
    #[serde(default)]
    extension: Option<bool>,
    #[serde(default)]
    resolvable: Option<bool>,
}

#[derive(Deserialize)]
struct JoinImplementsIn {
    graph: String,
    interface: String,
}

#[derive(Deserialize)]
struct JoinFieldContextArgumentIn {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    context: String,
    selection: String,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct JoinFieldIn {
    graph: Option<String>,
    #[serde(rename = "type")]
    ty: Option<String>,
    r#override: Option<String>,
    override_label: Option<String>,
    used_overridden: Option<bool>,
    external: Option<bool>,
    provides: Option<String>,
    requires: Option<String>,
    context_arguments: Option<Vec<JoinFieldContextArgumentIn>>,
}

#[derive(Deserialize)]
struct JoinUnionMemberIn {
    graph: String,
    member: String,
}

#[derive(Deserialize)]
struct JoinEnumValueIn {
    graph: String,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct JoinIn {
    r#type: Option<Vec<JoinTypeIn>>,
    implements: Option<Vec<JoinImplementsIn>>,
    field: Option<Vec<JoinFieldIn>>,
    union_member: Option<Vec<JoinUnionMemberIn>>,
    enum_value: Option<Vec<JoinEnumValueIn>>,
}

fn join(j: Option<JoinIn>) -> core::JoinAst {
    let j = j.unwrap_or_default();
    core::JoinAst {
        r#type: j
            .r#type
            .unwrap_or_default()
            .into_iter()
            .map(|t| core::JoinTypeAst {
                graph: t.graph,
                key: t.key,
                is_interface_object: t.is_interface_object,
                extension: t.extension,
                resolvable: t.resolvable,
            })
            .collect(),
        implements: j
            .implements
            .unwrap_or_default()
            .into_iter()
            .map(|i| core::JoinImplementsAst {
                graph: i.graph,
                interface: i.interface,
            })
            .collect(),
        field: j
            .field
            .unwrap_or_default()
            .into_iter()
            .map(|f| core::JoinFieldAst {
                graph: f.graph,
                r#type: f.ty,
                r#override: f.r#override,
                override_label: f.override_label,
                used_overridden: f.used_overridden,
                external: f.external,
                provides: f.provides,
                requires: f.requires,
                context_arguments: f.context_arguments.map(|args| {
                    args.into_iter()
                        .map(|a| core::JoinFieldContextArgumentAst {
                            name: a.name,
                            r#type: a.ty,
                            context: a.context,
                            selection: a.selection,
                        })
                        .collect()
                }),
            })
            .collect(),
        union_member: j
            .union_member
            .unwrap_or_default()
            .into_iter()
            .map(|u| core::JoinUnionMemberAst {
                graph: u.graph,
                member: u.member,
            })
            .collect(),
        enum_value: j
            .enum_value
            .unwrap_or_default()
            .into_iter()
            .map(|e| core::JoinEnumValueAst { graph: e.graph })
            .collect(),
    }
}

/// The common optional properties of the `*AST` objects.
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Common {
    tags: Option<Vec<String>>,
    inaccessible: Option<bool>,
    authenticated: Option<bool>,
    policies: Option<Vec<Vec<String>>>,
    scopes: Option<Vec<Vec<String>>>,
    contexts: Option<Vec<String>>,
    cost: Option<CostIn>,
    list_size: Option<ListSizeIn>,
    description: Option<DescriptionIn>,
    deprecated: Option<DeprecatedIn>,
    ast: Option<AstIn>,
    join: Option<JoinIn>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FieldArgumentIn {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    kind: ArgumentKindIn,
    #[serde(default)]
    default_value: Option<String>,
    #[serde(flatten)]
    common: Common,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum ArgumentKindIn {
    Scalar,
    Object,
    Enum,
}

impl From<FieldArgumentIn> for core::FieldArgumentAst {
    fn from(a: FieldArgumentIn) -> Self {
        let c = a.common;
        core::FieldArgumentAst {
            name: a.name,
            r#type: a.ty,
            kind: match a.kind {
                ArgumentKindIn::Scalar => ArgumentKind::Scalar,
                ArgumentKindIn::Object => ArgumentKind::Object,
                ArgumentKindIn::Enum => ArgumentKind::Enum,
            },
            default_value: a.default_value,
            inaccessible: c.inaccessible.unwrap_or(false),
            cost: c.cost.map(Into::into),
            tags: c.tags.unwrap_or_default(),
            description: c.description.map(Into::into),
            deprecated: c.deprecated.map(Into::into),
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct FieldIn {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    arguments: Option<Vec<FieldArgumentIn>>,
    #[serde(flatten)]
    common: Common,
}

impl From<FieldIn> for core::FieldAst {
    fn from(f: FieldIn) -> Self {
        let c = f.common;
        core::FieldAst {
            name: f.name,
            r#type: f.ty,
            arguments: f.arguments.unwrap_or_default().into_iter().map(Into::into).collect(),
            join: join(c.join),
            inaccessible: c.inaccessible.unwrap_or(false),
            authenticated: c.authenticated.unwrap_or(false),
            policies: c.policies.unwrap_or_default(),
            scopes: c.scopes.unwrap_or_default(),
            cost: c.cost.map(Into::into),
            list_size: c.list_size.map(Into::into),
            tags: c.tags.unwrap_or_default(),
            description: c.description.map(Into::into),
            deprecated: c.deprecated.map(Into::into),
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InputFieldIn {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    default_value: Option<String>,
    #[serde(flatten)]
    common: Common,
}

impl From<InputFieldIn> for core::InputFieldAst {
    fn from(f: InputFieldIn) -> Self {
        let c = f.common;
        core::InputFieldAst {
            name: f.name,
            r#type: f.ty,
            default_value: f.default_value,
            tags: c.tags.unwrap_or_default(),
            inaccessible: c.inaccessible.unwrap_or(false),
            cost: c.cost.map(Into::into),
            description: c.description.map(Into::into),
            deprecated: c.deprecated.map(Into::into),
            ast: ast(c.ast),
            join: join(c.join),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct EnumValueIn {
    name: String,
    #[serde(flatten)]
    common: Common,
}

impl From<EnumValueIn> for core::EnumValueAst {
    fn from(v: EnumValueIn) -> Self {
        let c = v.common;
        core::EnumValueAst {
            name: v.name,
            join: join(c.join),
            tags: c.tags.unwrap_or_default(),
            inaccessible: c.inaccessible.unwrap_or(false),
            description: c.description.map(Into::into),
            deprecated: c.deprecated.map(Into::into),
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
struct LinkImportIn {
    name: String,
    #[serde(default)]
    alias: Option<String>,
}

#[derive(Deserialize)]
struct LinkIn {
    url: String,
    #[serde(default)]
    import: Option<Vec<LinkImportIn>>,
    #[serde(default, rename = "for")]
    for_: Option<String>,
}

/// `createSchemaNode`'s argument.
#[derive(Deserialize)]
pub(crate) struct SchemaIn {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    mutation: Option<String>,
    #[serde(default)]
    subscription: Option<String>,
    #[serde(default)]
    links: Option<Vec<LinkIn>>,
}

impl From<SchemaIn> for core::SchemaAst {
    fn from(s: SchemaIn) -> Self {
        core::SchemaAst {
            query: s.query,
            mutation: s.mutation,
            subscription: s.subscription,
            links: s
                .links
                .unwrap_or_default()
                .into_iter()
                .map(|l| core::LinkAst {
                    url: l.url,
                    import: l.import.map(|imports| {
                        imports
                            .into_iter()
                            .map(|i| core::LinkImportAst {
                                name: i.name,
                                alias: i.alias,
                            })
                            .collect()
                    }),
                    r#for: l.for_,
                })
                .collect(),
        }
    }
}

/// `createObjectTypeNode`'s and `createInterfaceTypeNode`'s argument.
#[derive(Deserialize)]
pub(crate) struct ObjectLikeIn {
    name: String,
    fields: Vec<FieldIn>,
    #[serde(default)]
    interfaces: Option<Vec<String>>,
    #[serde(flatten)]
    common: Common,
}

impl From<ObjectLikeIn> for core::ObjectTypeAst {
    fn from(o: ObjectLikeIn) -> Self {
        let c = o.common;
        core::ObjectTypeAst {
            name: o.name,
            fields: o.fields.into_iter().map(Into::into).collect(),
            interfaces: o.interfaces.unwrap_or_default(),
            join: join(c.join),
            tags: c.tags.unwrap_or_default(),
            inaccessible: c.inaccessible.unwrap_or(false),
            authenticated: c.authenticated.unwrap_or(false),
            policies: c.policies.unwrap_or_default(),
            scopes: c.scopes.unwrap_or_default(),
            contexts: c.contexts.unwrap_or_default(),
            cost: c.cost.map(Into::into),
            description: c.description.map(Into::into),
            ast: ast(c.ast),
        }
    }
}

impl From<ObjectLikeIn> for core::InterfaceTypeAst {
    fn from(o: ObjectLikeIn) -> Self {
        let c = o.common;
        core::InterfaceTypeAst {
            name: o.name,
            fields: o.fields.into_iter().map(Into::into).collect(),
            interfaces: o.interfaces.unwrap_or_default(),
            join: join(c.join),
            tags: c.tags.unwrap_or_default(),
            inaccessible: c.inaccessible.unwrap_or(false),
            authenticated: c.authenticated.unwrap_or(false),
            policies: c.policies.unwrap_or_default(),
            scopes: c.scopes.unwrap_or_default(),
            contexts: c.contexts.unwrap_or_default(),
            description: c.description.map(Into::into),
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InputObjectIn {
    name: String,
    fields: Vec<InputFieldIn>,
    #[serde(default)]
    is_one_of: Option<bool>,
    #[serde(flatten)]
    common: Common,
}

impl From<InputObjectIn> for core::InputObjectTypeAst {
    fn from(o: InputObjectIn) -> Self {
        let c = o.common;
        core::InputObjectTypeAst {
            name: o.name,
            fields: o.fields.into_iter().map(Into::into).collect(),
            join: join(c.join),
            tags: c.tags.unwrap_or_default(),
            inaccessible: c.inaccessible.unwrap_or(false),
            description: c.description.map(Into::into),
            is_one_of: o.is_one_of.unwrap_or(false),
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct UnionIn {
    name: String,
    members: Vec<String>,
    #[serde(flatten)]
    common: Common,
}

impl From<UnionIn> for core::UnionTypeAst {
    fn from(u: UnionIn) -> Self {
        let c = u.common;
        core::UnionTypeAst {
            name: u.name,
            join: join(c.join),
            members: u.members,
            inaccessible: c.inaccessible.unwrap_or(false),
            contexts: c.contexts.unwrap_or_default(),
            description: c.description.map(Into::into),
            tags: c.tags.unwrap_or_default(),
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct EnumIn {
    name: String,
    values: Vec<EnumValueIn>,
    #[serde(flatten)]
    common: Common,
}

impl From<EnumIn> for core::EnumTypeAst {
    fn from(e: EnumIn) -> Self {
        let c = e.common;
        core::EnumTypeAst {
            name: e.name,
            join: join(c.join),
            tags: c.tags.unwrap_or_default(),
            inaccessible: c.inaccessible.unwrap_or(false),
            authenticated: c.authenticated.unwrap_or(false),
            policies: c.policies.unwrap_or_default(),
            scopes: c.scopes.unwrap_or_default(),
            cost: c.cost.map(Into::into),
            description: c.description.map(Into::into),
            values: e.values.into_iter().map(Into::into).collect(),
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScalarIn {
    name: String,
    #[serde(default)]
    specified_by: Option<String>,
    #[serde(flatten)]
    common: Common,
}

impl From<ScalarIn> for core::ScalarTypeAst {
    fn from(s: ScalarIn) -> Self {
        let c = s.common;
        core::ScalarTypeAst {
            name: s.name,
            join: join(c.join),
            tags: c.tags.unwrap_or_default(),
            inaccessible: c.inaccessible.unwrap_or(false),
            authenticated: c.authenticated.unwrap_or(false),
            policies: c.policies.unwrap_or_default(),
            scopes: c.scopes.unwrap_or_default(),
            cost: c.cost.map(Into::into),
            description: c.description.map(Into::into),
            specified_by: s.specified_by,
            ast: ast(c.ast),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JoinGraphIn {
    name: String,
    enum_value: String,
    #[serde(default)]
    url: Option<String>,
}

impl From<JoinGraphIn> for core::JoinGraphAst {
    fn from(g: JoinGraphIn) -> Self {
        core::JoinGraphAst {
            name: g.name,
            enum_value: g.enum_value,
            url: g.url,
        }
    }
}

// Link classes (`utils/link`), as the shim's `toJSONValue()` writes them.

#[derive(Deserialize)]
pub(crate) struct LinkUrlIn {
    identity: String,
    name: Option<String>,
    version: Option<String>,
}

impl From<LinkUrlIn> for FederatedLinkUrl {
    fn from(u: LinkUrlIn) -> Self {
        FederatedLinkUrl::new(u.identity, u.name, u.version)
    }
}

#[derive(Deserialize)]
pub(crate) struct LinkImportClassIn {
    name: String,
    #[serde(rename = "as")]
    as_: Option<String>,
}

impl From<LinkImportClassIn> for FederatedLinkImport {
    fn from(i: LinkImportClassIn) -> Self {
        FederatedLinkImport::new(i.name, i.as_)
    }
}

#[derive(Deserialize)]
pub(crate) struct LinkClassIn {
    url: LinkUrlIn,
    #[serde(rename = "as")]
    as_: Option<String>,
    imports: Vec<LinkImportClassIn>,
}

impl From<LinkClassIn> for FederatedLink {
    fn from(l: LinkClassIn) -> Self {
        FederatedLink::new(l.url.into(), l.as_, l.imports.into_iter().map(Into::into).collect())
    }
}

// Outputs.

pub(crate) fn link_url_json(u: &FederatedLinkUrl) -> Value {
    json!({ "identity": u.identity(), "name": u.name(), "version": u.version() })
}

pub(crate) fn link_import_class_json(i: &FederatedLinkImport) -> Value {
    json!({ "name": i.name, "as": i.as_ })
}

pub(crate) fn link_class_json(l: &FederatedLink) -> Value {
    json!({
        "url": link_url_json(l.url()),
        "as": l.as_(),
        "imports": l.imports().iter().map(link_import_class_json).collect::<Vec<_>>(),
    })
}

/// `parseLinkUrl`'s `{ name, version, identity }`.
pub(crate) fn spec_link_url_json(u: &LinkUrl) -> Value {
    json!({ "name": u.name, "version": u.version, "identity": u.identity })
}

/// `{ kind, name, alias? }`.
pub(crate) fn spec_link_import_json(i: &LinkImport) -> Value {
    let mut m = Map::new();
    m.insert("kind".into(), i.kind.as_str().into());
    m.insert("name".into(), i.name.clone().into());
    if let Some(alias) = &i.alias {
        m.insert("alias".into(), alias.clone().into());
    }
    Value::Object(m)
}

/// A `mergeLinks` entry: `{ identity, version, imports, name }`.
pub(crate) fn spec_link_json(l: &Link) -> Value {
    json!({
        "identity": l.identity,
        "version": l.version,
        "imports": l.imports.iter().map(spec_link_import_json).collect::<Vec<_>>(),
        "name": l.name,
    })
}

pub(crate) fn specs_json(s: &SupergraphSpecs) -> Value {
    json!({
        "tag": s.tag,
        "context": s.context,
        "cost": {
            "used": s.cost.used,
            "names": { "cost": s.cost.names.cost, "listSize": s.cost.names.list_size },
        },
        "inaccessible": s.inaccessible,
        "policy": s.policy,
        "requiresScopes": s.requires_scopes,
        "authenticated": s.authenticated,
    })
}

/// `{ context, selection }`; `notAReference` has both undefined.
pub(crate) fn context_reference_json(r: &ContextReference) -> Value {
    let mut m = Map::new();
    if let Some(context) = &r.context {
        m.insert("context".into(), context.clone().into());
    }
    if let Some(selection) = &r.selection {
        m.insert("selection".into(), selection.clone().into());
    }
    Value::Object(m)
}
