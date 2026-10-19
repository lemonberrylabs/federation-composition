//! Port of `src/subgraph/validation/rules/query-root-type-inaccessible-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, OperationType, OperationTypeDefinition, VisitAction, VisitInfo, Visitor};

use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `QueryRootTypeInaccessibleRule(context)`.
/// Visitor callbacks: `SchemaDefinition`, `SchemaExtension`, `ObjectTypeDefinition`.
/// Fields: the locals of the TS factory.
pub struct QueryRootTypeInaccessibleRule {
    root_type_name: String,
}

impl<'a> SubgraphRule<'a> for QueryRootTypeInaccessibleRule {
    const NAME: &'static str = "QueryRootTypeInaccessibleRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        QueryRootTypeInaccessibleRule {
            root_type_name: "Query".to_string(),
        }
    }
}

impl QueryRootTypeInaccessibleRule {
    /// The `SchemaDefinition` / `SchemaExtension` callbacks.
    fn check_operation_types(&mut self, operation_types: &[OperationTypeDefinition]) {
        let non_query_type = operation_types.iter().find(|operation_type| {
            operation_type.operation == OperationType::Query && operation_type.ty.name.value != "Query"
        });

        if let Some(non_query_type) = non_query_type {
            self.root_type_name = non_query_type.ty.name.value.clone();
        }
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for QueryRootTypeInaccessibleRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::SchemaDefinition(node) => self.check_operation_types(&node.operation_types),
            NodeRef::SchemaExtension(node) => self.check_operation_types(&node.operation_types),
            NodeRef::ObjectTypeDefinition(node) => {
                let name = &node.name.value;

                if name != &self.root_type_name {
                    return VisitAction::Continue;
                }

                if node
                    .directives
                    .iter()
                    .any(|directive| cx.is_available_federation_directive("inaccessible", &directive.name.value))
                {
                    cx.report_error(GraphQLError::with_code(
                        "Type \"Query\" is @inaccessible but is the root query type, which must be in the API schema.",
                        "QUERY_ROOT_TYPE_INACCESSIBLE",
                    ));
                }
            }
            _ => {}
        }
        VisitAction::Continue
    }
}
