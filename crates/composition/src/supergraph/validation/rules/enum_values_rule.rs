//! Port of `src/supergraph/validation/rules/enum-values-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `EnumValuesRule(context, supergraph)`.
/// Visitor callbacks: `EnumType`.
pub struct EnumValuesRule;

impl<'s> SupergraphRule<'s> for EnumValuesRule {
    const NAME: &'static str = "EnumValuesRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(EnumValuesRule)
    }
}

impl<'s> SupergraphVisitor<'s> for EnumValuesRule {
    fn enum_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        enum_type_state: &'s EnumTypeState,
    ) -> JsResult<()> {
        if enum_type_state.referenced_by_input_type && enum_type_state.referenced_by_output_type {
            let mut missing_values: Vec<&String> = Vec::new();
            let total = enum_type_state.by_graph.len();

            for (value_name, value_state) in &enum_type_state.values {
                // If it's not used in all the subgraphs, it's missing in some of them
                if value_state.by_graph.len() != total {
                    // if it's inaccessible, don't report it as missing
                    if !value_state.inaccessible {
                        missing_values.push(value_name);
                    }
                }
            }

            if missing_values.is_empty() {
                return Ok(());
            }

            // NOTE(port): `set.values().next().value` is `undefined` (printed as such) for an
            // empty set.
            let example_input_type = format!(
                "(for example, as type of \"{}\")",
                enum_type_state
                    .input_type_references
                    .first()
                    .map_or("undefined", String::as_str)
            );
            let example_output_type = format!(
                "(for example, as type of \"{}\")",
                enum_type_state
                    .output_type_references
                    .first()
                    .map_or("undefined", String::as_str)
            );

            for missing_value in missing_values {
                let value_state = &enum_type_state.values[missing_value];

                let defined_in = cx.graph_ids_to_names(value_state.by_graph.keys())?;
                let not_defined_in = cx.graph_ids_to_names(
                    enum_type_state
                        .by_graph
                        .keys()
                        .filter(|key| !value_state.by_graph.contains_key(*key)),
                )?;

                let msg = [
                    format!(
                        "Enum type \"{}\" is used as both input type {example_input_type} and output type {example_output_type},",
                        enum_type_state.name
                    ),
                    format!(
                        "but value \"{missing_value}\" is not defined in all the subgraphs defining \"{}\":",
                        enum_type_state.name
                    ),
                    format!(
                        "\"{missing_value}\" is defined in subgraph{} \"{}\"",
                        if defined_in.len() > 1 { "s" } else { "" },
                        defined_in.join("\", \"")
                    ),
                    format!(
                        "but not in subgraph{} \"{}\"",
                        if not_defined_in.len() > 1 { "s" } else { "" },
                        not_defined_in.join("\", \"")
                    ),
                ]
                .join(" ");

                cx.report_error(GraphQLError::with_code(msg, "ENUM_VALUE_MISMATCH"));
            }
        } else if enum_type_state.referenced_by_input_type {
            let mut values_in_common: Vec<&String> = Vec::new();
            let total = enum_type_state.by_graph.len();

            for (value_name, value_state) in &enum_type_state.values {
                // If it's not used in all the subgraphs, it's missing in some of them
                if value_state.by_graph.len() == total {
                    values_in_common.push(value_name);
                }
            }

            if values_in_common.is_empty() {
                cx.report_error(GraphQLError::with_code(
                    format!(
                        "None of the values of enum type \"{}\" are defined consistently in all the subgraphs defining that type. As only values common to all subgraphs are merged, this would result in an empty type.",
                        enum_type_state.name
                    ),
                    "EMPTY_MERGED_ENUM_TYPE",
                ));
            }
        }

        Ok(())
    }
}
