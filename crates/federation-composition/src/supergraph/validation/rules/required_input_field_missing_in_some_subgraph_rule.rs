//! Port of `src/supergraph/validation/rules/required-input-field-missing-in-some-subgraph-rule.ts`.

use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use graphql_core::GraphQLError;

/// `RequiredInputFieldMissingInSomeSubgraphRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectTypeField`.
pub struct RequiredInputFieldMissingInSomeSubgraphRule;

impl<'s> SupergraphRule<'s> for RequiredInputFieldMissingInSomeSubgraphRule {
    const NAME: &'static str = "RequiredInputFieldMissingInSomeSubgraphRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(RequiredInputFieldMissingInSomeSubgraphRule)
    }
}

impl<'s> SupergraphVisitor<'s> for RequiredInputFieldMissingInSomeSubgraphRule {
    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        if field_state.r#type.ends_with('!') {
            // if the input object is defined in a single graph, this rule can be ignored
            if input_object_state.by_graph.len() == 1 {
                return Ok(());
            }

            // if the field is defined in all graphs, this rule can be ignored
            if input_object_state.by_graph.len() == field_state.by_graph.len() {
                return Ok(());
            }

            let graphs_with_required_field: Vec<&String> = field_state
                .by_graph
                .iter()
                .filter(|(_, field)| field.r#type.ends_with('!'))
                .map(|(graph, _)| graph)
                .collect();
            let graphs_without_field: Vec<&String> = input_object_state
                .by_graph
                .keys()
                .filter(|graph| !field_state.by_graph.contains_key(*graph))
                .collect();
            let required_in = format!(
                "subgraph{} \"{}\"",
                if graphs_with_required_field.len() > 1 { "s" } else { "" },
                cx.graph_ids_to_names(graphs_with_required_field.iter().copied())?
                    .join("\", \"")
            );
            let missing_in = format!(
                "subgraph{} \"{}\"",
                if graphs_without_field.len() > 1 { "s" } else { "" },
                cx.graph_ids_to_names(graphs_without_field.iter().copied())?
                    .join("\", \"")
            );
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Input object field \"{}.{}\" is required in some subgraphs but does not appear in all subgraphs: it is required in {required_in} but does not appear in {missing_in}",
                    input_object_state.name, field_state.name
                ),
                "REQUIRED_INPUT_FIELD_MISSING_IN_SOME_SUBGRAPH",
            ));
        }

        Ok(())
    }
}
