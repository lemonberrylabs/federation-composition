//! Port of graphql-js `UniqueEnumValueNamesRule`.

use crate::ast::{EnumValueDefinition, Kind, Name, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `UniqueEnumValueNamesRule`: each enum value is defined once
/// across an enum and its extensions.
pub struct UniqueEnumValueNamesRule<'a> {
    known_value_names: IndexMap<&'a str, IndexMap<&'a str, &'a Name>>,
}

impl<'a> UniqueEnumValueNamesRule<'a> {
    pub fn new(_context: &mut SDLValidationContext<'a>) -> Self {
        // NOTE(port): no schema, so `existingTypeMap` is empty.
        UniqueEnumValueNamesRule {
            known_value_names: IndexMap::default(),
        }
    }

    fn check_value_uniqueness(
        &mut self,
        context: &mut SDLValidationContext<'a>,
        name: &'a Name,
        value_nodes: &'a [EnumValueDefinition],
    ) -> VisitAction {
        let type_name = name.value.as_str();
        let value_names = self.known_value_names.entry(type_name).or_default();
        for value_def in value_nodes {
            let value_name = value_def.name.value.as_str();
            if value_names.contains_key(value_name) {
                context.report_error(GraphQLError::new(format!(
                    "Enum value \"{type_name}.{value_name}\" can only be defined once."
                )));
            } else {
                value_names.insert(value_name, &value_def.name);
            }
        }
        VisitAction::Skip
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueEnumValueNamesRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::EnumTypeDefinition, Kind::EnumTypeExtension])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::EnumTypeDefinition(node) => self.check_value_uniqueness(context, &node.name, &node.values),
            NodeRef::EnumTypeExtension(node) => self.check_value_uniqueness(context, &node.name, &node.values),
            _ => VisitAction::Continue,
        }
    }
}
