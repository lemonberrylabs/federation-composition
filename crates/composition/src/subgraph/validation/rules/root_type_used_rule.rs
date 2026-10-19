//! Port of `src/subgraph/validation/rules/root-type-used-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, OperationType, VisitAction, VisitInfo, Visitor};
use indexmap::IndexSet;

use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use graphql_js::{Definition, OperationTypeDefinition};

/// `RootTypeUsedRule(context)`.
/// Visitor callbacks: `SchemaDefinition`, `SchemaExtension`.
/// Fields: the locals of the TS factory.
pub struct RootTypeUsedRule<'a> {
    definitions: &'a [Definition],
}

impl<'a> SubgraphRule<'a> for RootTypeUsedRule<'a> {
    const NAME: &'static str = "RootTypeUsedRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        RootTypeUsedRule {
            definitions: &context.get_document().definitions,
        }
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for RootTypeUsedRule<'a> {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::SchemaDefinition(node) => validate_schema_node(&node.operation_types, self.definitions, cx),
            NodeRef::SchemaExtension(node) => validate_schema_node(&node.operation_types, self.definitions, cx),
            _ => {}
        }
        VisitAction::Continue
    }
}

/// `Record<OperationTypeNode, string | null>` returned by `findDefaultRootTypes`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DefaultRootTypes {
    query: Option<String>,
    mutation: Option<String>,
    subscription: Option<String>,
}

impl DefaultRootTypes {
    fn get(&self, operation: OperationType) -> Option<&str> {
        match operation {
            OperationType::Query => self.query.as_deref(),
            OperationType::Mutation => self.mutation.as_deref(),
            OperationType::Subscription => self.subscription.as_deref(),
        }
    }
}

fn find_default_root_types(definitions: &[Definition]) -> DefaultRootTypes {
    let mut found_root_types = DefaultRootTypes::default();

    let mut found: IndexSet<OperationType> = IndexSet::new();

    for definition in definitions {
        if found.len() == 3 {
            break;
        }

        if graphql_js::is_type_definition_node(definition) {
            let name = definition.name().map(|n| n.value.as_str());
            if name == Some("Query") {
                found_root_types.query = Some("Query".to_string());
                found.insert(OperationType::Query);
            } else if name == Some("Mutation") {
                found_root_types.mutation = Some("Mutation".to_string());
                found.insert(OperationType::Mutation);
            } else if name == Some("Subscription") {
                found_root_types.subscription = Some("Subscription".to_string());
                found.insert(OperationType::Subscription);
            }
        }
    }

    found_root_types
}

/// `validateSchemaNode(node, definitions, context)` for a schema definition or extension (its
/// `operationTypes`).
fn validate_schema_node(
    operation_types: &[OperationTypeDefinition],
    definitions: &[Definition],
    context: &mut SubgraphValidationContext<'_>,
) {
    let defined_default_root_types = find_default_root_types(definitions);
    for operation_type in operation_types {
        let operation = operation_type.operation.as_str();
        let default_root_type = defined_default_root_types.get(operation_type.operation);
        let used_root_type_name = &operation_type.ty.name.value;

        if let Some(default_root_type) = default_root_type {
            if default_root_type != used_root_type_name {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "The schema has a type named \"{default_root_type}\" but it is not set as the {operation} root type (\"{used_root_type_name}\" is instead): this is not supported by federation. If a root type does not use its default name, there should be no other type with that default name."
                    ),
                    format!("ROOT_{}_USED", operation.to_uppercase()),
                ));
            }
        }
    }
}
