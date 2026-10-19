//! Port of `src/subgraph/validation/rules/elements/list-size.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use graphql_js::Type;

/// `ListSizeRule(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct ListSizeRule {}

impl<'a> SubgraphRule<'a> for ListSizeRule {
    const NAME: &'static str = "ListSizeRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ListSizeRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}

fn is_list_type(type_node: &Type) -> bool {
    match type_node {
        Type::List(_) => true,
        Type::Named(_) => false,
        Type::NonNull(n) => is_list_type(&n.ty),
    }
}

fn is_int_type_or_nullable_int_type(type_node: &Type) -> bool {
    match type_node {
        Type::List(_) => false,
        Type::Named(n) => n.name.value == "Int",
        Type::NonNull(n) => is_int_type_or_nullable_int_type(&n.ty),
    }
}
