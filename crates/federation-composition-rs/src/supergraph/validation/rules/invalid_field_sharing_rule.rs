//! Port of `src/supergraph/validation/rules/invalid-field-sharing-rule.ts`.

use graphql_rs::GraphQLError;
use crate::specifications::federation::FederationVersion;
use crate::supergraph::composition::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::format::and_list;
use crate::utils::js::JsResult;

/// `InvalidFieldSharingRule(context, supergraphState)`.
/// Visitor callbacks: `ObjectTypeField`, `InterfaceTypeField`.
pub struct InvalidFieldSharingRule<'s> {
    supergraph: &'s SupergraphState,
    /// PERF: not in the TS. The result of the "is it the Subscription type" check for the last
    /// object type seen. The check depends only on the object type, and the TS repeats it for
    /// every field (a lookup per subgraph of the type).
    last_subscription_check: Option<(&'s ObjectTypeState, bool)>,
}

impl<'s> SupergraphRule<'s> for InvalidFieldSharingRule<'s> {
    const NAME: &'static str = "InvalidFieldSharingRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(InvalidFieldSharingRule {
            supergraph,
            last_subscription_check: None,
        })
    }
}

impl<'s> InvalidFieldSharingRule<'s> {
    /// `objectTypeState.byGraph.keys().some(graph => subgraph(graph).schema.subscriptionType === name)`,
    /// memoized for consecutive fields of the same object type.
    fn is_subscription_type(&mut self, cx: &SupergraphValidationContext, object_type_state: &'s ObjectTypeState) -> bool {
        if let Some((last, is_subscription)) = self.last_subscription_check {
            if std::ptr::eq(last, object_type_state) {
                return is_subscription;
            }
        }
        let is_subscription = object_type_state.by_graph.keys().any(|graph| {
            cx.subgraph_states
                .get(graph)
                .and_then(|s| s.schema.subscription_type())
                == Some(object_type_state.name.as_str())
        });
        self.last_subscription_check = Some((object_type_state, is_subscription));
        is_subscription
    }
}

/// `if (fieldHasOverride) { const overrideGraphId = context.graphNameToId(fieldHasOverride);
/// if (overrideGraphId && fieldState.byGraph.has(overrideGraphId)) continue; }`: whether the
/// field's `@override` is effective (the overridden graph defines the field).
fn is_effective_override(
    cx: &SupergraphValidationContext,
    field_has_override: Option<&str>,
    defines_field: impl Fn(&str) -> bool,
) -> bool {
    let Some(field_has_override) = field_has_override.filter(|o| !o.is_empty()) else {
        return false;
    };
    match cx.graph_name_to_id(field_has_override) {
        Some(override_graph_id) if !override_graph_id.is_empty() => defines_field(&override_graph_id),
        _ => false,
    }
}

impl<'s> SupergraphVisitor<'s> for InvalidFieldSharingRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        // check if it's Subscription
        if self.is_subscription_type(cx, object_type_state) {
            if field_state.by_graph.len() > 1 {
                cx.report_error(GraphQLError::with_code(
                    "Fields on root level subscription object cannot be marked as shareable",
                    "INVALID_FIELD_SHARING",
                ));
            }
            return Ok(());
        }

        if field_state.used_as_key && field_state.by_graph.values().all(|field| field.used_as_key) {
            // If the field is used as a key in all graphs, we don't have to check if it's resolvable by multiple subgraphs.
            return Ok(());
        }

        let mut non_sharable_in: Vec<&String> = Vec::new();
        let mut resolvable_in: Vec<&String> = Vec::new();

        for (graph_id, field) in &field_state.by_graph {
            let field_is_shareable = field.shareable;
            let field_is_external = field.external;
            let field_is_used_as_key = field.used_as_key;

            if field_is_external {
                continue;
            }

            if is_effective_override(cx, field.r#override.as_deref(), |id| field_state.by_graph.contains_key(id)) {
                // if a field tries to override some graph, check if it actually exists there.
                // if it does, exclude it from invalid-field-sharing rule as override is effective.
                continue;
            }

            let fed_v1_field_in_extension = field.version == FederationVersion::V1_0 && field.extension;

            if field_is_shareable || field_is_used_as_key || (object_type_state.is_entity && fed_v1_field_in_extension) {
                resolvable_in.push(graph_id);
                continue;
            }

            non_sharable_in.push(graph_id);
            resolvable_in.push(graph_id);
        }

        // [Graph ID, Interface name]
        let mut interface_object_field_in: Vec<(&String, &String)> = Vec::new();

        if !non_sharable_in.is_empty() {
            // Check if there are any @interfaceObject types that implement the field
            for interface_name in &object_type_state.interfaces {
                let Some(interface_state) = self.supergraph.interface_types.get(interface_name) else {
                    continue;
                };
                if !interface_state.has_interface_object {
                    continue;
                }

                let Some(interface_object_field_state) = interface_state.fields.get(&field_state.name) else {
                    continue;
                };
                if interface_object_field_state.used_as_key {
                    continue;
                }

                for (graph_id, field) in &interface_object_field_state.by_graph {
                    let is_interface_object = interface_state
                        .by_graph
                        .get(graph_id)
                        .is_some_and(|g| g.is_interface_object);

                    if !is_interface_object {
                        continue;
                    }

                    let field_is_shareable = field.shareable;
                    let field_is_external = field.external;
                    let field_is_used_as_key = field.used_as_key;

                    if field_is_external {
                        continue;
                    }

                    if is_effective_override(cx, field.r#override.as_deref(), |id| field_state.by_graph.contains_key(id)) {
                        // if a field tries to override some graph, check if it actually exists there.
                        // if it does, exclude it from invalid-field-sharing rule as override is effective.
                        continue;
                    }

                    interface_object_field_in.push((graph_id, interface_name));

                    if field_is_shareable || field_is_used_as_key {
                        resolvable_in.push(graph_id);
                        continue;
                    }

                    non_sharable_in.push(graph_id);
                    resolvable_in.push(graph_id);
                }
            }
        }

        if !non_sharable_in.is_empty() && resolvable_in.len() > 1 {
            let is_non_sharable_in_all = resolvable_in.iter().all(|graph_id| non_sharable_in.contains(graph_id));

            let mut resolved_from = Vec::new();
            for graph_id in &resolvable_in {
                let name = cx.graph_id_to_name(graph_id)?;

                let interface_object_field = interface_object_field_in.iter().find(|(g, _)| g == graph_id);

                resolved_from.push(match interface_object_field {
                    None => format!("\"{name}\""),
                    Some((_, interface_name)) => format!(
                        "\"{name}\" (through @interfaceObject field \"{interface_name}.{}\")",
                        field_state.name
                    ),
                });
            }
            let resolved_from = and_list(&resolved_from, false, None);

            let defined_in = if is_non_sharable_in_all {
                "all of them".to_string()
            } else {
                format!(
                    "subgraph{} {}",
                    if non_sharable_in.len() > 1 { "s" } else { "" },
                    and_list(&cx.graph_ids_to_names(non_sharable_in.iter().copied())?, true, Some("\""))
                )
            };

            let message = format!(
                "Non-shareable field \"{}.{}\" is resolved from multiple subgraphs: it is resolved from subgraphs {resolved_from} and defined as non-shareable in {defined_in}",
                object_type_state.name, field_state.name
            );

            cx.report_error(GraphQLError::with_code(message, "INVALID_FIELD_SHARING"));
        }

        Ok(())
    }

    fn interface_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_type_state: &'s InterfaceTypeState,
        field_state: &'s InterfaceTypeFieldState,
    ) -> JsResult<()> {
        if !interface_type_state.has_interface_object {
            return Ok(());
        }

        // detect a non-shareable field in a type with @interfaceObject and implementations of the interface

        let mut non_sharable_in: Vec<&String> = Vec::new();
        let mut resolvable_in: Vec<&String> = Vec::new();
        let mut interface_object_field_in: Vec<&String> = Vec::new();

        for (graph_id, field) in &field_state.by_graph {
            let is_interface_object = interface_type_state
                .by_graph
                .get(graph_id)
                .is_some_and(|g| g.is_interface_object);

            if !is_interface_object {
                continue;
            }

            let field_is_shareable = field.shareable;
            let field_is_external = field.external;
            let field_is_used_as_key = field.used_as_key;

            if field_is_external {
                continue;
            }

            if is_effective_override(cx, field.r#override.as_deref(), |id| field_state.by_graph.contains_key(id)) {
                // if a field tries to override some graph, check if it actually exists there.
                // if it does, exclude it from invalid-field-sharing rule as override is effective.
                continue;
            }

            if field_is_shareable || field_is_used_as_key {
                resolvable_in.push(graph_id);
                continue;
            }

            non_sharable_in.push(graph_id);
            resolvable_in.push(graph_id);
            interface_object_field_in.push(graph_id);
        }

        if !non_sharable_in.is_empty() && resolvable_in.len() > 1 {
            let is_non_sharable_in_all = resolvable_in.iter().all(|graph_id| non_sharable_in.contains(graph_id));

            let resolved_from = and_list(
                &cx.graph_ids_to_names(resolvable_in.iter().copied())?,
                false,
                Some("\""),
            );

            let defined_in = if is_non_sharable_in_all {
                "all of them".to_string()
            } else {
                format!(
                    "subgraph{} {}",
                    if non_sharable_in.len() > 1 { "s" } else { "" },
                    and_list(&cx.graph_ids_to_names(non_sharable_in.iter().copied())?, true, Some("\""))
                )
            };

            let message = format!(
                "Non-shareable field \"{}.{}\" is resolved from multiple subgraphs: it is resolved from subgraphs {resolved_from} and defined as non-shareable in {defined_in}",
                interface_type_state.name, field_state.name
            );

            cx.report_error(GraphQLError::with_code(message, "INVALID_FIELD_SHARING"));
        }

        Ok(())
    }
}
