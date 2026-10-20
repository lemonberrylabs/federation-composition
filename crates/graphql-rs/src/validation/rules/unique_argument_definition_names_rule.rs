//! Port of graphql-js `UniqueArgumentDefinitionNamesRule`.

use crate::ast::{FieldDefinition, InputValueDefinition, Kind, Name, NodeRef};
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

use super::group_by;

/// graphql-js `UniqueArgumentDefinitionNamesRule`: each argument of a field
/// or directive definition is defined once.
pub struct UniqueArgumentDefinitionNamesRule;

impl UniqueArgumentDefinitionNamesRule {
    pub fn new(_context: &mut SDLValidationContext<'_>) -> Self {
        UniqueArgumentDefinitionNamesRule
    }
}

fn check_arg_uniqueness_per_field(
    context: &mut SDLValidationContext<'_>,
    name: &Name,
    field_nodes: &[FieldDefinition],
) -> VisitAction {
    let type_name = &name.value;
    for field_def in field_nodes {
        let field_name = &field_def.name.value;
        check_arg_uniqueness(
            context,
            &format!("{type_name}.{field_name}"),
            &field_def.arguments,
        );
    }
    VisitAction::Skip
}

fn check_arg_uniqueness(
    context: &mut SDLValidationContext<'_>,
    parent_name: &str,
    argument_nodes: &[InputValueDefinition],
) -> VisitAction {
    let seen_args = group_by(argument_nodes, |arg| arg.name.value.as_str());
    for (arg_name, arg_nodes) in seen_args {
        if arg_nodes.len() > 1 {
            context.report_error(GraphQLError::new(format!(
                "Argument \"{parent_name}({arg_name}:)\" can only be defined once."
            )));
        }
    }
    VisitAction::Skip
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueArgumentDefinitionNamesRule {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[
            Kind::DirectiveDefinition,
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
            NodeRef::DirectiveDefinition(directive_node) => check_arg_uniqueness(
                context,
                &format!("@{}", directive_node.name.value),
                &directive_node.arguments,
            ),
            NodeRef::InterfaceTypeDefinition(node) => {
                check_arg_uniqueness_per_field(context, &node.name, &node.fields)
            }
            NodeRef::InterfaceTypeExtension(node) => {
                check_arg_uniqueness_per_field(context, &node.name, &node.fields)
            }
            NodeRef::ObjectTypeDefinition(node) => {
                check_arg_uniqueness_per_field(context, &node.name, &node.fields)
            }
            NodeRef::ObjectTypeExtension(node) => {
                check_arg_uniqueness_per_field(context, &node.name, &node.fields)
            }
            _ => VisitAction::Continue,
        }
    }
}
