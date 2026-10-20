//! Port of graphql-js `UniqueFieldDefinitionNamesRule`.

use crate::ast::{Kind, Name, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `UniqueFieldDefinitionNamesRule`: each field is defined once
/// across a type and its extensions.
pub struct UniqueFieldDefinitionNamesRule<'a> {
    known_field_names: IndexMap<&'a str, IndexMap<&'a str, &'a Name>>,
}

impl<'a> UniqueFieldDefinitionNamesRule<'a> {
    pub fn new(_context: &mut SDLValidationContext<'a>) -> Self {
        // NOTE(port): no schema, so `existingTypeMap` is empty.
        UniqueFieldDefinitionNamesRule {
            known_field_names: IndexMap::default(),
        }
    }

    fn check_field_uniqueness(
        &mut self,
        context: &mut SDLValidationContext<'a>,
        name: &'a Name,
        field_names_nodes: impl Iterator<Item = &'a Name>,
    ) -> VisitAction {
        let type_name = name.value.as_str();
        let field_names = self.known_field_names.entry(type_name).or_default();
        for field_name_node in field_names_nodes {
            let field_name = field_name_node.value.as_str();
            if field_names.contains_key(field_name) {
                context.report_error(GraphQLError::new(format!(
                    "Field \"{type_name}.{field_name}\" can only be defined once."
                )));
            } else {
                field_names.insert(field_name, field_name_node);
            }
        }
        VisitAction::Skip
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueFieldDefinitionNamesRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[
            Kind::InputObjectTypeDefinition,
            Kind::InputObjectTypeExtension,
            Kind::InterfaceTypeDefinition,
            Kind::InterfaceTypeExtension,
            Kind::ObjectTypeDefinition,
            Kind::ObjectTypeExtension,
        ])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::InputObjectTypeDefinition(node) => {
                self.check_field_uniqueness(context, &node.name, node.fields.iter().map(|f| &f.name))
            }
            NodeRef::InputObjectTypeExtension(node) => {
                self.check_field_uniqueness(context, &node.name, node.fields.iter().map(|f| &f.name))
            }
            NodeRef::InterfaceTypeDefinition(node) => {
                self.check_field_uniqueness(context, &node.name, node.fields.iter().map(|f| &f.name))
            }
            NodeRef::InterfaceTypeExtension(node) => {
                self.check_field_uniqueness(context, &node.name, node.fields.iter().map(|f| &f.name))
            }
            NodeRef::ObjectTypeDefinition(node) => {
                self.check_field_uniqueness(context, &node.name, node.fields.iter().map(|f| &f.name))
            }
            NodeRef::ObjectTypeExtension(node) => {
                self.check_field_uniqueness(context, &node.name, node.fields.iter().map(|f| &f.name))
            }
            _ => VisitAction::Continue,
        }
    }
}
