//! Port of `src/graphql/transform-supergraph-to-public-schema.ts`.

use std::sync::LazyLock;

use crate::utils::collections::IndexSet;
use graphql_rs::{
    visit_edit, Definition, Directive, Document, EditAction, EditVisitor, Kind, NamedType, Node, NodeRef, Value,
    VisitInfo,
};

use super::supergraph_spec::{
    get_supergraph_spec_nodes, EXTRA_FEDERATION_DIRECTIVE_NAMES, EXTRA_FEDERATION_TYPE_NAMES,
};
use crate::utils::js::JsResult;
use crate::utils::link::extract_link_implementations;

/// `specifiedDirectives` names without `oneOf` (older graphql versions do not support it).
static SPECIFIED_DIRECTIVES: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_rs::specified_directives()
        .iter()
        .map(|d| d.name.value.clone())
        .filter(|d| d != "oneOf")
        .collect()
});

/// The first schema definition's directives (`documentNode.definitions.find(...)?.directives`).
fn schema_definition_directives(document_node: &Document) -> Option<&[Directive]> {
    document_node.definitions.iter().find_map(|node| match node {
        Definition::SchemaDefinition(schema) => Some(schema.directives.as_slice()),
        _ => None,
    })
}

/// `getAdditionalDirectivesToStrip(documentNode)`: the `as:` names of `@link`s on the schema
/// definition; `None` when the schema definition has no directives.
fn get_additional_directives_to_strip(document_node: &Document) -> Option<IndexSet<String>> {
    let directives = schema_definition_directives(document_node).filter(|d| !d.is_empty())?;

    let mut additional_directives_to_strip = IndexSet::default();
    for directive in directives {
        if directive.name.value != "link" {
            continue;
        }
        let as_arg = directive.arguments.iter().find(|arg| arg.name.value == "as");

        if let Some(Value::String(value)) = as_arg.map(|a| &a.value) {
            additional_directives_to_strip.insert(value.value.clone());
        }
    }

    Some(additional_directives_to_strip)
}

const FEDERATION_INACCESSIBLE_DIRECTIVE_URL_PREFIX: &str = "https://specs.apollo.dev/inaccessible";

/// `getInaccessibleDirectiveName(documentNode)`.
fn get_inaccessible_directive_name(document_node: &Document) -> String {
    if let Some(directives) = schema_definition_directives(document_node) {
        for directive in directives {
            if directive.name.value != "link" {
                continue;
            }
            let url_arg = directive.arguments.iter().find(|arg| arg.name.value == "url");
            let as_arg = directive.arguments.iter().find(|arg| arg.name.value == "as");

            if let Some(Value::String(url)) = url_arg.map(|a| &a.value) {
                if url.value.starts_with(FEDERATION_INACCESSIBLE_DIRECTIVE_URL_PREFIX) {
                    if let Some(Value::String(as_)) = as_arg.map(|a| &a.value) {
                        return as_.value.clone();
                    }
                    break;
                }
            }
        }
    }

    "inaccessible".to_string()
}

/// The visitor of the first `visit`: the TS closures and the sets they share.
struct PublicSchemaVisitor {
    additional_federation_directives: Option<IndexSet<String>>,
    inaccessible_directive_name: String,
    spec_prefixes: Vec<String>,
    supergraph_directives: IndexSet<String>,
    supergraph_enums: IndexSet<String>,
    supergraph_input_objects: IndexSet<String>,
    supergraph_scalars: IndexSet<String>,
    definitions_to_remove: IndexSet<String>,
    directives_to_remove: IndexSet<String>,
    inaccessible_interface_types: IndexSet<String>,
}

impl PublicSchemaVisitor {
    /// `removeFederationOrSpecifiedDirectives(node)`: `true` for `return null`.
    fn remove_federation_or_specified_directives(&self, kind: Kind, name: &str) -> bool {
        self.belongs_to_linked_spec(name)
            || self.supergraph_directives.contains(name)
            || EXTRA_FEDERATION_DIRECTIVE_NAMES.contains(name)
            || self
                .additional_federation_directives
                .as_ref()
                .is_some_and(|d| d.contains(name))
            || self.directives_to_remove.contains(name)
            || (kind == Kind::DirectiveDefinition && SPECIFIED_DIRECTIVES.contains(name))
    }

    /// `belongsToLinkedSpec(node)`.
    fn belongs_to_linked_spec(&self, name: &str) -> bool {
        self.spec_prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix.as_str()))
    }

    /// `hasInaccessibleDirective(node)`.
    fn has_inaccessible_directive(&self, directives: &[Directive]) -> bool {
        directives
            .iter()
            .any(|d| d.name.value == self.inaccessible_directive_name)
    }

    /// `removeInaccessibleNode(node)`: `true` for `return null`.
    fn remove_inaccessible_node(&mut self, kind: Kind, name: &str, directives: &[Directive]) -> bool {
        if self.has_inaccessible_directive(directives) || self.belongs_to_linked_spec(name) {
            if kind == Kind::InterfaceTypeDefinition {
                self.inaccessible_interface_types.insert(name.to_string());
            }
            return true;
        }
        false
    }
}

fn delete_if(remove: bool) -> EditAction {
    if remove {
        EditAction::Delete
    } else {
        EditAction::Continue
    }
}

impl EditVisitor for PublicSchemaVisitor {
    fn enter(&mut self, node: NodeRef<'_>, _info: &VisitInfo<'_, '_>) -> EditAction {
        let kind = node.kind();
        match node {
            NodeRef::DirectiveDefinition(n) => {
                delete_if(self.remove_federation_or_specified_directives(kind, &n.name.value))
            }
            NodeRef::Directive(n) => delete_if(self.remove_federation_or_specified_directives(kind, &n.name.value)),
            NodeRef::SchemaExtension(_) | NodeRef::SchemaDefinition(_) => EditAction::Delete,
            NodeRef::ScalarTypeDefinition(n) => {
                let name = n.name.value.as_str();
                delete_if(
                    self.belongs_to_linked_spec(name)
                        || self.supergraph_scalars.contains(name)
                        || EXTRA_FEDERATION_TYPE_NAMES.contains(name)
                        || self.definitions_to_remove.contains(name)
                        || self.has_inaccessible_directive(&n.directives),
                )
            }
            NodeRef::EnumTypeDefinition(n) => {
                let name = n.name.value.as_str();
                delete_if(
                    self.belongs_to_linked_spec(name)
                        || self.supergraph_enums.contains(name)
                        || EXTRA_FEDERATION_TYPE_NAMES.contains(name)
                        || self.definitions_to_remove.contains(name)
                        || self.has_inaccessible_directive(&n.directives),
                )
            }
            NodeRef::EnumValueDefinition(n) => {
                delete_if(self.remove_inaccessible_node(kind, &n.name.value, &n.directives))
            }
            NodeRef::ObjectTypeDefinition(n) => {
                delete_if(self.remove_inaccessible_node(kind, &n.name.value, &n.directives))
            }
            NodeRef::FieldDefinition(n) => delete_if(self.remove_inaccessible_node(kind, &n.name.value, &n.directives)),
            NodeRef::InterfaceTypeDefinition(n) => {
                delete_if(self.remove_inaccessible_node(kind, &n.name.value, &n.directives))
            }
            NodeRef::UnionTypeDefinition(n) => {
                delete_if(self.remove_inaccessible_node(kind, &n.name.value, &n.directives))
            }
            NodeRef::InputObjectTypeDefinition(n) => {
                let name = n.name.value.as_str();
                delete_if(
                    self.belongs_to_linked_spec(name)
                        || self.supergraph_input_objects.contains(name)
                        || self.definitions_to_remove.contains(name)
                        || self.has_inaccessible_directive(&n.directives),
                )
            }
            NodeRef::InputValueDefinition(n) => {
                delete_if(self.remove_inaccessible_node(kind, &n.name.value, &n.directives))
            }
            _ => EditAction::Continue,
        }
    }
}

/// The visitor of the second `visit` (`removeInaccessibleInterfaces`).
struct InaccessibleInterfacesVisitor<'s> {
    inaccessible_interface_types: &'s IndexSet<String>,
}

impl InaccessibleInterfacesVisitor<'_> {
    /// `node.interfaces.filter(...)`, or `None` when nothing is removed.
    fn new_interfaces(&self, interfaces: &[NamedType]) -> Option<Vec<NamedType>> {
        let new_interfaces: Vec<NamedType> = interfaces
            .iter()
            .filter(|value| !self.inaccessible_interface_types.contains(&value.name.value))
            .cloned()
            .collect();

        if new_interfaces.len() == interfaces.len() {
            return None;
        }

        Some(new_interfaces)
    }
}

impl EditVisitor for InaccessibleInterfacesVisitor<'_> {
    fn enter(&mut self, node: NodeRef<'_>, _info: &VisitInfo<'_, '_>) -> EditAction {
        match node {
            NodeRef::ObjectTypeDefinition(n) => match self.new_interfaces(&n.interfaces) {
                Some(interfaces) => EditAction::Replace(Node::ObjectTypeDefinition(graphql_rs::ObjectTypeDefinition {
                    interfaces,
                    ..n.clone()
                })),
                None => EditAction::Continue,
            },
            NodeRef::InterfaceTypeDefinition(n) => match self.new_interfaces(&n.interfaces) {
                Some(interfaces) => {
                    EditAction::Replace(Node::InterfaceTypeDefinition(graphql_rs::InterfaceTypeDefinition {
                        interfaces,
                        ..n.clone()
                    }))
                }
                None => EditAction::Continue,
            },
            _ => EditAction::Continue,
        }
    }
}

/// `transformSupergraphToPublicSchema(documentNode)`: the public API schema, as served by a
/// gateway. The TS visitor callbacks (`removeFederationOrSpecifiedDirectives`,
/// `removeInaccessibleNode`, ...) are closures over local sets; port them as a
/// `graphql_rs::EditVisitor` struct holding those sets.
/// Fallible because `extractLinkImplementations` can throw.
pub fn transform_supergraph_to_public_schema(document_node: &Document) -> JsResult<Document> {
    let additional_federation_directives = get_additional_directives_to_strip(document_node);
    let inaccessible_directive_name = get_inaccessible_directive_name(document_node);

    let spec_links: Vec<_> = extract_link_implementations(document_node)?
        .into_links()
        .into_iter()
        .filter(|link| link.identity().contains("//specs.apollo.dev/"))
        .collect();

    let spec_prefixes: Vec<String> = spec_links
        .iter()
        .map(|l| {
            let identity = l.identity();
            let start = identity.rfind('/').map_or(0, |i| i + 1);
            format!("{}__", &identity[start..])
        })
        .collect();

    let mut supergraph_directives = IndexSet::default();
    let mut supergraph_enums = IndexSet::default();
    let mut supergraph_input_objects = IndexSet::default();
    let mut supergraph_scalars = IndexSet::default();

    for node in get_supergraph_spec_nodes() {
        match node.kind {
            Kind::EnumTypeDefinition => {
                supergraph_enums.insert(node.name.clone());
            }
            Kind::InputObjectTypeDefinition => {
                supergraph_input_objects.insert(node.name.clone());
            }
            Kind::ScalarTypeDefinition => {
                supergraph_scalars.insert(node.name.clone());
            }
            Kind::DirectiveDefinition => {
                supergraph_directives.insert(node.name.clone());
            }
            _ => {}
        }
    }

    let imported_pieces: Vec<String> = spec_links
        .iter()
        .flat_map(|l| {
            l.imports()
                .iter()
                .map(|i| i.as_.clone().unwrap_or_else(|| i.name.clone()))
        })
        .collect();
    // `name.replace("@", "")`: the first `@` anywhere.
    let definitions_to_remove: IndexSet<String> =
        imported_pieces.iter().map(|name| name.replacen('@', "", 1)).collect();
    let directives_to_remove: IndexSet<String> = imported_pieces
        .iter()
        .filter_map(|name| name.strip_prefix('@').map(str::to_string))
        .collect();

    let mut visitor = PublicSchemaVisitor {
        additional_federation_directives,
        inaccessible_directive_name,
        spec_prefixes,
        supergraph_directives,
        supergraph_enums,
        supergraph_input_objects,
        supergraph_scalars,
        definitions_to_remove,
        directives_to_remove,
        inaccessible_interface_types: IndexSet::default(),
    };

    let new_document_node = visit_edit(document_node, &mut visitor).into_document();

    if visitor.inaccessible_interface_types.is_empty() {
        return Ok(new_document_node);
    }

    // If there are inaccessible interface types we need to visit the document a second time and filter these out.
    let mut second = InaccessibleInterfacesVisitor {
        inaccessible_interface_types: &visitor.inaccessible_interface_types,
    };
    Ok(visit_edit(&new_document_node, &mut second).into_document())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected output generated with the TS function and graphql-js `print`.
    #[test]
    fn public_schema() {
        let sdl = r#"
schema
  @link(url: "https://specs.apollo.dev/link/v1.0")
  @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION)
  @link(url: "https://specs.apollo.dev/inaccessible/v0.2", as: "hidden", for: SECURITY)
{
  query: Query
}

directive @hidden on FIELD_DEFINITION | OBJECT | INTERFACE
directive @join__field(graph: join__Graph) repeatable on FIELD_DEFINITION
directive @custom on FIELD_DEFINITION
directive @skip(if: Boolean!) on FIELD

scalar join__FieldSet
scalar link__Import
enum join__Graph { A @join__graph(name: "a", url: "") }
enum link__Purpose { SECURITY EXECUTION }

interface Node @hidden { id: ID! }
interface Named { name: String }
type Query @join__type(graph: A) {
  a: Int @custom @join__field(graph: A)
  b: Int @hidden
  c(x: Int @hidden, y: Int): Int
}
type User implements Node & Named { id: ID! name: String }
enum E { A B @hidden }
"#;
        let doc = graphql_rs::parse(sdl).unwrap();
        let out = graphql_rs::print(&transform_supergraph_to_public_schema(&doc).unwrap());
        assert_eq!(
            out,
            "directive @custom on FIELD_DEFINITION\n\ninterface Named {\n  name: String\n}\n\ntype Query {\n  a: Int @custom\n  c(y: Int): Int\n}\n\ntype User implements Named {\n  id: ID!\n  name: String\n}\n\nenum E {\n  A\n}"
        );
    }
}
