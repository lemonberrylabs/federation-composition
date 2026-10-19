//! Port of `src/supergraph/validation/rules/interface-field-no-implementation-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::ObjectTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::{JsError, JsResult};
use crate::supergraph::composition::interface_type::InterfaceTypeState;

/// `InterfaceFieldNoImplementationRule(context, supergraph)`.
/// Visitor callbacks: `ObjectType`.
pub struct InterfaceFieldNoImplementationRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for InterfaceFieldNoImplementationRule<'s> {
    const NAME: &'static str = "InterfaceFieldNoImplementationRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(InterfaceFieldNoImplementationRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceFieldNoImplementationRule<'s> {
    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        if object_type_state.interfaces.is_empty() {
            return Ok(());
        }

        for interface_name in &object_type_state.interfaces {
            let interface_type_state = match get_type_from_supergraph(self.supergraph, interface_name) {
                None => {
                    return Err(JsError::error("Expected an interface to exist in supergraph state"));
                }
                Some(FoundType::Interface(i)) => i,
                Some(FoundType::Other) => {
                    // Covered by TYPE_KIND_MISMATCH rule
                    return Ok(());
                }
            };

            let mut non_required_fields: Vec<&String> = Vec::new();

            for (graph, interface_state_in_graph) in &interface_type_state.by_graph {
                if !interface_state_in_graph.is_interface_object {
                    continue;
                }

                for (field_name, interface_field_state) in &interface_type_state.fields {
                    let Some(interface_field_state_in_graph) = interface_field_state.by_graph.get(graph) else {
                        continue;
                    };

                    if interface_field_state_in_graph.external {
                        continue;
                    }

                    non_required_fields.push(field_name);
                }
            }

            for (field_name, interface_field_state) in &interface_type_state.fields {
                // skip fields that are defined in interface objects or in interface entities
                if non_required_fields.contains(&field_name) {
                    continue;
                }

                // TODO: detect if a field is missing in a non-entity object type definition
                if object_type_state.fields.contains_key(field_name) && object_type_state.is_entity {
                    continue;
                }

                for (graph, object_type_in_graph) in &object_type_state.by_graph {
                    // check if object in the graph, implements an interface of the same name
                    if !object_type_in_graph.interfaces.contains(interface_name) {
                        // if not, continue
                        continue;
                    }

                    let object_field_state = object_type_state.fields.get(field_name);

                    // if not, make sure it implements the field
                    if object_field_state.is_none() {
                        let interface_field_defined_in_graphs =
                            cx.graph_ids_to_names(interface_field_state.by_graph.keys())?;
                        let declared_in = if interface_field_defined_in_graphs.len() == 1 {
                            format!("subgraph \"{}\"", interface_field_defined_in_graphs[0])
                        } else {
                            format!(
                                "subgraphs {}",
                                interface_field_defined_in_graphs
                                    .iter()
                                    .map(|g| format!("\"{g}\""))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        };

                        let graph_name = cx.graph_id_to_name(graph)?;
                        cx.report_error(GraphQLError::with_code(
                            format!(
                                "Interface field \"{interface_name}.{field_name}\" is declared in {declared_in} but type \"{}\", which implements \"{interface_name}\" in subgraph \"{graph_name}\" does not have field \"{field_name}\".",
                                object_type_state.name
                            ),
                            "INTERFACE_FIELD_NO_IMPLEM",
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

/// What `getTypeFromSupergraph` found: the callers only distinguish an interface from the
/// other kinds it searches.
enum FoundType<'s> {
    Interface(&'s InterfaceTypeState),
    /// An object or union type.
    Other,
}

/// `getTypeFromSupergraph(state, name)`: `objectTypes ?? interfaceTypes ?? unionTypes`.
fn get_type_from_supergraph<'s>(state: &'s SupergraphState, name: &str) -> Option<FoundType<'s>> {
    if state.object_types.contains_key(name) {
        return Some(FoundType::Other);
    }
    if let Some(interface) = state.interface_types.get(name) {
        return Some(FoundType::Interface(interface));
    }
    if state.union_types.contains_key(name) {
        return Some(FoundType::Other);
    }
    None
}
