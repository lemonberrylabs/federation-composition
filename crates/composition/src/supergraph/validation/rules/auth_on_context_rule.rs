//! Port of `src/supergraph/validation/rules/auth-on-context-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use crate::subgraph::state::{FromContext, SubgraphState};

/// `AuthOnContextRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct AuthOnContextRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for AuthOnContextRule<'s> {
    const NAME: &'static str = "AuthOnContextRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for AuthOnContextRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// An entry of `contextualArgs(fieldState)`.
struct ContextualArg {
    graph_id: String,
    from_context: FromContext,
}

fn contextual_args(field_state: &ObjectTypeFieldState) -> Vec<ContextualArg> {
    todo!()
}

/// The object, interface and union types of the subgraph that provide `context_name`.
fn find_context_providers(subgraph_state: &SubgraphState, context_name: &str) -> Vec<String> {
    todo!()
}

fn create_context_access_requirement_error(graph_name: &str, field_coordinate: &str, context_name: &str) -> GraphQLError {
    todo!()
}
