//! Port of `src/supergraph/validation/rules/required-argument-missing-in-some-subgraph-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `RequiredArgumentMissingInSomeSubgraph(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`.
pub struct RequiredArgumentMissingInSomeSubgraph;

impl<'s> SupergraphRule<'s> for RequiredArgumentMissingInSomeSubgraph {
    const NAME: &'static str = "RequiredArgumentMissingInSomeSubgraph";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(RequiredArgumentMissingInSomeSubgraph)
    }
}

impl<'s> SupergraphVisitor<'s> for RequiredArgumentMissingInSomeSubgraph {
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        if arg_state.r#type.ends_with('!') {
            // If the argument and the field are defined in a single graph, this rule can be ignored
            if arg_state.by_graph.len() == 1 && field_state.by_graph.len() == arg_state.by_graph.len() {
                return Ok(());
            }

            // The argument is defined in all graphs implementing the field
            if field_state.by_graph.len() == arg_state.by_graph.len() {
                return Ok(());
            }

            let graphs_with_required_arg: Vec<&String> = arg_state
                .by_graph
                .iter()
                .filter(|(_, arg)| arg.r#type.ends_with('!'))
                .map(|(graph, _)| graph)
                .collect();
            let graphs_without_arg: Vec<&String> = field_state
                .by_graph
                .keys()
                .filter(|graph| !arg_state.by_graph.contains_key(*graph))
                .collect();

            let required_in = format!(
                "subgraph{} \"{}\"",
                if graphs_with_required_arg.len() > 1 { "s" } else { "" },
                cx.graph_ids_to_names(graphs_with_required_arg.iter().copied())?.join("\", \"")
            );

            let missing_in = format!(
                "subgraph{} \"{}\"",
                if graphs_without_arg.len() > 1 { "s" } else { "" },
                cx.graph_ids_to_names(graphs_without_arg.iter().copied())?.join("\", \"")
            );

            cx.report_error(GraphQLError::with_code(
                format!(
                    "Argument \"{}.{}({}:)\" is required in some subgraphs but does not appear in all subgraphs: it is required in {required_in} but does not appear in {missing_in}",
                    object_state.name, field_state.name, arg_state.name
                ),
                "REQUIRED_ARGUMENT_MISSING_IN_SOME_SUBGRAPH",
            ));
        }

        Ok(())
    }
}
