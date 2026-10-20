//! Port of `src/supergraph/validation/rules/external-missing-on-base-rule.ts`.

use graphql_rs::GraphQLError;
use crate::specifications::federation::FederationVersion;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::format::and_list;
use crate::utils::js::{JsError, JsResult};

/// `ExternalMissingOnBaseRule(context, supergraph)`.
/// Visitor callbacks: `ObjectType`, `ObjectTypeField`.
pub struct ExternalMissingOnBaseRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for ExternalMissingOnBaseRule<'s> {
    const NAME: &'static str = "ExternalMissingOnBaseRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(ExternalMissingOnBaseRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for ExternalMissingOnBaseRule<'s> {
    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        if object_type_state
            .by_graph
            .values()
            .all(|state_in_graph| state_in_graph.external)
        {
            let subgraphs = if object_type_state.by_graph.len() > 1 {
                "subgraphs"
            } else {
                "subgraph"
            };
            // NOTE(port): the TS interpolates `(andList(names), true, '"')`, a comma
            // expression whose value is `'"'`: the subgraph list is computed (and can throw)
            // but the message only shows `"`.
            let names = cx.graph_ids_to_names(object_type_state.by_graph.keys())?;
            let _ = and_list(&names, true, None);
            let shown = "\"";
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type \"{}\" is marked @external on all the subgraphs in which it is listed ({subgraphs} {shown}).",
                    object_type_state.name
                ),
                "EXTERNAL_MISSING_ON_BASE",
            ));
        }

        Ok(())
    }

    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        // Check if the field is marked @external on all the subgraphs in which it is listed.
        let mut external_everywhere = true;
        for (graph_id, field_state_in_graph) in &field_state.by_graph {
            let graph_version = cx
                .subgraph_states
                .get(graph_id)
                .ok_or_else(|| JsError::type_error("Cannot read properties of undefined (reading 'federation')"))?
                .federation
                .version;

            let result = if field_state_in_graph.used_as_key {
                let object_in_graph = object_state
                    .by_graph
                    .get(graph_id)
                    .ok_or_else(|| JsError::type_error("Cannot read properties of undefined (reading 'extension')"))?;
                field_state_in_graph.external && !object_in_graph.extension
            } else if graph_version == FederationVersion::V1_0 {
                // In Fed v1: if a field is provided or required but it's not @key field and it's marked as @external, it's an error
                field_state_in_graph.external && field_state_in_graph.used
            } else {
                field_state_in_graph.external
            };

            if !result {
                external_everywhere = false;
                break;
            }
        }

        if external_everywhere {
            // check if interfaceObject defines the field
            for interface_name in &object_state.interfaces {
                let Some(interface_state) = self.supergraph.interface_types.get(interface_name) else {
                    continue;
                };

                if !interface_state.has_interface_object {
                    continue;
                }

                for (graph_id, interface_state_in_graph) in &interface_state.by_graph {
                    if !interface_state_in_graph.is_interface_object {
                        continue;
                    }

                    let Some(interface_field_state) = interface_state.fields.get(&field_state.name) else {
                        continue;
                    };

                    let Some(interface_field_in_graph) = interface_field_state.by_graph.get(graph_id) else {
                        continue;
                    };

                    if !interface_field_in_graph.external {
                        // interface object defines the field so it's not external in all subgraphs
                        return Ok(());
                    }
                }
            }

            let subgraphs = if field_state.by_graph.len() > 1 {
                "subgraphs"
            } else {
                "subgraph"
            };
            let names = cx.graph_ids_to_names(field_state.by_graph.keys())?;
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Field \"{}.{}\" is marked @external on all the subgraphs in which it is listed ({subgraphs} {}).",
                    object_state.name,
                    field_state.name,
                    and_list(&names, true, Some("\""))
                ),
                "EXTERNAL_MISSING_ON_BASE",
            ));
        }

        Ok(())
    }
}
