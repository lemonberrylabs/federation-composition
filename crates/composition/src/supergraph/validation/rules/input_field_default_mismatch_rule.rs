//! Port of `src/supergraph/validation/rules/input-field-default-mismatch-rule.ts`.

use graphql_js::GraphQLError;
use crate::utils::collections::IndexMap;

use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `InputFieldDefaultMismatchRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectTypeField`.
pub struct InputFieldDefaultMismatchRule;

impl<'s> SupergraphRule<'s> for InputFieldDefaultMismatchRule {
    const NAME: &'static str = "InputFieldDefaultMismatchRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(InputFieldDefaultMismatchRule)
    }
}

impl<'s> SupergraphVisitor<'s> for InputFieldDefaultMismatchRule {
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        if field_state.default_value.is_none() {
            return Ok(());
        }

        let mut default_value_to_graphs: IndexMap<&str, Vec<&String>> = IndexMap::default();

        for (graph_name, field) in &field_state.by_graph {
            if let Some(default_value) = &field.default_value {
                default_value_to_graphs
                    .entry(default_value.as_str())
                    .or_default()
                    .push(graph_name);
            }
        }

        if default_value_to_graphs.len() > 1 {
            let mut groups = Vec::new();
            for (default_value, graphs) in &default_value_to_graphs {
                let plural = if graphs.len() > 1 { "s" } else { "" };
                groups.push(format!(
                    "default value {default_value} in subgraph{plural} \"{}\"",
                    cx.graph_ids_to_names(graphs.iter().copied())?.join("\", \"")
                ));
            }
            let rest = &groups[2..];
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Input field \"{}.{}\" has incompatible default values across subgraphs: it has {} but {}{}",
                    input_object_state.name,
                    field_state.name,
                    groups[0],
                    groups[1],
                    if !rest.is_empty() {
                        format!(" and {}", rest.join(" and "))
                    } else {
                        String::new()
                    }
                ),
                "INPUT_FIELD_DEFAULT_MISMATCH",
            ));
        }

        Ok(())
    }
}
