//! Port of `src/subgraph/validation/rules/unique-argument-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::IndexSet;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::Argument;

/// `UniqueArgumentNamesRule(context)`.
/// Visitor callbacks: `Field: checkArgUniqueness`, `Directive: checkArgUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueArgumentNamesRule {}

impl<'a> SimpleRule<'a> for UniqueArgumentNamesRule {
    const NAME: &'static str = "UniqueArgumentNamesRule";

    fn new(_context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueArgumentNamesRule {}
    }
}

impl UniqueArgumentNamesRule {
    /// `checkArgUniqueness(parentNode)` (its `arguments`).
    fn check_arg_uniqueness(&mut self, context: &mut SimpleValidationContext<'_>, argument_nodes: &[Argument]) {
        let mut seen_args: IndexSet<&str> = IndexSet::new();

        for argument_node in argument_nodes {
            if seen_args.contains(argument_node.name.value.as_str()) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "There can be only one argument named \"{}\".",
                        argument_node.name.value
                    ),
                    "INVALID_GRAPHQL",
                ));
            } else {
                seen_args.insert(&argument_node.name.value);
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueArgumentNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::Field(n) => self.check_arg_uniqueness(cx, &n.arguments),
            NodeRef::Directive(n) => self.check_arg_uniqueness(cx, &n.arguments),
            _ => {}
        }
        VisitAction::Continue
    }
}
