//! Port of graphql-js `UniqueOperationTypesRule`.

use crate::ast::{Kind, NodeRef, OperationType, OperationTypeDefinition};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `UniqueOperationTypesRule`: each operation type is defined once.
pub struct UniqueOperationTypesRule<'a> {
    defined_operation_types: IndexMap<OperationType, &'a OperationTypeDefinition>,
}

impl<'a> UniqueOperationTypesRule<'a> {
    pub fn new(_context: &mut SDLValidationContext<'a>) -> Self {
        // NOTE(port): `existingOperationTypes` is `{}` without a schema.
        UniqueOperationTypesRule {
            defined_operation_types: IndexMap::default(),
        }
    }

    fn check_operation_types(
        &mut self,
        context: &mut SDLValidationContext<'a>,
        operation_types_nodes: &'a [OperationTypeDefinition],
    ) -> VisitAction {
        for operation_type in operation_types_nodes {
            let operation = operation_type.operation;
            if self.defined_operation_types.contains_key(&operation) {
                context.report_error(GraphQLError::new(format!(
                    "There can be only one {} type in schema.",
                    operation.as_str()
                )));
            } else {
                self.defined_operation_types
                    .insert(operation, operation_type);
            }
        }
        VisitAction::Skip
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueOperationTypesRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::SchemaDefinition, Kind::SchemaExtension])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::SchemaDefinition(node) => {
                self.check_operation_types(context, &node.operation_types)
            }
            NodeRef::SchemaExtension(node) => {
                self.check_operation_types(context, &node.operation_types)
            }
            _ => VisitAction::Continue,
        }
    }
}
