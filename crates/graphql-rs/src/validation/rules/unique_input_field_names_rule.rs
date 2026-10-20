//! Port of graphql-js `UniqueInputFieldNamesRule`.

use crate::ast::{Kind, Name, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `UniqueInputFieldNamesRule`: an input object value gives each
/// field once.
pub struct UniqueInputFieldNamesRule<'a> {
    known_name_stack: Vec<IndexMap<&'a str, &'a Name>>,
    known_names: IndexMap<&'a str, &'a Name>,
}

impl<'a> UniqueInputFieldNamesRule<'a> {
    pub fn new(_context: &mut SDLValidationContext<'a>) -> Self {
        UniqueInputFieldNamesRule {
            known_name_stack: Vec::new(),
            known_names: IndexMap::default(),
        }
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueInputFieldNamesRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter_leave(&[Kind::ObjectValue, Kind::ObjectField], &[Kind::ObjectValue])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::ObjectValue(_) => {
                let known_names = std::mem::take(&mut self.known_names);
                self.known_name_stack.push(known_names);
            }
            NodeRef::ObjectField(node) => {
                let field_name = node.name.value.as_str();
                if self.known_names.contains_key(field_name) {
                    context.report_error(GraphQLError::new(format!(
                        "There can be only one input field named \"{field_name}\"."
                    )));
                } else {
                    self.known_names.insert(field_name, &node.name);
                }
            }
            _ => {}
        }
        VisitAction::Continue
    }

    fn leave(
        &mut self,
        node: NodeRef<'a>,
        _context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        if let NodeRef::ObjectValue(_) = node {
            // TS: invariant(prevKnownNames != null); enter pushed it.
            self.known_names = self.known_name_stack.pop().expect("Unexpected invariant triggered.");
        }
        VisitAction::Continue
    }
}
