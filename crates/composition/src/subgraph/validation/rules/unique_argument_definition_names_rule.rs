//! Port of `src/subgraph/validation/rules/unique-argument-definition-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::{FieldDefinition, InputValueDefinition};

/// `UniqueArgumentDefinitionNamesRule(context)`.
/// Visitor callbacks: `DirectiveDefinition`, `InterfaceTypeDefinition`/`InterfaceTypeExtension`/`ObjectTypeDefinition`/`ObjectTypeExtension: checkArgUniquenessPerField`.
/// Fields: the locals of the TS factory.
pub struct UniqueArgumentDefinitionNamesRule {}

impl<'a> SimpleRule<'a> for UniqueArgumentDefinitionNamesRule {
    const NAME: &'static str = "UniqueArgumentDefinitionNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl UniqueArgumentDefinitionNamesRule {
    /// `checkArgUniquenessPerField(typeNode)` (its `name` and `fields`).
    fn check_arg_uniqueness_per_field(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str, fields: &[FieldDefinition]) {
        todo!()
    }

    fn check_arg_uniqueness(
        &mut self,
        context: &mut SimpleValidationContext<'_>,
        parent_name: &str,
        argument_nodes: &[InputValueDefinition],
    ) {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueArgumentDefinitionNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
