//! Port of `src/subgraph/validation/rules/query-root-type-inaccessible-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `QueryRootTypeInaccessibleRule(context)`.
/// Visitor callbacks: `SchemaDefinition`, `SchemaExtension`, `ObjectTypeDefinition`.
/// Fields: the locals of the TS factory.
pub struct QueryRootTypeInaccessibleRule {}

impl<'a> SubgraphRule<'a> for QueryRootTypeInaccessibleRule {
    const NAME: &'static str = "QueryRootTypeInaccessibleRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for QueryRootTypeInaccessibleRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
