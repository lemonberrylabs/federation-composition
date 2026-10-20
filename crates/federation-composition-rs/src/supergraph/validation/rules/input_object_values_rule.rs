//! Port of `src/supergraph/validation/rules/input-object-values-rule.ts`.

use graphql_rs::GraphQLError;
use crate::supergraph::composition::input_object_type::InputObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `InputObjectValuesRule(context, supergraph)`.
/// Visitor callbacks: `InputObjectType`.
pub struct InputObjectValuesRule;

impl<'s> SupergraphRule<'s> for InputObjectValuesRule {
    const NAME: &'static str = "InputObjectValuesRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(InputObjectValuesRule)
    }
}

impl<'s> SupergraphVisitor<'s> for InputObjectValuesRule {
    fn input_object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_type_state: &'s InputObjectTypeState,
    ) -> JsResult<()> {
        let mut fields_in_common: Vec<&String> = Vec::new();
        let total = input_object_type_state.by_graph.len();
        for (field_name, field_state) in &input_object_type_state.fields {
            // If it's not used in all the subgraphs, it's missing in some of them
            if field_state.by_graph.len() == total {
                fields_in_common.push(field_name);
            }
        }
        if fields_in_common.is_empty() {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "None of the fields of input object type \"{}\" are consistently defined in all the subgraphs defining that type. As only fields common to all subgraphs are merged, this would result in an empty type.",
                    input_object_type_state.name
                ),
                "EMPTY_MERGED_INPUT_TYPE",
            ));
        }

        Ok(())
    }
}
