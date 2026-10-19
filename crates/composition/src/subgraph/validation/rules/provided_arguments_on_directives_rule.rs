//! Port of `src/subgraph/validation/rules/provided-arguments-on-directives-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::{Type, Value};

/// `ProvidedArgumentsOnDirectivesRule(context)`.
/// Visitor callbacks: `Directive: { leave(directiveNode, _k, _p, _pp, ancestors) }`.
/// Fields: the locals of the TS factory.
pub struct ProvidedArgumentsOnDirectivesRule {}

impl<'a> SimpleRule<'a> for ProvidedArgumentsOnDirectivesRule {
    const NAME: &'static str = "ProvidedArgumentsOnDirectivesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for ProvidedArgumentsOnDirectivesRule {
    fn leave(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}

/// `printValueNode(valueNode)`. The TS throws for a variable, which const SDL values never are.
fn print_value_node(value_node: &Value) -> String {
    todo!()
}

/// `printTypeNode(typeNode, context)`.
fn print_type_node(type_node: &Type, context: &SimpleValidationContext<'_>) -> Option<String> {
    todo!()
}
