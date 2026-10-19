//! Port of `src/subgraph/validation/rules/unique-operation-types-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::OperationTypeDefinition;

/// `UniqueOperationTypesRule(context)`.
/// Visitor callbacks: `SchemaDefinition`/`SchemaExtension: checkOperationTypes`.
/// Fields: the locals of the TS factory.
pub struct UniqueOperationTypesRule {}

impl<'a> SimpleRule<'a> for UniqueOperationTypesRule {
    const NAME: &'static str = "UniqueOperationTypesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl UniqueOperationTypesRule {
    /// `checkOperationTypes(node)` for a schema definition or extension (its `operationTypes`).
    fn check_operation_types(&mut self, context: &mut SimpleValidationContext<'_>, operation_types: &[OperationTypeDefinition]) {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueOperationTypesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
