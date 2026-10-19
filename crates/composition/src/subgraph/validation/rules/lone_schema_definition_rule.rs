//! Port of `src/subgraph/validation/rules/lone-schema-definition-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `LoneSchemaDefinitionRule(context)`.
/// Visitor callbacks: `SchemaDefinition`.
/// Fields: the locals of the TS factory.
pub struct LoneSchemaDefinitionRule {}

impl<'a> SimpleRule<'a> for LoneSchemaDefinitionRule {
    const NAME: &'static str = "LoneSchemaDefinitionRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for LoneSchemaDefinitionRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
