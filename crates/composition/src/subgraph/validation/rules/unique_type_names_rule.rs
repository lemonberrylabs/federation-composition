//! Port of `src/subgraph/validation/rules/unique-type-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueTypeNamesRule(context)`.
/// Visitor callbacks: `ScalarTypeDefinition`/`ObjectTypeDefinition`/`InterfaceTypeDefinition`/`UnionTypeDefinition`/`EnumTypeDefinition`/`InputObjectTypeDefinition: checkTypeName`.
/// Fields: the locals of the TS factory.
pub struct UniqueTypeNamesRule {}

impl<'a> SimpleRule<'a> for UniqueTypeNamesRule {
    const NAME: &'static str = "UniqueTypeNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl UniqueTypeNamesRule {
    /// `checkTypeName(node)` (its `name`).
    fn check_type_name(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str) {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueTypeNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
