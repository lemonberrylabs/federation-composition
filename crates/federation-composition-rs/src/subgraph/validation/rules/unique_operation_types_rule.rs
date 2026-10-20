//! Port of `src/subgraph/validation/rules/unique-operation-types-rule.ts`.

use crate::utils::collections::IndexSet;
use graphql_rs::{GraphQLError, NodeRef, OperationType, VisitAction, VisitInfo, Visitor};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_rs::OperationTypeDefinition;

/// `UniqueOperationTypesRule(context)`.
/// Visitor callbacks: `SchemaDefinition`/`SchemaExtension: checkOperationTypes`.
/// Fields: the locals of the TS factory.
pub struct UniqueOperationTypesRule {
    defined_operation_types: IndexSet<OperationType>,
}

impl<'a> SimpleRule<'a> for UniqueOperationTypesRule {
    const NAME: &'static str = "UniqueOperationTypesRule";
    type Visitor = Self;

    fn new(_context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueOperationTypesRule {
            defined_operation_types: IndexSet::default(),
        }
    }
}

impl UniqueOperationTypesRule {
    /// `checkOperationTypes(node)` for a schema definition or extension (its `operationTypes`).
    fn check_operation_types(
        &mut self,
        context: &mut SimpleValidationContext<'_>,
        operation_types: &[OperationTypeDefinition],
    ) {
        for operation_type in operation_types {
            let operation = operation_type.operation;
            let already_defined_operation_type = self.defined_operation_types.contains(&operation);

            if already_defined_operation_type {
                context.report_error(GraphQLError::with_code(
                    format!("There can be only one {} type in schema.", operation.as_str()),
                    "INVALID_GRAPHQL",
                ));
            } else {
                self.defined_operation_types.insert(operation);
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueOperationTypesRule {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::SchemaDefinition, Kind::SchemaExtension])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut SimpleValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::SchemaDefinition(n) => self.check_operation_types(cx, &n.operation_types),
            NodeRef::SchemaExtension(n) => self.check_operation_types(cx, &n.operation_types),
            _ => {}
        }
        VisitAction::Continue
    }
}
