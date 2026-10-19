//! Port of `src/supergraph/validation/rules/override-source-has-override.ts`.

use graphql_js::GraphQLError;
use crate::utils::collections::IndexSet;

use crate::supergraph::composition::object_type::{FieldStateInGraph, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `OverrideSourceHasOverrideRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`.
pub struct OverrideSourceHasOverrideRule;

impl<'s> SupergraphRule<'s> for OverrideSourceHasOverrideRule {
    const NAME: &'static str = "OverrideSourceHasOverrideRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(OverrideSourceHasOverrideRule)
    }
}

impl<'s> SupergraphVisitor<'s> for OverrideSourceHasOverrideRule {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        if field_state.r#override.is_none() {
            return Ok(());
        }

        let graphs_with_override: Vec<(&String, &FieldStateInGraph)> =
            field_state.by_graph.iter().filter(only_with_override).collect();

        if graphs_with_override.len() == 1 {
            return Ok(());
        }

        for (graph, field_state_in_graph) in &graphs_with_override {
            // find any circular references
            let mut visited_graphs: IndexSet<String> = IndexSet::default();
            let override_name = field_state_in_graph
                .r#override
                .as_deref()
                .expect("filtered by onlyWithOverride");
            let mut override_value = cx.graph_name_to_id(override_name);

            // `while (overrideValue)`: graph ids are strings; `""` is falsy.
            while let Some(current) = override_value.clone().filter(|v| !v.is_empty()) {
                if visited_graphs.contains(&current) {
                    let graph_name = cx.graph_id_to_name(graph)?;
                    let override_graph_name = cx.graph_id_to_name(&current)?;
                    cx.report_error(GraphQLError::with_code(
                        format!(
                            "Field \"{}.{}\" on subgraph \"{graph_name}\" is also marked with directive @override in subgraph \"{override_graph_name}\". Only one @override directive is allowed per field.",
                            object_type_state.name, field_state.name
                        ),
                        "OVERRIDE_SOURCE_HAS_OVERRIDE",
                    ));
                    break;
                }

                visited_graphs.insert(current.clone());
                let Some(graph_from_override) = field_state.by_graph.get(&current) else {
                    break;
                };

                // `if (!graphFromOverride.override) break;`
                let Some(next_override) = graph_from_override.r#override.as_deref().filter(|o| !o.is_empty()) else {
                    break;
                };

                override_value = cx.graph_name_to_id(next_override);
            }
        }

        Ok(())
    }
}

/// `onlyWithOverride(entry)`: entries whose `override` is set (`!== null`).
fn only_with_override(entry: &(&String, &FieldStateInGraph)) -> bool {
    entry.1.r#override.is_some()
}
