//! Port of `src/subgraph/validation/rules/elements/inaccessible.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `InaccessibleRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct InaccessibleRules {}

impl<'a> SubgraphRule<'a> for InaccessibleRules {
    const NAME: &'static str = "InaccessibleRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for InaccessibleRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
