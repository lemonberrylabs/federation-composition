//! Port of graphql-js `UniqueArgumentNamesRule`.

use crate::ast::{Argument, Kind, NodeRef};
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

use super::group_by;

/// graphql-js `UniqueArgumentNamesRule`: a field or directive is given each
/// argument once.
pub struct UniqueArgumentNamesRule;

impl UniqueArgumentNamesRule {
    pub fn new(_context: &mut SDLValidationContext<'_>) -> Self {
        UniqueArgumentNamesRule
    }
}

fn check_arg_uniqueness(context: &mut SDLValidationContext<'_>, argument_nodes: &[Argument]) {
    let seen_args = group_by(argument_nodes, |arg| arg.name.value.as_str());
    for (arg_name, arg_nodes) in seen_args {
        if arg_nodes.len() > 1 {
            context.report_error(GraphQLError::new(format!(
                "There can be only one argument named \"{arg_name}\"."
            )));
        }
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueArgumentNamesRule {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::Field, Kind::Directive])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::Field(parent_node) => check_arg_uniqueness(context, &parent_node.arguments),
            NodeRef::Directive(parent_node) => check_arg_uniqueness(context, &parent_node.arguments),
            _ => {}
        }
        VisitAction::Continue
    }
}
