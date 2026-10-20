//! Port of `src/supergraph/validation/rules/external-argument-missing-rule.ts`.

use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use graphql_rs::GraphQLError;

/// `ExternalArgumentMissingRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`.
pub struct ExternalArgumentMissingRule;

impl<'s> SupergraphRule<'s> for ExternalArgumentMissingRule {
    const NAME: &'static str = "ExternalArgumentMissingRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(ExternalArgumentMissingRule)
    }
}

impl<'s> SupergraphVisitor<'s> for ExternalArgumentMissingRule {
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
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
            .keys()
            .filter(|graph| /*arg.type.endsWith('!') && */ field_state.by_graph.get(*graph).map(|f| f.external) != Some(true))
            .collect();
        let external_graphs_without_arg: Vec<&String> = field_state
            .by_graph
            .keys()
            .filter(|graph| {
                !arg_state.by_graph.contains_key(*graph)
                    && field_state.by_graph.get(*graph).map(|f| f.external) == Some(true)
            })
            .collect();

        if external_graphs_without_arg.is_empty() {
            return Ok(());
        }

        let required_in = format!(
            "subgraph{} \"{}\"",
            if graphs_with_required_arg.len() > 1 { "s" } else { "" },
            cx.graph_ids_to_names(graphs_with_required_arg.iter().copied())?
                .join("\", \"")
        );

        let missing_in = format!(
            "subgraph{} \"{}\"",
            if external_graphs_without_arg.len() > 1 { "s" } else { "" },
            cx.graph_ids_to_names(external_graphs_without_arg.iter().copied())?
                .join("\", \"")
        );

        let field_coordinate = format!("{}.{}", object_state.name, field_state.name);
        let arg_coordinate = format!("{}.{}({}:)", object_state.name, field_state.name, arg_state.name);

        cx.report_error(GraphQLError::with_code(
            format!(
                "Field \"{field_coordinate}\" is missing argument \"{arg_coordinate}\" in some subgraphs where it is marked @external: argument \"{arg_coordinate}\" is declared in {required_in} but not in {missing_in} (where \"{field_coordinate}\" is @external)"
            ),
            "EXTERNAL_ARGUMENT_MISSING",
        ));

        Ok(())
    }
}
