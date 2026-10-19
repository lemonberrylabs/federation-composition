//! Port of `src/subgraph/validation/rules/unique-input-field-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::IndexSet;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueInputFieldNamesRule(context)`.
/// Visitor callbacks: `ObjectValue: { enter, leave }`, `ObjectField`.
/// Fields: the locals of the TS factory.
pub struct UniqueInputFieldNamesRule {
    known_name_stack: Vec<IndexSet<String>>,
    known_names: IndexSet<String>,
}

impl<'a> SimpleRule<'a> for UniqueInputFieldNamesRule {
    const NAME: &'static str = "UniqueInputFieldNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueInputFieldNamesRule {
            known_name_stack: Vec::new(),
            known_names: IndexSet::new(),
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueInputFieldNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::ObjectValue(_) => {
                self.known_name_stack.push(std::mem::take(&mut self.known_names));
            }
            NodeRef::ObjectField(node) => {
                let field_name = &node.name.value;
                if self.known_names.contains(field_name) {
                    cx.report_error(GraphQLError::with_code(
                        format!("There can be only one input field named \"{field_name}\"."),
                        "INVALID_GRAPHQL",
                    ));
                } else {
                    self.known_names.insert(field_name.clone());
                }
            }
            _ => {}
        }
        VisitAction::Continue
    }

    fn leave(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::ObjectValue(_) = node {
            // NOTE(port): the TS throws `Assertion failed: nothing else in the stack` on an empty
            // stack, which cannot happen: every leave follows its enter.
            let prev_known_names = self
                .known_name_stack
                .pop()
                .expect("Assertion failed: nothing else in the stack");

            self.known_names = prev_known_names;
        }
        VisitAction::Continue
    }
}
