//! Port of `src/subgraph/validation/rules/elements/provides.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `ProvidesRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(directiveNode)`. The `visitFields` interceptors become a local struct implementing `VisitFieldsInterceptor`.
/// Fields: the locals of the TS factory.
pub struct ProvidesRules {}

impl<'a> SubgraphRule<'a> for ProvidesRules {
    const NAME: &'static str = "ProvidesRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ProvidesRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
