//! Port of `src/supergraph/validation/rules/interface-key-missing-implementation-type.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use crate::utils::version::satisfies_version_range;

/// `InterfaceKeyMissingImplementationTypeRule(context, supergraph)`.
/// Visitor callbacks: `InterfaceType`.
pub struct InterfaceKeyMissingImplementationTypeRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InterfaceKeyMissingImplementationTypeRule<'s> {
    const NAME: &'static str = "InterfaceKeyMissingImplementationTypeRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(InterfaceKeyMissingImplementationTypeRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceKeyMissingImplementationTypeRule<'s> {
    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        if !interface_state.is_entity || interface_state.has_interface_object {
            // We don't need to check for this rule for non-entities or when at least one subgraph uses @interfaceObject
            return Ok(());
        }

        let mut some_subgraphs_are_missing_implementation = false;

        for interface_state_in_graph in interface_state.by_graph.values() {
            if satisfies_version_range(interface_state_in_graph.version, "< v2.3") {
                continue;
            }

            if interface_state_in_graph.keys.is_empty() {
                continue;
            }

            if interface_state_in_graph.implemented_by.is_empty() && !interface_state_in_graph.is_interface_object {
                some_subgraphs_are_missing_implementation = true;
                break;
            }
        }

        if !some_subgraphs_are_missing_implementation {
            return Ok(()); // No issues detected, supergraph looks good!
        }

        for (graph, interface_state_in_graph) in &interface_state.by_graph {
            if interface_state_in_graph.keys.is_empty() {
                continue;
            }
            // From testing, it seems that the first key is the one that is used in the error message.
            let first_key_fields = &interface_state_in_graph.keys[0].fields;
            let graph_name = cx.graph_id_to_name(graph)?;

            // `.sort()` without a comparator: UTF-16 order, byte order for the ASCII type names.
            let mut types_to_define: Vec<String> = interface_state
                .implemented_by
                .iter()
                .filter(|object_type_name| !interface_state_in_graph.implemented_by.contains(*object_type_name))
                .cloned()
                .collect();
            types_to_define.sort();

            // The template literal reads `typesToDefine.length` before and after
            // `joinWithAnd`, which pops from the array.
            let types_word = if types_to_define.len() > 1 { "types" } else { "type" };
            let joined = join_with_and(&mut types_to_define);
            let them_word = if types_to_define.len() > 1 { "them" } else { "it" };

            cx.report_error(GraphQLError::with_code(
                format!(
                    "[{graph_name}] Interface type \"{name}\" has a resolvable key (@key(fields: \"{first_key_fields}\")) in subgraph \"{graph_name}\" but that subgraph is missing some of the supergraph implementation types of \"{name}\". Subgraph \"{graph_name}\" should define {types_word} {joined} (and have {them_word} implement \"{name}\").",
                    name = interface_state.name
                ),
                "INTERFACE_KEY_MISSING_IMPLEMENTATION_TYPE",
            ));
        }

        Ok(())
    }
}

/// `joinWithAnd(list)`. NOTE(port): the TS `list.pop()` mutates the caller's array.
fn join_with_and(list: &mut Vec<String>) -> String {
    if list.len() <= 2 {
        return format!("\"{}\"", list.join("\" and \""));
    }

    let last = list.pop().unwrap_or_default();

    format!("\"{}\" and \"{last}\"", list.join("\", \""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn join_with_and_formats_and_pops() {
        assert_eq!(join_with_and(&mut s(&[])), "\"\"");
        assert_eq!(join_with_and(&mut s(&["A"])), "\"A\"");
        assert_eq!(join_with_and(&mut s(&["A", "B"])), "\"A\" and \"B\"");
        let mut list = s(&["A", "B", "C"]);
        assert_eq!(join_with_and(&mut list), "\"A\", \"B\" and \"C\"");
        assert_eq!(list, s(&["A", "B"]));
    }
}
