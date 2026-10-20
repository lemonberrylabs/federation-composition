//! Port of `src/supergraph/validation/rules/external-type-mismatch-rule.ts`.

use crate::utils::collections::IndexMap;
use graphql_core::GraphQLError;

use crate::specifications::federation::FederationVersion;
use crate::supergraph::composition::object_type::{is_federation_extension, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::format::and_list;
use crate::utils::js::{JsError, JsResult};

/// `ExternalTypeMismatchRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`.
pub struct ExternalTypeMismatchRule;

impl<'s> SupergraphRule<'s> for ExternalTypeMismatchRule {
    const NAME: &'static str = "ExternalTypeMismatchRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(ExternalTypeMismatchRule)
    }
}

impl<'s> SupergraphVisitor<'s> for ExternalTypeMismatchRule {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        if field_state.used_as_key {
            // Ignore fields used as keys
            return Ok(());
        }

        let mut group_by_type: IndexMap<&str, Vec<&String>> = IndexMap::default();
        let mut graphs_with_equal_type: Vec<&String> = Vec::new();

        for (graph_id, field) in &field_state.by_graph {
            let graph_version = cx
                .subgraph_states
                .get(graph_id)
                .ok_or_else(|| JsError::type_error("Cannot read properties of undefined (reading 'federation')"))?
                .federation
                .version;
            let is_external = if graph_version == FederationVersion::V1_0 {
                field.external && {
                    let meta = object_type_state.by_graph.get(graph_id).ok_or_else(|| {
                        JsError::type_error("Cannot read properties of undefined (reading 'extensionType')")
                    })?;
                    is_federation_extension(meta, graph_version)
                }
            } else {
                field.external
            };
            if !is_external {
                graphs_with_equal_type.push(graph_id);

                continue;
            }

            if field.r#type == field_state.r#type {
                graphs_with_equal_type.push(graph_id);
                continue;
            }

            group_by_type.entry(field.r#type.as_str()).or_default().push(graph_id);
        }

        if !group_by_type.is_empty() && !graphs_with_equal_type.is_empty() {
            let mut groups = Vec::new();
            for (ty, graphs) in &group_by_type {
                let plural = if graphs.len() > 1 { "s" } else { "" };
                groups.push(format!(
                    "type \"{ty}\" in subgraph{plural} {}",
                    and_list(&cx.graph_ids_to_names(graphs.iter().copied())?, true, Some("\""))
                ));
            }
            let first = &groups[0];
            let rest = &groups[1..];

            let non_external = format!(
                "type \"{}\" in subgraph{} {}",
                field_state.r#type,
                if graphs_with_equal_type.len() > 1 { "s" } else { "" },
                and_list(
                    &cx.graph_ids_to_names(graphs_with_equal_type.iter().copied())?,
                    true,
                    Some("\"")
                )
            );

            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type of field \"{}.{}\" is incompatible across subgraphs (where marked @external): it has {non_external} but {first}{}",
                    object_type_state.name,
                    field_state.name,
                    if !rest.is_empty() {
                        format!(" and {}", rest.join(" and "))
                    } else {
                        String::new()
                    }
                ),
                "EXTERNAL_TYPE_MISMATCH",
            ));
        }

        Ok(())
    }
}
