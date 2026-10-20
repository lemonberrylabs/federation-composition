//! Port of graphql-js `UniqueDirectiveNamesRule`.

use crate::ast::{Kind, Name, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `UniqueDirectiveNamesRule`: each directive is defined once.
pub struct UniqueDirectiveNamesRule<'a> {
    known_directive_names: IndexMap<&'a str, &'a Name>,
}

impl<'a> UniqueDirectiveNamesRule<'a> {
    pub fn new(_context: &mut SDLValidationContext<'a>) -> Self {
        // NOTE(port): no schema, so no "already exists in the schema" check.
        UniqueDirectiveNamesRule {
            known_directive_names: IndexMap::default(),
        }
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueDirectiveNamesRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::DirectiveDefinition])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let NodeRef::DirectiveDefinition(node) = node else {
            return VisitAction::Continue;
        };
        let directive_name = node.name.value.as_str();
        if self.known_directive_names.contains_key(directive_name) {
            context.report_error(GraphQLError::new(format!(
                "There can be only one directive named \"@{directive_name}\"."
            )));
        } else {
            self.known_directive_names
                .insert(directive_name, &node.name);
        }
        VisitAction::Skip
    }
}
