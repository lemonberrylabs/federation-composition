//! Port of `src/supergraph/validation/rules/directive-composition-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::directive::DirectiveState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `DirectiveCompositionRule(context, supergraph)`. Fields: `link_identity_to_major_version`, `applied_directive_to_graphs`, `applied_directive_to_link_identities` (built in `new`).
/// Visitor callbacks: `Directive`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct DirectiveCompositionRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for DirectiveCompositionRule<'s> {
    const NAME: &'static str = "DirectiveCompositionRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for DirectiveCompositionRule<'s> {
    fn directive(
        &mut self,
        cx: &mut SupergraphValidationContext,
        directive_state: &'s DirectiveState,
    ) -> JsResult<()> {
        todo!()
    }
}
