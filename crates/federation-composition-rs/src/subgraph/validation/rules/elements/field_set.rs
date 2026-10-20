//! Port of `src/subgraph/validation/rules/elements/field-set.ts`.

use graphql_rs::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `FieldSetRules(context)`.
/// Visitor callbacks: `ScalarTypeDefinition`, `ObjectTypeDefinition`, `InterfaceTypeDefinition`, `UnionTypeDefinition`, `EnumTypeDefinition`, `InputObjectTypeDefinition`.
/// Fields: the locals of the TS factory.
pub struct FieldSetRules {}

impl<'a> SubgraphRule<'a> for FieldSetRules {
    const NAME: &'static str = "FieldSetRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        FieldSetRules {}
    }
}

impl FieldSetRules {
    /// `validateFieldSet(node, receivedType)` (the node's name).
    fn validate_field_set(&mut self, context: &mut SubgraphValidationContext<'_>, node_name: &str, received_type: &str) {
        if node_name == "FieldSet" && context.is_available_federation_type("FieldSet") {
            context.report_error(GraphQLError::with_code(
                format!(
                    "Invalid definition for type FieldSet: FieldSet should be a ScalarType but is defined as a {received_type}"
                ),
                "TYPE_DEFINITION_INVALID",
            ));
        }
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for FieldSetRules {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::ScalarTypeDefinition, Kind::ObjectTypeDefinition, Kind::InterfaceTypeDefinition, Kind::UnionTypeDefinition, Kind::EnumTypeDefinition, Kind::InputObjectTypeDefinition])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::ScalarTypeDefinition(node) => {
                if node.name.value == "FieldSet" && cx.is_available_federation_type("FieldSet") {
                    cx.mark_as_federation_definition_replacement("FieldSet");
                }
            }
            NodeRef::ObjectTypeDefinition(node) => self.validate_field_set(cx, &node.name.value, "ObjectType"),
            NodeRef::InterfaceTypeDefinition(node) => self.validate_field_set(cx, &node.name.value, "InterfaceType"),
            NodeRef::UnionTypeDefinition(node) => self.validate_field_set(cx, &node.name.value, "UnionType"),
            NodeRef::EnumTypeDefinition(node) => self.validate_field_set(cx, &node.name.value, "EnumType"),
            NodeRef::InputObjectTypeDefinition(node) => {
                self.validate_field_set(cx, &node.name.value, "InputObjectType")
            }
            _ => {}
        }
        VisitAction::Continue
    }
}
