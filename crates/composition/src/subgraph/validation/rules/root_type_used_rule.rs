//! Port of `src/subgraph/validation/rules/root-type-used-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use graphql_js::{Definition, OperationTypeDefinition};

/// `RootTypeUsedRule(context)`.
/// Visitor callbacks: `SchemaDefinition`, `SchemaExtension`.
/// Fields: the locals of the TS factory.
pub struct RootTypeUsedRule {}

impl<'a> SubgraphRule<'a> for RootTypeUsedRule {
    const NAME: &'static str = "RootTypeUsedRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for RootTypeUsedRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}

/// `Record<OperationTypeNode, string | null>` returned by `findDefaultRootTypes`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DefaultRootTypes {
    query: Option<String>,
    mutation: Option<String>,
    subscription: Option<String>,
}

fn find_default_root_types(definitions: &[Definition]) -> DefaultRootTypes {
    todo!()
}

/// `validateSchemaNode(node, definitions, context)` for a schema definition or extension (its
/// `operationTypes`).
fn validate_schema_node(
    operation_types: &[OperationTypeDefinition],
    definitions: &[Definition],
    context: &mut SubgraphValidationContext<'_>,
) {
    todo!()
}
