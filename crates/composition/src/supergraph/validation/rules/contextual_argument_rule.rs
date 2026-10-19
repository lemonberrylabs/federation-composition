//! Port of `src/supergraph/validation/rules/contextual-argument-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `ContextualArgumentRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`.
pub struct ContextualArgumentRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for ContextualArgumentRule<'s> {
    const NAME: &'static str = "ContextualArgumentRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(ContextualArgumentRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for ContextualArgumentRule<'s> {
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        if arg_state.from_context.is_none() {
            return Ok(());
        }

        let is_required_somewhere = arg_state.by_graph.values().any(|arg_in_graph| {
            arg_in_graph.from_context.is_none()
                && arg_in_graph.r#type.ends_with('!')
                // `!argInGraph.defaultValue`: `""` is falsy too.
                && arg_in_graph.default_value.as_deref().map_or(true, str::is_empty)
        });

        if !is_required_somewhere {
            return Ok(());
        }

        let coordinate = format!("{}.{}({}:)", object_state.name, field_state.name, arg_state.name);

        cx.report_error(GraphQLError::with_code(
            format!(
                "Argument \"{coordinate}\" is contextual in at least one subgraph but in \"{coordinate}\" it does not have @fromContext, is not nullable and has no default value."
            ),
            "CONTEXTUAL_ARGUMENT_NOT_CONTEXTUAL_IN_ALL_SUBGRAPHS",
        ));

        Ok(())
    }
}
