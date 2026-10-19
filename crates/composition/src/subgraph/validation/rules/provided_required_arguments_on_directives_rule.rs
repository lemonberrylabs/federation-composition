//! Port of `src/subgraph/validation/rules/provided-required-arguments-on-directives-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::InputValueDefinition;

/// `ProvidedRequiredArgumentsOnDirectivesRule(context)`.
/// Visitor callbacks: `Directive: { leave(directiveNode) }`.
/// Fields: the locals of the TS factory.
pub struct ProvidedRequiredArgumentsOnDirectivesRule {}

impl<'a> SimpleRule<'a> for ProvidedRequiredArgumentsOnDirectivesRule {
    const NAME: &'static str = "ProvidedRequiredArgumentsOnDirectivesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for ProvidedRequiredArgumentsOnDirectivesRule {
    fn leave(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}

fn is_required_argument_node(arg: &InputValueDefinition) -> bool {
    matches!(arg.ty, graphql_js::Type::NonNull(_)) && arg.default_value.is_none()
}
