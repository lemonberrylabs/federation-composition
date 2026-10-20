//! Port of `src/supergraph/validation/rules/auth-on-context-rule.ts`.

use graphql_rs::GraphQLError;
use crate::subgraph::helpers::parse_fields;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use crate::subgraph::state::{FromContext, SubgraphState, SubgraphType};
use super::auth_on_requires_rule::{ensure_access_to_selection_set, ProvisionedAccess, RequiresCurrentType};

/// `AuthOnContextRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`.
pub struct AuthOnContextRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for AuthOnContextRule<'s> {
    const NAME: &'static str = "AuthOnContextRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(AuthOnContextRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for AuthOnContextRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        let supergraph = self.supergraph;
        for ContextualArg { graph_id, from_context } in contextual_args(field_state) {
            let subgraph_state = cx.subgraph_states.get(&graph_id).cloned();
            let selection_set = parse_fields(&from_context.selection)?;

            let (Some(subgraph_state), Some(selection_set)) = (subgraph_state, selection_set) else {
                continue;
            };

            let mut provisioned_access = ProvisionedAccess::new(object_type_state, field_state);

            for provider_name in find_context_providers(&subgraph_state, &from_context.context) {
                let provider_type = if let Some(t) = supergraph.object_types.get(&provider_name) {
                    Some(RequiresCurrentType::Object(t))
                } else if let Some(t) = supergraph.interface_types.get(&provider_name) {
                    Some(RequiresCurrentType::Interface(t))
                } else {
                    supergraph.union_types.get(&provider_name).map(RequiresCurrentType::Union)
                };

                let Some(provider_type) = provider_type else {
                    continue;
                };
                if ensure_access_to_selection_set(supergraph, provider_type, &selection_set, &mut provisioned_access)?
                    .is_none()
                {
                    continue;
                }

                let graph_name = cx.graph_id_to_name(&graph_id)?;
                let context_graph_name = cx.graph_id_to_name(&graph_id)?;
                cx.report_error(create_context_access_requirement_error(
                    &graph_name,
                    &format!("{}.{}", object_type_state.name, field_state.name),
                    &format!("{context_graph_name}__{}", from_context.context),
                ));
                return Ok(());
            }
        }

        Ok(())
    }
}

/// An entry of `contextualArgs(fieldState)`.
struct ContextualArg {
    graph_id: String,
    from_context: FromContext,
}

fn contextual_args(field_state: &ObjectTypeFieldState) -> Vec<ContextualArg> {
    let mut args = Vec::new();

    for arg in field_state.args.values() {
        for (graph_id, arg_in_graph) in &arg.by_graph {
            if let Some(from_context) = &arg_in_graph.from_context {
                args.push(ContextualArg {
                    graph_id: graph_id.clone(),
                    from_context: from_context.clone(),
                });
            }
        }
    }

    args
}

/// The object, interface and union types of the subgraph that provide `context_name`.
fn find_context_providers(subgraph_state: &SubgraphState, context_name: &str) -> Vec<String> {
    let mut providers = Vec::new();

    for (type_name, type_state) in &subgraph_state.types {
        let contexts = match type_state {
            SubgraphType::Object(t) => Some(&t.contexts),
            SubgraphType::Interface(t) => Some(&t.contexts),
            SubgraphType::Union(t) => Some(&t.contexts),
            _ => None,
        };
        if contexts.is_some_and(|c| c.contains(context_name)) {
            providers.push(type_name.clone());
        }
    }

    providers
}

fn create_context_access_requirement_error(graph_name: &str, field_coordinate: &str, context_name: &str) -> GraphQLError {
    GraphQLError::with_code(
        format!(
            "[{graph_name}] Field \"{field_coordinate}\" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context {context_name} from @fromContext selection set."
        ),
        "MISSING_TRANSITIVE_AUTH_REQUIREMENTS",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_access_requirement_error_message() {
        let e = create_context_access_requirement_error("a", "T.f", "a__ctx");
        assert_eq!(
            e.message,
            "[a] Field \"T.f\" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context a__ctx from @fromContext selection set."
        );
        assert_eq!(e.code(), Some("MISSING_TRANSITIVE_AUTH_REQUIREMENTS"));
    }
}
