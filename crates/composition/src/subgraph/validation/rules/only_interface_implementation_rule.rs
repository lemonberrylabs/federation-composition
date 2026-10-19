//! Port of `src/subgraph/validation/rules/only-interface-implementation-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::{Kind, NamedType};

/// `OnlyInterfaceImplementationRule(context)`.
/// Visitor callbacks: `ObjectTypeDefinition: check`, `ObjectTypeExtension: check`.
/// Fields: the locals of the TS factory.
pub struct OnlyInterfaceImplementationRule {}

impl<'a> SimpleRule<'a> for OnlyInterfaceImplementationRule {
    const NAME: &'static str = "OnlyInterfaceImplementationRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl OnlyInterfaceImplementationRule {
    /// `fillTypeNameToKindMap()` (lazily, on the first `check`).
    fn fill_type_name_to_kind_map(&mut self, context: &SimpleValidationContext<'_>) {
        todo!()
    }

    /// `findKindByName(typeName)`.
    fn find_kind_by_name(&mut self, context: &SimpleValidationContext<'_>, type_name: &str) -> Option<Kind> {
        todo!()
    }

    /// `check(node)` for an object type definition or extension (its `name` and `interfaces`).
    fn check(&mut self, context: &mut SimpleValidationContext<'_>, name: &str, interfaces: &[NamedType]) {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for OnlyInterfaceImplementationRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
