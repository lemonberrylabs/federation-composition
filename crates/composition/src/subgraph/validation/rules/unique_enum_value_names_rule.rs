//! Port of `src/subgraph/validation/rules/unique-enum-value-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::EnumValueDefinition;

/// `UniqueEnumValueNamesRule(context)`.
/// Visitor callbacks: `EnumTypeDefinition`/`EnumTypeExtension: checkValueUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueEnumValueNamesRule {}

impl<'a> SimpleRule<'a> for UniqueEnumValueNamesRule {
    const NAME: &'static str = "UniqueEnumValueNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl UniqueEnumValueNamesRule {
    /// `checkValueUniqueness(node)` for an enum definition or extension (its `name` and `values`).
    fn check_value_uniqueness(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str, values: &[EnumValueDefinition]) {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueEnumValueNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
