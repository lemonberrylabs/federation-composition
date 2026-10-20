//! Port of graphql-js `KnownTypeNamesRule`.

use crate::ast::{Kind, NodeRef};
use crate::collections::IndexSet;
use crate::error::GraphQLError;
use crate::jsutils::did_you_mean::did_you_mean;
use crate::jsutils::suggestion_list::suggestion_list;
use crate::predicates::{is_type_definition_node, is_type_system_definition_node, is_type_system_extension_node};
use crate::r#type::scalars::SPECIFIED_SCALAR_TYPE_NAMES;
use crate::specified::INTROSPECTION_TYPE_NAMES;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{Ancestor, VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `KnownTypeNamesRule`: every referenced type is defined (in SDL,
/// the specified scalars and introspection types count as defined).
pub struct KnownTypeNamesRule<'a> {
    type_names: IndexSet<&'a str>,
}

impl<'a> KnownTypeNamesRule<'a> {
    pub fn new(context: &mut SDLValidationContext<'a>) -> Self {
        let definitions = &context.get_document().definitions;
        // NOTE(port): no schema, so no existing type names.
        let type_names = definitions
            .iter()
            .filter(|def| is_type_definition_node(*def))
            .map(|def| def.name().expect("type definitions have a name").value.as_str())
            .collect();
        KnownTypeNamesRule { type_names }
    }
}

/// graphql-js `standardTypeNames`, in order.
fn standard_type_names() -> impl Iterator<Item = &'static str> {
    SPECIFIED_SCALAR_TYPE_NAMES
        .iter()
        .chain(INTROSPECTION_TYPE_NAMES.iter())
        .copied()
}

/// graphql-js `isSDLNode`: a node (not a list) of a type system kind.
fn is_sdl_node(value: Ancestor<'_>) -> bool {
    match value {
        Ancestor::Node(node) => is_type_system_definition_node(node) || is_type_system_extension_node(node),
        Ancestor::List(_) => false,
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for KnownTypeNamesRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::NamedType])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let NodeRef::NamedType(node) = node else {
            return VisitAction::Continue;
        };
        let type_name = node.name.value.as_str();
        if !self.type_names.contains(type_name) {
            let definition_node = info.ancestors.get(2).copied().or(info.parent);
            let is_sdl = definition_node.is_some_and(is_sdl_node);
            if is_sdl && standard_type_names().any(|name| name == type_name) {
                return VisitAction::Continue;
            }
            let mut options: Vec<&str> = Vec::new();
            if is_sdl {
                for name in standard_type_names() {
                    options.push(name);
                }
            }
            options.extend(self.type_names.iter().copied());
            let suggested_types = suggestion_list(type_name, &options);
            context.report_error(GraphQLError::new(format!(
                "Unknown type \"{type_name}\".{}",
                did_you_mean(None, &suggested_types)
            )));
        }
        VisitAction::Continue
    }
}
