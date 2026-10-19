//! Port of `src/supergraph/validation/rules/extension-with-base.ts`.

use graphql_js::GraphQLError;
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::ExtensionType;
use crate::supergraph::composition::object_type::ObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::{JsError, JsResult};

/// `ExtensionWithBaseRule(context, supergraph)`.
/// Visitor callbacks: `ObjectType`.
pub struct ExtensionWithBaseRule;

impl<'s> SupergraphRule<'s> for ExtensionWithBaseRule {
    const NAME: &'static str = "ExtensionWithBaseRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(ExtensionWithBaseRule)
    }
}

impl<'s> SupergraphVisitor<'s> for ExtensionWithBaseRule {
    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        if object_type_state.name == "Query"
            || object_type_state.name == "Mutation"
            || object_type_state.name == "Subscription"
        {
            return Ok(());
        }

        if !object_type_state.has_definition {
            // It looks like we should not report an error when:
            // - more than one subgraph defines the type
            // - all of the subgraphs use Fed v1 and annotate the type with @extend...
            if object_type_state.by_graph.len() > 1 {
                let mut every = true;
                for (graph_id, meta) in &object_type_state.by_graph {
                    // `context.subgraphStates.get(graphId)!.federation.version`
                    let subgraph = cx.subgraph_states.get(graph_id).ok_or_else(|| {
                        JsError::type_error("Cannot read properties of undefined (reading 'federation')")
                    })?;
                    let matches = if subgraph.federation.version == FederationVersion::V1_0 {
                        meta.extension_type == Some(ExtensionType::Extends)
                    } else {
                        false
                    };
                    if !matches {
                        every = false;
                        break;
                    }
                }
                if every {
                    return Ok(());
                }
            }

            // Valid (but fails to detect)
            //  fed v1 - type @extends
            //  fed v1 - type @extends
            //  fed v1 - extend type
            //
            // Invalid (but fails to detect)
            //  fed v1 - type @extends
            //  fed v1 - extend type
            //

            for graph in object_type_state.by_graph.keys() {
                let graph_name = cx.graph_id_to_name(graph)?;
                cx.report_error(GraphQLError::with_code(
                    format!(
                        "[{graph_name}] Type \"{name}\" is an extension type, but there is no type definition for \"{name}\" in any subgraph.",
                        name = object_type_state.name
                    ),
                    "EXTENSION_WITH_NO_BASE",
                ));
            }
        }

        Ok(())
    }
}
