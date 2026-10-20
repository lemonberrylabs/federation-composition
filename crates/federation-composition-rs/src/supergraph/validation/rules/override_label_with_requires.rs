//! Port of `src/supergraph/validation/rules/override-label-with-requires.ts`.

use graphql_rs::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `OverrideLabelWithRequiresRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`.
pub struct OverrideLabelWithRequiresRule;

impl<'s> SupergraphRule<'s> for OverrideLabelWithRequiresRule {
    const NAME: &'static str = "OverrideLabelWithRequiresRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(OverrideLabelWithRequiresRule)
    }
}

/// JS truthiness of a `string | null`.
fn is_truthy(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|v| !v.is_empty())
}

impl<'s> SupergraphVisitor<'s> for OverrideLabelWithRequiresRule {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        for (graph_id, field_state_in_graph) in &field_state.by_graph {
            if !is_truthy(&field_state_in_graph.r#override) || !is_truthy(&field_state_in_graph.override_label) {
                continue;
            }

            let Some(source_graph_id) = field_state_in_graph
                .r#override
                .as_deref()
                .and_then(|o| cx.graph_name_to_id(o))
                .filter(|id| !id.is_empty())
            else {
                continue;
            };

            let source_field_state_in_graph = field_state.by_graph.get(&source_graph_id);

            if !source_field_state_in_graph.is_some_and(|f| is_truthy(&f.requires)) {
                continue;
            }

            let graph_name = cx.graph_id_to_name(graph_id)?;
            let source_graph_name = cx.graph_id_to_name(&source_graph_id)?;
            cx.report_error(GraphQLError::with_code(
                format!(
                    "@override with label (progressive override) cannot be used on field \"{t}.{f}\" on subgraph \"{graph_name}\" since \"{t}.{f}\" on \"{source_graph_name}\" is marked with directive \"@requires\". Use non-progressive @override (without label) for this migration.",
                    t = object_type_state.name,
                    f = field_state.name
                ),
                "OVERRIDE_COLLISION_WITH_ANOTHER_DIRECTIVE",
            ));
        }

        Ok(())
    }
}
