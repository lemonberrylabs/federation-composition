//! Port of graphql-js `PossibleTypeExtensionsRule`.

use crate::ast::{Definition, Kind, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::jsutils::did_you_mean::did_you_mean;
use crate::jsutils::suggestion_list::suggestion_list;
use crate::predicates::is_type_definition_node;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `PossibleTypeExtensionsRule`: a type extension extends a
/// defined type of the same kind.
pub struct PossibleTypeExtensionsRule<'a> {
    defined_types: IndexMap<&'a str, &'a Definition>,
}

impl<'a> PossibleTypeExtensionsRule<'a> {
    pub fn new(context: &mut SDLValidationContext<'a>) -> Self {
        let mut defined_types = IndexMap::default();
        for def in &context.get_document().definitions {
            if is_type_definition_node(def) {
                defined_types.insert(def.name().expect("type definitions have a name").value.as_str(), def);
            }
        }
        PossibleTypeExtensionsRule { defined_types }
    }

    fn check_extension(&mut self, context: &mut SDLValidationContext<'a>, node: NodeRef<'a>) {
        let type_name = node.name().expect("type extensions have a name").value.as_str();
        let def_node = self.defined_types.get(type_name);
        // NOTE(port): no schema, so no existing type.
        match def_node {
            Some(def_node) => {
                let expected_kind = def_kind_to_ext_kind(def_node.kind());
                if expected_kind != node.kind() {
                    let kind_str = extension_kind_to_type_name(node.kind());
                    context.report_error(GraphQLError::new(format!(
                        "Cannot extend non-{kind_str} type \"{type_name}\"."
                    )));
                }
            }
            None => {
                let all_type_names: Vec<&str> = self.defined_types.keys().copied().collect();
                context.report_error(GraphQLError::new(format!(
                    "Cannot extend type \"{type_name}\" because it is not defined.{}",
                    did_you_mean(None, &suggestion_list(type_name, &all_type_names))
                )));
            }
        }
    }
}

/// graphql-js `defKindToExtKind`.
fn def_kind_to_ext_kind(kind: Kind) -> Kind {
    match kind {
        Kind::ScalarTypeDefinition => Kind::ScalarTypeExtension,
        Kind::ObjectTypeDefinition => Kind::ObjectTypeExtension,
        Kind::InterfaceTypeDefinition => Kind::InterfaceTypeExtension,
        Kind::UnionTypeDefinition => Kind::UnionTypeExtension,
        Kind::EnumTypeDefinition => Kind::EnumTypeExtension,
        Kind::InputObjectTypeDefinition => Kind::InputObjectTypeExtension,
        _ => unreachable!("only type definitions are defined types"),
    }
}

/// graphql-js `extensionKindToTypeName`.
fn extension_kind_to_type_name(kind: Kind) -> &'static str {
    match kind {
        Kind::ScalarTypeExtension => "scalar",
        Kind::ObjectTypeExtension => "object",
        Kind::InterfaceTypeExtension => "interface",
        Kind::UnionTypeExtension => "union",
        Kind::EnumTypeExtension => "enum",
        Kind::InputObjectTypeExtension => "input object",
        // TS: invariant(false, 'Unexpected kind: ' + inspect(kind)).
        other => panic!("Unexpected kind: \"{other}\""),
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for PossibleTypeExtensionsRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[
            Kind::ScalarTypeExtension,
            Kind::ObjectTypeExtension,
            Kind::InterfaceTypeExtension,
            Kind::UnionTypeExtension,
            Kind::EnumTypeExtension,
            Kind::InputObjectTypeExtension,
        ])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        if matches!(
            node,
            NodeRef::ScalarTypeExtension(_)
                | NodeRef::ObjectTypeExtension(_)
                | NodeRef::InterfaceTypeExtension(_)
                | NodeRef::UnionTypeExtension(_)
                | NodeRef::EnumTypeExtension(_)
                | NodeRef::InputObjectTypeExtension(_)
        ) {
            self.check_extension(context, node);
        }
        VisitAction::Continue
    }
}
