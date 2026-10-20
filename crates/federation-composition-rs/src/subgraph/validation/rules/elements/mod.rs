//! Port of `src/subgraph/validation/rules/elements/`.

pub mod authenticated;
pub mod compose_directive;
pub mod context;
pub mod cost;
pub mod extends;
pub mod external;
pub mod field_set;
pub mod from_context;
pub mod inaccessible;
pub mod interface_object;
pub mod key;
pub mod list_size;
pub mod r#override;
pub mod policy;
pub mod provides;
pub mod requires;
pub mod requires_scopes;
pub mod shareable;
pub mod tag;

// Helpers shared by the rules of this directory. They are not TS functions: each one is a
// fragment the TS repeats inline in several rules.

use graphql_rs::{Ancestor, Kind, NodeRef, PathKey, Type, VisitAction, VisitInfo};

use crate::subgraph::validation::validation_context::SubgraphValidationContext;
use crate::utils::js::{JsError, JsResult};

/// Ends a visitor callback: `Ok` is the TS `return` (`undefined`), `Err` is a TS `throw`
/// inside the callback, recorded with `cx.throw` (which stops the whole traversal).
pub(crate) fn finish(cx: &mut SubgraphValidationContext<'_>, result: JsResult<()>) -> VisitAction {
    match result {
        Ok(()) => VisitAction::Continue,
        Err(error) => cx.throw(error),
    }
}

/// The node annotated by the visited directive, found the way the TS does it:
///
/// ```ts
/// const directivesKeyAt = paths.findIndex((path) => path === "directives");
/// if (directivesKeyAt === -1) throw new Error('Could not find "directives" key in ancestors');
/// const parent = ancestors[directivesKeyAt];
/// if (!parent) throw new Error(notFoundMessage);
/// if (Array.isArray(parent)) throw new Error("Expected parent to be a single node");
/// if (!("kind" in parent)) throw new Error("Expected parent to be a node");
/// ```
///
/// The last check cannot fail for a node.
pub(crate) fn find_annotated_node<'a>(info: &VisitInfo<'a, '_>, not_found_message: &str) -> JsResult<NodeRef<'a>> {
    let Some(directives_key_at) = info.path.iter().position(|path| *path == PathKey::Key("directives")) else {
        return Err(JsError::error("Could not find \"directives\" key in ancestors"));
    };

    match info.ancestors.get(directives_key_at) {
        None => Err(JsError::error(not_found_message)),
        Some(Ancestor::List(_)) => Err(JsError::error("Expected parent to be a single node")),
        Some(Ancestor::Node(node)) => Ok(*node),
    }
}

/// `node.name.value` for a node that has a name (type definitions and extensions, fields,
/// input values, enum values, directive definitions). The callers only pass named kinds; a
/// nameless node gives `""`.
pub(crate) fn node_name(node: NodeRef<'_>) -> &str {
    node.name().map_or("", |name| name.value.as_str())
}

/// `node.type` of a `FieldDefinition` or `InputValueDefinition` (what
/// `TypeNodeInfo.getFieldDef()` returns).
pub(crate) fn field_type(node: NodeRef<'_>) -> Option<&Type> {
    match node {
        NodeRef::FieldDefinition(field) => Some(&field.ty),
        NodeRef::InputValueDefinition(input) => Some(&input.ty),
        _ => None,
    }
}

/// `kind === Kind.OBJECT_TYPE_DEFINITION || kind === Kind.OBJECT_TYPE_EXTENSION`.
pub(crate) fn is_object_kind(kind: Kind) -> bool {
    matches!(kind, Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension)
}

/// `kind === Kind.INTERFACE_TYPE_DEFINITION || kind === Kind.INTERFACE_TYPE_EXTENSION`.
pub(crate) fn is_interface_kind(kind: Kind) -> bool {
    matches!(kind, Kind::InterfaceTypeDefinition | Kind::InterfaceTypeExtension)
}
