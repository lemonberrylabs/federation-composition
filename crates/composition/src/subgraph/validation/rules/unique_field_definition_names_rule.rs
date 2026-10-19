//! Port of `src/subgraph/validation/rules/unique-field-definition-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::Name;

/// `UniqueFieldDefinitionNamesRule(context)`.
/// Visitor callbacks: `InputObjectTypeDefinition`/`InputObjectTypeExtension`/`InterfaceTypeDefinition`/`InterfaceTypeExtension`/`ObjectTypeDefinition`/`ObjectTypeExtension: checkFieldUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueFieldDefinitionNamesRule {}

impl<'a> SimpleRule<'a> for UniqueFieldDefinitionNamesRule {
    const NAME: &'static str = "UniqueFieldDefinitionNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl UniqueFieldDefinitionNamesRule {
    /// `checkFieldUniqueness(node)`: `node.name` and the names of `node.fields` (field or input
    /// value definitions).
    fn check_field_uniqueness(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str, field_names: &[&Name]) {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueFieldDefinitionNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
