//! Port of `src/subgraph/validation/rules/elements/key.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `KeyRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(directiveNode)`. The `visitFields` interceptors become a local struct implementing `VisitFieldsInterceptor`.
/// Fields: the locals of the TS factory.
pub struct KeyRules {}

impl<'a> SubgraphRule<'a> for KeyRules {
    const NAME: &'static str = "KeyRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for KeyRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
