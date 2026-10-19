//! Port of `src/supergraph/validation/rules/field-argument-default-mismatch-rule.ts`.

use graphql_js::GraphQLError;
use crate::utils::collections::IndexMap;

use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `FieldArgumentDefaultMismatchRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`.
pub struct FieldArgumentDefaultMismatchRule;

impl<'s> SupergraphRule<'s> for FieldArgumentDefaultMismatchRule {
    const NAME: &'static str = "FieldArgumentDefaultMismatchRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(FieldArgumentDefaultMismatchRule)
    }
}

impl<'s> SupergraphVisitor<'s> for FieldArgumentDefaultMismatchRule {
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        if arg_state.default_value.is_none() {
            return Ok(());
        }

        let mut default_value_to_graphs: IndexMap<&str, Vec<&String>> = IndexMap::default();

        for (graph_name, arg) in &arg_state.by_graph {
            if let Some(default_value) = &arg.default_value {
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
                    "Argument \"{}.{}({}:)\" has incompatible default values across subgraphs: it has {} but {}{}",
                    object_state.name,
                    field_state.name,
                    arg_state.name,
                    groups[0],
                    groups[1],
                    if !rest.is_empty() {
                        format!(" and {}", rest.join(" and "))
                    } else {
                        String::new()
                    }
                ),
                "FIELD_ARGUMENT_DEFAULT_MISMATCH",
            ));
        }

        Ok(())
    }
}
