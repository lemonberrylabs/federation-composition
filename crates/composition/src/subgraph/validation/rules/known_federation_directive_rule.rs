//! Port of `src/subgraph/validation/rules/known-federation-directive-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `KnownFederationDirectivesRule(context)`.
/// Visitor callbacks: `Directive`.
/// Fields: the locals of the TS factory.
pub struct KnownFederationDirectivesRule {}

impl<'a> SubgraphRule<'a> for KnownFederationDirectivesRule {
    const NAME: &'static str = "KnownFederationDirectivesRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for KnownFederationDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
