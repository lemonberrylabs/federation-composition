//! Port of `src/subgraph/validation/rules/elements/field-set.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `FieldSetRules(context)`.
/// Visitor callbacks: `ScalarTypeDefinition`, `ObjectTypeDefinition`, `InterfaceTypeDefinition`, `UnionTypeDefinition`, `EnumTypeDefinition`, `InputObjectTypeDefinition`.
/// Fields: the locals of the TS factory.
pub struct FieldSetRules {}

impl<'a> SubgraphRule<'a> for FieldSetRules {
    const NAME: &'static str = "FieldSetRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl FieldSetRules {
    /// `validateFieldSet(node, receivedType)` (the node's name).
    fn validate_field_set(&mut self, context: &mut SubgraphValidationContext<'_>, node_name: &str, received_type: &str) {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for FieldSetRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
