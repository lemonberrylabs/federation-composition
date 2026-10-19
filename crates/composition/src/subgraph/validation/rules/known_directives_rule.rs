//! Port of `src/subgraph/validation/rules/known-directives-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::{Ancestor, DirectiveLocation, OperationType};

/// `KnownDirectivesRule(context)`.
/// Visitor callbacks: `Directive(node, _key, _parent, _path, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct KnownDirectivesRule {}

impl<'a> SimpleRule<'a> for KnownDirectivesRule {
    const NAME: &'static str = "KnownDirectivesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for KnownDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}

/// `getDirectiveLocationForASTPath(ancestors)`: the location of the node the directive is
/// applied to (`ancestors[ancestors.length - 1]`).
fn get_directive_location_for_ast_path(ancestors: &[Ancestor<'_>]) -> Option<DirectiveLocation> {
    todo!()
}

fn get_directive_location_for_operation(operation: OperationType) -> DirectiveLocation {
    match operation {
        OperationType::Query => DirectiveLocation::Query,
        OperationType::Mutation => DirectiveLocation::Mutation,
        OperationType::Subscription => DirectiveLocation::Subscription,
    }
}
