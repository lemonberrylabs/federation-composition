//! Port of graphql-js `UniqueTypeNamesRule`.

use crate::ast::{Kind, Name, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `UniqueTypeNamesRule`: each type is defined once.
pub struct UniqueTypeNamesRule<'a> {
    known_type_names: IndexMap<&'a str, &'a Name>,
}

impl<'a> UniqueTypeNamesRule<'a> {
    pub fn new(_context: &mut SDLValidationContext<'a>) -> Self {
        // NOTE(port): no schema, so no "already exists in the schema" check.
        UniqueTypeNamesRule {
            known_type_names: IndexMap::default(),
        }
    }

    fn check_type_name(&mut self, context: &mut SDLValidationContext<'a>, name: &'a Name) -> VisitAction {
        let type_name = name.value.as_str();
        if self.known_type_names.contains_key(type_name) {
            context.report_error(GraphQLError::new(format!(
                "There can be only one type named \"{type_name}\"."
            )));
        } else {
            self.known_type_names.insert(type_name, name);
        }
        VisitAction::Skip
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueTypeNamesRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[
            Kind::ScalarTypeDefinition,
            Kind::ObjectTypeDefinition,
            Kind::InterfaceTypeDefinition,
            Kind::UnionTypeDefinition,
            Kind::EnumTypeDefinition,
            Kind::InputObjectTypeDefinition,
        ])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::ScalarTypeDefinition(node) => self.check_type_name(context, &node.name),
            NodeRef::ObjectTypeDefinition(node) => self.check_type_name(context, &node.name),
            NodeRef::InterfaceTypeDefinition(node) => self.check_type_name(context, &node.name),
            NodeRef::UnionTypeDefinition(node) => self.check_type_name(context, &node.name),
            NodeRef::EnumTypeDefinition(node) => self.check_type_name(context, &node.name),
            NodeRef::InputObjectTypeDefinition(node) => self.check_type_name(context, &node.name),
            _ => VisitAction::Continue,
        }
    }
}
