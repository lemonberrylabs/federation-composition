//! Port of `src/supergraph/composition/object-type.ts`.

use graphql_js::{Definition, Selection, SelectionSet};
use crate::utils::collections::{IndexMap, IndexSet};

use super::ast::{
    create_object_type_node, CostAst, FieldArgumentAst, FieldAst, JoinAst, JoinFieldAst, JoinFieldContextArgumentAst,
    JoinImplementsAst, JoinTypeAst, ObjectTypeAst,
};
use super::common::{convert_to_const, get_or_insert_with, ComposeHelpers, Graph, Key, MapByGraph, TypeBuilder};
use super::interface_type::{
    graph_federation_version, list_size_ast, merge_list_size, truthy, InterfaceTypeFieldState, InterfaceTypeState,
};
use crate::specifications::federation::FederationVersion;
use crate::subgraph::helpers::parse_fields;
use crate::subgraph::state::{
    Ast, ArgumentKind, Deprecated, Description, ExtensionType, FromContext, ListSize, ObjectType,
};
use crate::supergraph::state::{SubgraphStates, SupergraphState};
use crate::utils::auth::merge_scope_policies;
use crate::utils::graphql::extract_named_type_name;
use crate::utils::helpers::{ensure_value, math_max};
use crate::utils::js::{JsError, JsResult};

/// `RequiresUsageStateInGraph`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequiresUsageStateInGraph {
    pub is_actively_required: bool,
    pub is_only_overridden_required: bool,
}

/// `RequiresFieldUsageIndex`: field coordinate to graph id to usage.
pub type RequiresFieldUsageIndex = IndexMap<String, IndexMap<String, RequiresUsageStateInGraph>>;

/// `isFederationExtension(meta, version)`.
pub fn is_federation_extension(meta: &ObjectTypeStateInGraph, version: FederationVersion) -> bool {
    let has_extends_directive = meta.extension_type == Some(ExtensionType::Extends);

    if meta.extension {
        if version == FederationVersion::V1_0 && !has_extends_directive {
            return false;
        }

        if has_extends_directive {
            return true;
        }

        if meta.has_definition {
            return false;
        }

        return true;
    }

    false
}

/// `objectTypeBuilder()`. Holds the closure's `requiresUsageIndex`, built lazily from the
/// whole supergraph state on the first `composeSupergraphNode` call.
/// NOTE(port): the index is built when the first object type is composed, after
/// `SupergraphStateBuilder::build` has transformed (`createFieldsTransformer`) only the object
/// types visited so far; keep that interleaving.
#[derive(Clone, Debug, Default)]
pub struct ObjectTypeBuilder {
    requires_usage_index: Option<RequiresFieldUsageIndex>,
}

/// `objectTypeBuilder()`.
pub fn object_type_builder() -> ObjectTypeBuilder {
    ObjectTypeBuilder::default()
}

impl ObjectTypeBuilder {
    /// `getRequiresUsageIndexLazy(supergraphState, graphNameToId)`.
    /// NOTE(port): if `buildRequiresUsageIndex` throws, the index stays unset (as in the TS,
    /// where the assignment never happens).
    fn get_requires_usage_index_lazy(&mut self, helpers: ComposeHelpers<'_>) -> JsResult<&RequiresFieldUsageIndex> {
        if self.requires_usage_index.is_none() {
            let built = build_requires_usage_index(helpers)?;
            self.requires_usage_index = Some(built);
        }

        Ok(self.requires_usage_index.get_or_insert_with(IndexMap::default))
    }

    /// `composeSupergraphState(objectType, _graphs, { supergraphState })`: merges auth
    /// directives from interface objects. Only `supergraphState.interfaceTypes` is read, so
    /// that is what it receives. Throws `Interface "X" not found in Supergraph state`.
    pub fn compose_supergraph_state(
        &mut self,
        object_type: &mut ObjectTypeState,
        _graphs: &SubgraphStates,
        interface_types: &IndexMap<String, InterfaceTypeState>,
    ) -> JsResult<()> {
        for interface_name in &object_type.interfaces {
            let Some(interface_state) = interface_types.get(interface_name) else {
                return Err(interface_not_found(interface_name));
            };

            for (interface_field_name, interface_field) in &interface_state.fields {
                if !interface_state.has_interface_object {
                    continue;
                }

                // Merge auth directives from interface object field with the object field
                // Check REF_AUTH_INTERFACE_OBJECT_SHAREABLE in tests
                if let Some(field_state) = object_type.fields.get_mut(interface_field_name) {
                    field_state.scopes = merge_scope_policies(&field_state.scopes, &interface_field.scopes);
                    field_state.policies = merge_scope_policies(&field_state.policies, &interface_field.policies);
                    if interface_field.authenticated {
                        field_state.authenticated = true;
                    }
                }
            }
        }

        Ok(())
    }
}

fn interface_not_found(interface_name: &str) -> JsError {
    JsError::error(format!("Interface \"{interface_name}\" not found in Supergraph state"))
}

/// `lastIndexOf("!")`: `-1` when missing. Type strings are ASCII, so byte indexes are
/// UTF-16 indexes.
fn last_index_of_bang(value: &str) -> i64 {
    value.rfind('!').map_or(-1, |i| i as i64)
}

impl TypeBuilder<ObjectType, ObjectTypeState> for ObjectTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, ObjectTypeState>,
        type_name: &str,
        ty: &ObjectType,
    ) {
        let object_type_state = get_or_create_object_type(state, type_name);

        for tag in &ty.tags {
            object_type_state.tags.insert(tag.clone());
        }
        for context_name in &ty.contexts {
            object_type_state.contexts.insert(format!("{}__{}", graph.name, context_name));
        }

        if ty.inaccessible {
            object_type_state.inaccessible = true;
        }

        if ty.authenticated {
            object_type_state.authenticated = true;
        }

        // NOTE(port): `if (type.policies)` / `if (type.scopes)` test arrays: always truthy.
        object_type_state.policies = merge_scope_policies(&object_type_state.policies, &ty.policies);

        object_type_state.scopes = merge_scope_policies(&object_type_state.scopes, &ty.scopes);

        if let Some(cost) = ty.cost {
            object_type_state.cost = Some(math_max(cost, object_type_state.cost));
        }

        let is_definition = ty.is_definition
            && (if graph.version == FederationVersion::V1_0 {
                ty.extension_type != Some(ExtensionType::Extends)
            } else {
                true
            });

        if ty.description.is_some() && object_type_state.description.is_none() {
            object_type_state.description = ty.description.clone();
        }

        if is_definition {
            object_type_state.has_definition = true;
        }

        for directive in &ty.ast.directives {
            object_type_state.ast.directives.push(directive.clone());
        }

        for interface_name in &ty.interfaces {
            object_type_state.interfaces.insert(interface_name.clone());
        }

        if !ty.keys.is_empty() {
            object_type_state.is_entity = true;
        }

        // NOTE(port): the TS stores the subgraph's `keys`/`interfaces` by reference; subgraph
        // states are read-only by now, so copies are equivalent.
        object_type_state.by_graph.insert(
            graph.id.clone(),
            ObjectTypeStateInGraph {
                has_definition: is_definition,
                extension: ty.extension,
                extension_type: ty.extension_type,
                external: ty.external,
                keys: ty.keys.clone(),
                inaccessible: ty.inaccessible,
                shareable: ty.shareable,
                interfaces: ty.interfaces.clone(),
                version: graph.version,
                authenticated: ty.authenticated,
                policies: ty.policies.clone(),
                scopes: ty.scopes.clone(),
            },
        );
        // NOTE(port): the TS keeps `typeInGraph` and calls `isFederationExtension(typeInGraph,
        // graph.version)` per external field; that pure result does not change during the loop,
        // so it is computed once here (the field state borrow excludes `byGraph`).
        let type_in_graph_is_federation_extension = is_federation_extension(
            object_type_state.by_graph.get(&graph.id).expect("typeInGraph was just set"),
            graph.version,
        );

        // PERF: a new type state gets room for the fields up front, instead of growing (and
        // moving the field states) as they are added. Only the capacity changes.
        if object_type_state.fields.is_empty() {
            object_type_state.fields.reserve(ty.fields.len());
        }

        for field in ty.fields.values() {
            let field_state = get_or_create_field(object_type_state, &field.name, &field.r#type);

            for tag in &field.tags {
                field_state.tags.insert(tag.clone());
            }

            let used_as_key = ty.fields_used_as_keys.contains(&field.name);

            if used_as_key {
                field_state.used_as_key = true;
            }

            // It's the first time we visited a non-external field, we should force the type on that field to match the local type
            let is_external = if graph.version == FederationVersion::V1_0 {
                field.external && type_in_graph_is_federation_extension
            } else {
                field.external
            };
            let should_force_type =
                // If it's not an external field and it's first time we visited a non-external field,
                // we should force the type but only if it's not used as a key
                // If it's used as a key, some other logic applies. Federation v2 is such a mess...
                !used_as_key && !is_external && !field_state.internal_seen_non_external;
            let should_change_type = should_force_type
                // If it's an external field, let's ignore it
                || (!is_external
                    // if the field type is nullable and
                    // the existing type is non-null
                    // Existing -> Incoming -> Result
                    // [A!]!    -> [A!]     -> [A!]
                    // [A!]!    -> [A]      -> [A]
                    // [A!]     -> [A!]!    -> [A!]
                    // [A!]     -> [A]      -> [A]
                    // [A]!     -> [A!]     -> [A!]
                    // Least nullable wins
                    && last_index_of_bang(&field_state.r#type) > last_index_of_bang(&field.r#type));

            if should_change_type {
                // Replace the non-null type with a nullable type
                field_state.r#type = field.r#type.clone();
            }

            if field.is_leaf {
                field_state.is_leaf = true;
            }

            if !field_state.internal_seen_non_external && !is_external {
                field_state.internal_seen_non_external = true;
            }

            if field.inaccessible {
                field_state.inaccessible = true;
            }

            if field.authenticated {
                field_state.authenticated = true;
            }

            // NOTE(port): `if (field.policies)` / `if (field.scopes)` test arrays: always truthy.
            field_state.policies = merge_scope_policies(&field_state.policies, &field.policies);

            field_state.scopes = merge_scope_policies(&field_state.scopes, &field.scopes);

            if let Some(cost) = field.cost {
                field_state.cost = Some(math_max(cost, field_state.cost));
            }

            if let Some(list_size) = &field.list_size {
                field_state.list_size = Some(merge_list_size(field_state.list_size.as_ref(), list_size));
            }

            // first wins BUT a graph overriding a field of an entity type (that provided the description) is an exception (it's applied at the supergraph level REF_1)
            if field.description.is_some() && field_state.description.is_none() {
                field_state.description = field.description.clone();
            }

            if truthy(&field.r#override) {
                field_state.r#override = field.r#override.clone();
            }

            if truthy(&field.override_label) {
                field_state.override_label = field.override_label.clone();
            }

            // First deprecation wins
            if field.deprecated.is_some() && field_state.deprecated.is_none() {
                field_state.deprecated = field.deprecated.clone();
            }

            for directive in &field.ast.directives {
                field_state.ast.directives.push(directive.clone());
            }

            field_state.by_graph.insert(
                graph.id.clone(),
                FieldStateInGraph {
                    r#type: field.r#type.clone(),
                    external: field.external,
                    inaccessible: field.inaccessible,
                    description: field.description.clone(),
                    r#override: field.r#override.clone(),
                    override_label: field.override_label.clone(),
                    provides: field.provides.clone(),
                    requires: field.requires.clone(),
                    provided: field.provided,
                    required: field.required,
                    shareable: field.shareable,
                    extension: field.extension,
                    authenticated: field.authenticated,
                    policies: field.policies.clone(),
                    scopes: field.scopes.clone(),
                    used: field.used,
                    used_as_key,
                    version: graph.version,
                },
            );

            for arg in field.args.values() {
                let arg_state = get_or_create_arg(field_state, &arg.name, &arg.r#type, arg.kind);

                for tag in &arg.tags {
                    arg_state.tags.insert(tag.clone());
                }

                if arg.r#type.ends_with('!') {
                    arg_state.r#type = arg.r#type.clone();
                }

                if arg.inaccessible {
                    arg_state.inaccessible = true;
                }

                // First description wins
                if arg.description.is_some() && arg_state.description.is_none() {
                    arg_state.description = arg.description.clone();
                }

                if arg.deprecated.is_some() && arg_state.deprecated.is_none() {
                    arg_state.deprecated = arg.deprecated.clone();
                }

                for directive in &arg.ast.directives {
                    arg_state.ast.directives.push(directive.clone());
                }

                // `typeof arg.defaultValue !== "undefined"`
                if arg.default_value.is_some() {
                    arg_state.default_value = arg.default_value.clone();
                }

                if arg.from_context.is_some() {
                    arg_state.from_context = arg.from_context.clone();
                }

                if let Some(cost) = arg.cost {
                    arg_state.cost = Some(math_max(cost, arg_state.cost));
                }

                arg_state.kind = arg.kind;

                arg_state.by_graph.insert(
                    graph.id.clone(),
                    ArgStateInGraph {
                        r#type: arg.r#type.clone(),
                        kind: arg.kind,
                        inaccessible: arg.inaccessible,
                        default_value: arg.default_value.clone(),
                        from_context: arg.from_context.clone(),
                        version: graph.version,
                    },
                );
            }
        }
    }

    fn compose_supergraph_node(
        &mut self,
        object_type: &ObjectTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        let supergraph_state = helpers.supergraph_state;
        let is_query = object_type.name == "Query";
        let requires_usage_index = self.get_requires_usage_index_lazy(helpers)?;

        let mut join_types: Vec<JoinTypeAst> = if is_query {
            // if it's a Query, we need to annotate the object type with `@join__type` pointing to all subgraphs
            graphs
                .values()
                .map(|graph| JoinTypeAst {
                    graph: graph.graph.id.clone(),
                    ..Default::default()
                })
                .collect()
        } else {
            // If it's not a Query, we follow the regular logic
            let mut join_types = Vec::new();
            for (graph_id, meta) in &object_type.by_graph {
                if !meta.keys.is_empty() {
                    for key in &meta.keys {
                        join_types.push(JoinTypeAst {
                            graph: graph_id.clone(),
                            key: Some(key.fields.clone()),
                            // To support Fed v1, we need to only apply `extension: true` when it's a type annotated with @extends (not by using `extend type` syntax, this needs to be ignored)
                            extension: Some(is_federation_extension(meta, graph_federation_version(graphs, graph_id))),
                            resolvable: Some(key.resolvable),
                            is_interface_object: None,
                        });
                    }
                    continue;
                }

                join_types.push(JoinTypeAst {
                    graph: graph_id.clone(),
                    ..Default::default()
                });
            }
            join_types
        };

        // a list of fields defined by interfaces (that type implements)
        let mut interface_field_graphs_by_field_name: IndexMap<String, IndexSet<String>> = IndexMap::default();
        let mut resolvable_fields_from_interface_objects: Vec<&InterfaceTypeFieldState> = Vec::new();

        for interface_name in &object_type.interfaces {
            let Some(interface_state) = supergraph_state.interface_types.get(interface_name) else {
                return Err(interface_not_found(interface_name));
            };

            for (interface_field_name, interface_field) in &interface_state.fields {
                if let Some(found) = interface_field_graphs_by_field_name.get_mut(interface_field_name) {
                    for graph_id in interface_field.by_graph.keys() {
                        found.insert(graph_id.clone());
                    }
                } else {
                    interface_field_graphs_by_field_name.insert(
                        interface_field_name.clone(),
                        interface_field.by_graph.keys().cloned().collect(),
                    );
                }

                if !interface_state.has_interface_object {
                    continue;
                }

                if !resolvable_fields_from_interface_objects
                    .iter()
                    .any(|f| f.name == *interface_field_name)
                {
                    resolvable_fields_from_interface_objects.push(interface_field);
                }
            }
        }

        if !is_query {
            join_types.retain(|join_type| {
                should_keep_object_type_join_type_in_graph(ShouldKeepObjectTypeJoinTypeInGraphInput {
                    graph_id: &join_type.graph,
                    object_type,
                    requires_usage_index,
                    interface_field_graphs_by_field_name: &interface_field_graphs_by_field_name,
                    helpers,
                })
            });
        }

        // NOTE(port): the TS assigns `field.description` here, mutating the supergraph state.
        // Nothing reads object field descriptions after this call (it is made once per type,
        // from `build()`, after validation), so the corrected descriptions are kept locally.
        let mut corrected_descriptions: IndexMap<&str, Option<Description>> = IndexMap::default();
        if object_type.is_entity {
            for field in object_type.fields.values() {
                // Correct description if needed (REF_1)
                if field.description.is_none() {
                    continue;
                }

                // check if a field was overridden
                if !truthy(&field.r#override) {
                    continue;
                }

                for field_in_graph in field.by_graph.values() {
                    // if a field is shareable, ignore the description (I don't know why...don't ask me)
                    if truthy(&field_in_graph.r#override) && !field_in_graph.shareable {
                        // use description from that graph
                        corrected_descriptions.insert(field.name.as_str(), field_in_graph.description.clone());
                    }
                }
            }
        }

        let cx = ObjectComposeCx {
            object_type,
            graphs,
            helpers,
            interface_field_graphs_by_field_name: &interface_field_graphs_by_field_name,
        };

        // Properties of the `createObjectTypeNode` argument are evaluated in order: `cost`
        // (whose `ensureValue` can throw) comes before `fields`.
        let cost = match object_type.cost {
            Some(cost) => Some(CostAst {
                cost,
                directive_name: ensure_value(
                    supergraph_state.specs.cost.names.cost.clone(),
                    "Directive name of @cost is not defined",
                )?,
            }),
            None => None,
        };

        let mut fields: Vec<FieldAst> = Vec::with_capacity(object_type.fields.len());
        let mut field_coordinate_buffer = String::new();

        for field in object_type.fields.values() {
            let field_in_graphs: Vec<(&String, &FieldStateInGraph)> = field.by_graph.iter().collect();
            let field_coordinate = write_field_coordinate(&mut field_coordinate_buffer, &object_type.name, &field.name);
            let is_effectively_required = |graph_id: &str, meta: &FieldStateInGraph| {
                is_field_coordinate_effectively_required(requires_usage_index, field_coordinate, graph_id, meta.required)
            };
            let has_only_overridden_requires_usage = |graph_id: &str| {
                has_only_overridden_requires_dependency(requires_usage_index, field_coordinate, graph_id)
            };

            let has_different_output_type = field_in_graphs.iter().any(|(_, meta)| meta.r#type != field.r#type);
            let is_defined_everywhere = field.by_graph.len()
                == (if is_query {
                    graphs.len()
                } else {
                    object_type.by_graph.len()
                });
            let mut join_fields: Vec<JoinFieldAst> = Vec::new();

            let mut differences_between_graphs = DifferencesBetweenGraphs::default();

            for (graph_id, meta) in &field_in_graphs {
                if meta.external {
                    differences_between_graphs.external = if field.used_as_key {
                        !cx.object_type_in_graph(graph_id).extension
                    } else {
                        true
                    };
                }
                if meta.r#override.is_some() {
                    differences_between_graphs.r#override = true;
                }
                if meta.override_label.is_some() {
                    differences_between_graphs.override_label = true;
                }
                if meta.provides.is_some() {
                    differences_between_graphs.provides = true;
                }
                if meta.requires.is_some() {
                    differences_between_graphs.requires = true;
                }
                if meta.r#type != field.r#type {
                    differences_between_graphs.r#type = true;
                }
                if field
                    .args
                    .values()
                    .any(|arg| arg.by_graph.get(*graph_id).is_some_and(|a| a.from_context.is_some()))
                {
                    differences_between_graphs.context_arguments = true;
                }
            }

            let override_join_field_info = if differences_between_graphs.r#override {
                Some(cx.collect_override_join_field_info(&field_in_graphs))
            } else {
                None
            };

            let mut graph_join_field_state: IndexMap<&str, GraphJoinFieldState> = IndexMap::default();

            // Precompute per-graph values once
            // PERF: only the `@override` branches below read `graphJoinFieldState`, so it is
            // only built for fields with an override. The values are pure (the TS could only
            // throw here for a field graph missing from the type's graphs, which cannot happen).
            for (graph_id, meta) in field_in_graphs.iter().filter(|_| differences_between_graphs.r#override) {
                let has_only_requires_usage = has_only_overridden_requires_usage(graph_id);
                graph_join_field_state.insert(
                    graph_id.as_str(),
                    GraphJoinFieldState {
                        is_required: is_effectively_required(graph_id, meta),
                        external_on_join_field: cx.should_set_external_on_join_field(
                            meta,
                            graph_id,
                            field,
                            has_only_requires_usage,
                        ),
                        used_overridden: is_used_overridden_in_graph(
                            field,
                            meta,
                            &interface_field_graphs_by_field_name,
                            graph_id,
                            helpers,
                        ),
                    },
                );
            }

            let empty_override_labels: IndexMap<String, String> = IndexMap::default();
            let override_labels = override_join_field_info
                .as_ref()
                .map_or(&empty_override_labels, |info| &info.override_labels);
            let overridden_graphs = override_join_field_info.as_ref().map(|info| &info.overridden_graphs);

            let should_emit_override_join_field =
                |graph_id: &str, meta: &FieldStateInGraph, mode: OverrideJoinFieldMode| -> bool {
                    let override_label = override_labels.get(graph_id);

                    if mode == OverrideJoinFieldMode::Query {
                        let has_explicit_override = meta.r#override.is_some();
                        let has_override_label = override_label.is_some_and(|l| !l.is_empty());
                        let is_shareable_source_graph =
                            meta.shareable && overridden_graphs.is_some_and(|g| !g.contains(graph_id));

                        return has_explicit_override || has_override_label || is_shareable_source_graph;
                    }

                    let graph_state = graph_join_field_state
                        .get(graph_id)
                        .expect("graphJoinFieldState has every graph of the field");
                    let is_overridden_source_graph = overridden_graphs.is_some_and(|g| g.contains(graph_id));
                    let needs_override_label =
                        meta.override_label.is_some() || override_label.is_some_and(|l| !l.is_empty());
                    let needs_external_for_required = graph_state.external_on_join_field && graph_state.is_required;
                    let has_used_overridden = graph_state.used_overridden
                        || (is_overridden_source_graph && graph_state.is_required && !graph_state.external_on_join_field);

                    needs_external_for_required
                        || needs_override_label
                        || has_used_overridden
                        || !is_overridden_source_graph
                };

            let build_override_join_fields =
                |mode: OverrideJoinFieldMode, graphs_to_emit: &[(&String, &FieldStateInGraph)]| -> Vec<JoinFieldAst> {
                    graphs_to_emit
                        .iter()
                        .map(|(graph_id, meta)| {
                            let graph_state = graph_join_field_state
                                .get(graph_id.as_str())
                                .expect("graphJoinFieldState has every graph of the field");
                            let is_overridden_source_graph = overridden_graphs.is_some_and(|g| g.contains(*graph_id));
                            let used_overridden = if mode == OverrideJoinFieldMode::Object {
                                Some(
                                    graph_state.used_overridden
                                        || (is_overridden_source_graph
                                            && graph_state.is_required
                                            && !graph_state.external_on_join_field),
                                )
                            } else {
                                None
                            };

                            cx.build_join_field_ast(
                                graph_id,
                                meta,
                                field,
                                differences_between_graphs.r#type,
                                if mode == OverrideJoinFieldMode::Query {
                                    meta.override_label.clone()
                                } else {
                                    override_labels.get(graph_id.as_str()).cloned()
                                },
                                if mode == OverrideJoinFieldMode::Query {
                                    Some(meta.external)
                                } else if graph_state.external_on_join_field {
                                    Some(true)
                                } else {
                                    None
                                },
                                used_overridden,
                            )
                        })
                        .collect()
                };

            if !is_query && field.by_graph.len() == 1 {
                let (graph_id, field_in_graph) = field.by_graph.first().expect("byGraph has one entry");

                if
                // a field is external
                field_in_graph.external
                    // it's not used as a key
                    && !field_in_graph.used_as_key
                    // it's not part of any @requires(fields:)
                    && !is_effectively_required(graph_id, field_in_graph)
                    // it's not part of any @provides(fields:)
                    && !field_in_graph.provided
                    // it's not part of any @override(from:) and it's not used by any interface
                    && !is_used_overridden_in_graph(
                        field,
                        field_in_graph,
                        &interface_field_graphs_by_field_name,
                        graph_id,
                        helpers,
                    )
                    // and it's Federation v1
                    && graph_federation_version(graphs, graph_id) == FederationVersion::V1_0
                {
                    // drop the field
                    continue;
                }
            }

            if is_query {
                // If it's a Query type, we don't need to emit `@join__field` directives when there's only one graph
                // We do not have to emit `@join__field` if the field is shareable in every graph as well.

                if differences_between_graphs.r#override && graphs.len() > 1 {
                    let graphs_to_emit: Vec<(&String, &FieldStateInGraph)> = field_in_graphs
                        .iter()
                        .filter(|(graph_id, meta)| {
                            should_emit_override_join_field(graph_id, meta, OverrideJoinFieldMode::Query)
                        })
                        .copied()
                        .collect();

                    join_fields = build_override_join_fields(OverrideJoinFieldMode::Query, &graphs_to_emit);
                } else {
                    join_fields = if graphs.len() > 1 && !is_defined_everywhere {
                        field_in_graphs
                            .iter()
                            .map(|(graph_id, meta)| JoinFieldAst {
                                graph: Some((*graph_id).clone()),
                                provides: if differences_between_graphs.provides {
                                    meta.provides.clone()
                                } else {
                                    None
                                },
                                ..Default::default()
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                }
            } else if is_defined_everywhere {
                let has_differences_between_graphs = differences_between_graphs.some();

                // We probably need to emit `@join__field` for every graph, except the one where the override was applied
                if differences_between_graphs.r#override {
                    let graphs_to_emit: Vec<(&String, &FieldStateInGraph)> = field_in_graphs
                        .iter()
                        .filter(|(graph_id, f)| should_emit_override_join_field(graph_id, f, OverrideJoinFieldMode::Object))
                        .copied()
                        .collect();

                    // Do not emit `@join__field` if there's only one graph left
                    // and the type has a single `@join__type` matching the graph.
                    // In that case field-level join info is redundant.
                    if !(graphs_to_emit.len() == 1
                        && join_types.len() == 1
                        && join_types[0].graph == *graphs_to_emit[0].0)
                    {
                        join_fields = build_override_join_fields(OverrideJoinFieldMode::Object, &graphs_to_emit);
                    }
                } else if has_differences_between_graphs {
                    join_fields = cx.create_join_fields(
                        &field_in_graphs,
                        field,
                        has_different_output_type,
                        &is_effectively_required,
                        &has_only_overridden_requires_usage,
                    );
                }
            } else {
                // An override is a special case, we need to emit `@join__field` only for graphs where @override was applied
                if differences_between_graphs.r#override {
                    let graphs_to_emit: Vec<(&String, &FieldStateInGraph)> = field_in_graphs
                        .iter()
                        .filter(|(graph_id, meta)| {
                            should_emit_override_join_field(graph_id, meta, OverrideJoinFieldMode::Object)
                        })
                        .copied()
                        .collect();

                    join_fields = build_override_join_fields(OverrideJoinFieldMode::Object, &graphs_to_emit);
                } else {
                    join_fields = cx.create_join_fields(
                        &field_in_graphs,
                        field,
                        has_different_output_type,
                        &is_effectively_required,
                        &has_only_overridden_requires_usage,
                    );
                }
            }

            let field_cost = match field.cost {
                Some(cost) => Some(CostAst {
                    cost,
                    directive_name: ensure_value(
                        supergraph_state.specs.cost.names.cost.clone(),
                        "Directive name of @cost is not defined",
                    )?,
                }),
                None => None,
            };
            let field_list_size = match &field.list_size {
                Some(list_size) => Some(list_size_ast(
                    list_size,
                    ensure_value(
                        supergraph_state.specs.cost.names.list_size.clone(),
                        "Directive name of @listSize is not defined",
                    )?,
                )),
                None => None,
            };

            // If there's only one graph on both field and type
            // and it has no properties, we don't need to emit `@join__field`
            let join_field = if join_fields.len() == 1
                && join_types.len() == 1
                && is_empty_join_field(&join_fields[0])
            {
                Vec::new()
            } else {
                join_fields
            };

            let mut arguments: Vec<FieldArgumentAst> = Vec::new();
            for arg in field.args.values() {
                if arg.from_context.is_some() {
                    continue;
                }

                // ignore the argument if it's not available in all subgraphs implementing the field
                if arg.by_graph.len() != field.by_graph.len() {
                    continue;
                }

                arguments.push(FieldArgumentAst {
                    name: arg.name.clone(),
                    r#type: arg.r#type.clone(),
                    kind: arg.kind,
                    inaccessible: arg.inaccessible,
                    cost: match arg.cost {
                        Some(cost) => Some(CostAst {
                            cost,
                            directive_name: ensure_value(
                                supergraph_state.specs.cost.names.cost.clone(),
                                "Directive name of @cost is not defined",
                            )?,
                        }),
                        None => None,
                    },
                    tags: arg.tags.iter().cloned().collect(),
                    default_value: arg.default_value.clone(),
                    description: arg.description.clone(),
                    deprecated: arg.deprecated.clone(),
                    ast: convert_to_const(&arg.ast.directives),
                });
            }

            let description = match corrected_descriptions.get(field.name.as_str()) {
                Some(corrected) => corrected.clone(),
                None => field.description.clone(),
            };

            fields.push(FieldAst {
                name: field.name.clone(),
                r#type: field.r#type.clone(),
                inaccessible: field.inaccessible,
                authenticated: field.authenticated,
                policies: field.policies.clone(),
                scopes: field.scopes.clone(),
                tags: field.tags.iter().cloned().collect(),
                description,
                deprecated: field.deprecated.clone(),
                cost: field_cost,
                list_size: field_list_size,
                ast: convert_to_const(&field.ast.directives),
                join: JoinAst {
                    field: join_field,
                    ..Default::default()
                },
                arguments,
            });
        }

        for field in resolvable_fields_from_interface_objects
            .iter()
            .filter(|f| !object_type.fields.contains_key(&f.name))
        {
            let mut scopes: Vec<Vec<String>> = Vec::new();
            let mut policies: Vec<Vec<String>> = Vec::new();
            let mut authenticated = false;

            for field_in_graph in field.by_graph.values() {
                scopes = merge_scope_policies(&scopes, &field_in_graph.scopes);
                policies = merge_scope_policies(&policies, &field_in_graph.policies);
                if field_in_graph.authenticated {
                    authenticated = true;
                }
            }

            let field_cost = match field.cost {
                Some(cost) => Some(CostAst {
                    cost,
                    directive_name: ensure_value(
                        supergraph_state.specs.cost.names.cost.clone(),
                        "Directive name of @cost is not defined",
                    )?,
                }),
                None => None,
            };
            let field_list_size = match &field.list_size {
                Some(list_size) => Some(list_size_ast(
                    list_size,
                    ensure_value(
                        supergraph_state.specs.cost.names.list_size.clone(),
                        "Directive name of @listSize is not defined",
                    )?,
                )),
                None => None,
            };

            let mut arguments: Vec<FieldArgumentAst> = Vec::new();
            for arg in field.args.values() {
                if arg.from_context.is_some() {
                    continue;
                }

                // ignore the argument if it's not available in all subgraphs implementing the field
                if arg.by_graph.len() != field.by_graph.len() {
                    continue;
                }

                arguments.push(FieldArgumentAst {
                    name: arg.name.clone(),
                    r#type: arg.r#type.clone(),
                    kind: arg.kind,
                    inaccessible: false,
                    cost: match arg.cost {
                        Some(cost) => Some(CostAst {
                            cost,
                            directive_name: ensure_value(
                                supergraph_state.specs.cost.names.cost.clone(),
                                "Directive name of @cost is not defined",
                            )?,
                        }),
                        None => None,
                    },
                    tags: arg.tags.iter().cloned().collect(),
                    default_value: arg.default_value.clone(),
                    description: arg.description.clone(),
                    deprecated: arg.deprecated.clone(),
                    ast: convert_to_const(&arg.ast.directives),
                });
            }

            fields.push(FieldAst {
                name: field.name.clone(),
                r#type: field.r#type.clone(),
                inaccessible: field.inaccessible,
                authenticated,
                policies,
                scopes,
                cost: field_cost,
                list_size: field_list_size,
                tags: field.tags.iter().cloned().collect(),
                description: field.description.clone(),
                deprecated: field.deprecated.clone(),
                ast: convert_to_const(&field.ast.directives),
                join: JoinAst {
                    // `[{}]`: a bare `@join__field`
                    field: vec![JoinFieldAst::default()],
                    ..Default::default()
                },
                arguments,
            });
        }

        let mut implements: Vec<JoinImplementsAst> = Vec::new();
        if !object_type.interfaces.is_empty() {
            for (graph, meta) in &object_type.by_graph {
                for interface_name in &meta.interfaces {
                    implements.push(JoinImplementsAst {
                        // NOTE(port): JS `toUpperCase`; graph ids are ASCII.
                        graph: graph.to_uppercase(),
                        interface: interface_name.clone(),
                    });
                }
            }
        }

        Ok(Definition::ObjectTypeDefinition(create_object_type_node(ObjectTypeAst {
            name: object_type.name.clone(),
            // DirectiveNode and ConstDirectiveNode are identical (except the readonly shit...)
            ast: convert_to_const(&object_type.ast.directives),
            cost,
            description: object_type.description.clone(),
            fields,
            interfaces: object_type.interfaces.iter().cloned().collect(),
            tags: object_type.tags.iter().cloned().collect(),
            inaccessible: object_type.inaccessible,
            authenticated: object_type.authenticated,
            policies: object_type.policies.clone(),
            scopes: object_type.scopes.clone(),
            contexts: object_type.contexts.iter().cloned().collect(),
            join: JoinAst {
                r#type: join_types,
                implements,
                ..Default::default()
            },
        })))
    }
}

/// The `joinFields[0]` test of `join.field`: `!external && !override && !overrideLabel &&
/// !provides && !requires && !contextArguments?.length && !usedOverridden && !type`.
fn is_empty_join_field(join_field: &JoinFieldAst) -> bool {
    join_field.external != Some(true)
        && !truthy(&join_field.r#override)
        && !truthy(&join_field.override_label)
        && !truthy(&join_field.provides)
        && !truthy(&join_field.requires)
        && join_field.context_arguments.as_ref().is_none_or(|c| c.is_empty())
        && join_field.used_overridden != Some(true)
        && !truthy(&join_field.r#type)
}

/// `differencesBetweenGraphs` of `composeSupergraphNode`.
#[derive(Clone, Copy, Debug, Default)]
struct DifferencesBetweenGraphs {
    r#override: bool,
    override_label: bool,
    r#type: bool,
    external: bool,
    provides: bool,
    requires: bool,
    context_arguments: bool,
}

impl DifferencesBetweenGraphs {
    /// `Object.values(differencesBetweenGraphs).some((value) => value === true)`.
    fn some(&self) -> bool {
        self.r#override
            || self.override_label
            || self.r#type
            || self.external
            || self.provides
            || self.requires
            || self.context_arguments
    }
}

/// The values of `graphJoinFieldState`.
#[derive(Clone, Copy, Debug)]
struct GraphJoinFieldState {
    is_required: bool,
    external_on_join_field: bool,
    used_overridden: bool,
}

/// `mode: "query" | "object"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverrideJoinFieldMode {
    Query,
    Object,
}

/// The result of `collectOverrideJoinFieldInfo`.
/// NOTE(port): `overrideLabels` is a plain object keyed by graph id; it is only looked up.
struct OverrideJoinFieldInfo {
    override_labels: IndexMap<String, String>,
    overridden_graphs: IndexSet<String>,
}

/// The locals of `composeSupergraphNode` its inner functions (`buildContextArguments`,
/// `shouldSetExternalOnJoinField`, ...) close over.
struct ObjectComposeCx<'a> {
    object_type: &'a ObjectTypeState,
    graphs: &'a SubgraphStates,
    helpers: ComposeHelpers<'a>,
    interface_field_graphs_by_field_name: &'a IndexMap<String, IndexSet<String>>,
}

impl ObjectComposeCx<'_> {
    /// `objectType.byGraph.get(graphId)!`.
    /// NOTE(port): field graph ids are always graph ids of the type (both are set in the same
    /// `visitSubgraphState`); the TS would throw a `TypeError` otherwise.
    fn object_type_in_graph(&self, graph_id: &str) -> &ObjectTypeStateInGraph {
        self.object_type
            .by_graph
            .get(graph_id)
            .expect("Cannot read properties of undefined (reading 'extension')")
    }

    fn build_context_arguments(&self, graph_id: &str, field: &ObjectTypeFieldState) -> Vec<JoinFieldContextArgumentAst> {
        field
            .args
            .values()
            .filter_map(|arg| {
                let from_context = arg.by_graph.get(graph_id).and_then(|a| a.from_context.as_ref())?;

                Some(JoinFieldContextArgumentAst {
                    name: arg.name.clone(),
                    r#type: arg.r#type.clone(),
                    context: format!(
                        "{}__{}",
                        self.graphs
                            .get(graph_id)
                            .expect("Cannot read properties of undefined (reading 'graph')")
                            .graph
                            .name,
                        from_context.context
                    ),
                    selection: from_context.selection.clone(),
                })
            })
            .collect()
    }

    fn should_set_external_on_join_field(
        &self,
        field_state_in_graph: &FieldStateInGraph,
        graph_id: &str,
        _field_state: &ObjectTypeFieldState,
        has_only_overridden_requires_usage: bool,
    ) -> bool {
        // Decide whether `@join__field(..., external: true)` should be emitted
        if !field_state_in_graph.external {
            return false;
        }

        // If the only `@requires` usage for this subgraph comes from field variants
        // that were overridden away, keeping `external: true` would break Apollo's QP.
        if has_only_overridden_requires_usage {
            return false;
        }

        if field_state_in_graph.provided {
            return true;
        }

        // mark field as external if it's annotated with @external, but it's not used as a key on the extension type
        if field_state_in_graph.used_as_key && self.object_type_in_graph(graph_id).extension {
            return false;
        }

        true
    }

    fn should_drop_fed_v1_unused_external_join_field(
        &self,
        graph_id: &str,
        field_state_in_graph: &FieldStateInGraph,
        is_required_or_provided: bool,
        used_overridden: bool,
    ) -> bool {
        field_state_in_graph.external
            && !field_state_in_graph.used_as_key
            && !is_required_or_provided
            && !used_overridden
            && graph_federation_version(self.graphs, graph_id) == FederationVersion::V1_0
    }

    fn create_join_fields(
        &self,
        field_in_graphs: &[(&String, &FieldStateInGraph)],
        field: &ObjectTypeFieldState,
        has_different_output_type: bool,
        is_effectively_required: &dyn Fn(&str, &FieldStateInGraph) -> bool,
        has_only_overridden_requires_usage: &dyn Fn(&str) -> bool,
    ) -> Vec<JoinFieldAst> {
        field_in_graphs
            .iter()
            .filter_map(|(graph_id, meta)| {
                let graph_id = graph_id.as_str();
                let r#type = if has_different_output_type {
                    Some(meta.r#type.clone())
                } else {
                    None
                };
                let r#override = meta.r#override.clone();
                let override_label = meta.override_label.clone();
                let used_overridden = is_used_overridden_in_graph(
                    field,
                    meta,
                    self.interface_field_graphs_by_field_name,
                    graph_id,
                    self.helpers,
                );
                let in_interface_graphs = self.interface_field_graphs_by_field_name.get(&field.name);
                let is_interface_field_in_graph = in_interface_graphs.is_some_and(|g| g.contains(graph_id));
                let external = self.should_set_external_on_join_field(
                    meta,
                    graph_id,
                    field,
                    has_only_overridden_requires_usage(graph_id),
                );
                let provides = meta.provides.clone();
                let requires = meta.requires.clone();

                let context_arguments = self.build_context_arguments(graph_id, field);

                let has_any_join_field_metadata = truthy(&r#type)
                    || truthy(&r#override)
                    || truthy(&provides)
                    || truthy(&requires)
                    || !context_arguments.is_empty()
                    || used_overridden;
                let is_required_or_provided = meta.provided || is_effectively_required(graph_id, meta);

                if self.should_drop_fed_v1_unused_external_join_field(
                    graph_id,
                    meta,
                    is_required_or_provided,
                    used_overridden,
                ) {
                    return None;
                }

                if external
                    && self.object_type_in_graph(graph_id).extension
                    && !has_any_join_field_metadata
                    && !is_required_or_provided
                    && !is_interface_field_in_graph
                {
                    return None;
                }

                if meta.external
                    && !external
                    && has_only_overridden_requires_usage(graph_id)
                    && !has_any_join_field_metadata
                    && !is_required_or_provided
                    && !is_interface_field_in_graph
                {
                    return None;
                }

                Some(JoinFieldAst {
                    graph: Some(graph_id.to_string()),
                    r#type,
                    r#override,
                    override_label,
                    used_overridden: Some(used_overridden),
                    external: Some(external),
                    provides,
                    requires,
                    context_arguments: if context_arguments.is_empty() {
                        None
                    } else {
                        Some(context_arguments)
                    },
                })
            })
            .collect()
    }

    fn collect_override_join_field_info(
        &self,
        field_in_graphs: &[(&String, &FieldStateInGraph)],
    ) -> OverrideJoinFieldInfo {
        let mut override_labels: IndexMap<String, String> = IndexMap::default();
        let mut overridden_graphs: IndexSet<String> = IndexSet::default();

        for (to_graph_id, meta) in field_in_graphs {
            let Some(override_name) = meta.r#override.as_deref().filter(|o| !o.is_empty()) else {
                continue;
            };

            let Some(from_graph_id) = self.helpers.graph_name_to_id(override_name).filter(|id| !id.is_empty()) else {
                continue;
            };

            overridden_graphs.insert(from_graph_id.clone());

            if let Some(override_label) = meta.override_label.as_deref().filter(|l| !l.is_empty()) {
                override_labels.insert(from_graph_id, override_label.to_string());
                override_labels.insert((*to_graph_id).clone(), override_label.to_string());
            }
        }

        OverrideJoinFieldInfo {
            override_labels,
            overridden_graphs,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_join_field_ast(
        &self,
        graph_id: &str,
        meta: &FieldStateInGraph,
        field: &ObjectTypeFieldState,
        has_different_output_type: bool,
        override_label: Option<String>,
        external: Option<bool>,
        used_overridden: Option<bool>,
    ) -> JoinFieldAst {
        let context_arguments = self.build_context_arguments(graph_id, field);

        JoinFieldAst {
            graph: Some(graph_id.to_string()),
            r#override: meta.r#override.clone(),
            override_label,
            used_overridden: Some(used_overridden.unwrap_or_else(|| {
                is_used_overridden_in_graph(
                    field,
                    meta,
                    self.interface_field_graphs_by_field_name,
                    graph_id,
                    self.helpers,
                )
            })),
            r#type: if has_different_output_type {
                Some(meta.r#type.clone())
            } else {
                None
            },
            external,
            provides: meta.provides.clone(),
            requires: meta.requires.clone(),
            context_arguments: if context_arguments.is_empty() {
                None
            } else {
                Some(context_arguments)
            },
        }
    }
}

/// `ObjectTypeState | InterfaceTypeState` (`markRequiresUsageFromSelectionSet`'s `currentType`).
#[derive(Clone, Copy, Debug)]
pub enum ObjectOrInterfaceTypeState<'s> {
    Object(&'s ObjectTypeState),
    Interface(&'s InterfaceTypeState),
}

impl<'s> ObjectOrInterfaceTypeState<'s> {
    fn name(&self) -> &'s str {
        match self {
            ObjectOrInterfaceTypeState::Object(t) => &t.name,
            ObjectOrInterfaceTypeState::Interface(t) => &t.name,
        }
    }

    /// `currentType.fields.get(fieldName)?.type`.
    fn field_type(&self, field_name: &str) -> Option<&'s str> {
        match self {
            ObjectOrInterfaceTypeState::Object(t) => t.fields.get(field_name).map(|f| f.r#type.as_str()),
            ObjectOrInterfaceTypeState::Interface(t) => t.fields.get(field_name).map(|f| f.r#type.as_str()),
        }
    }

    /// `supergraphState.objectTypes.get(name) ?? supergraphState.interfaceTypes.get(name)`.
    fn lookup(supergraph_state: &'s SupergraphState, name: &str) -> Option<Self> {
        supergraph_state
            .object_types
            .get(name)
            .map(ObjectOrInterfaceTypeState::Object)
            .or_else(|| {
                supergraph_state
                    .interface_types
                    .get(name)
                    .map(ObjectOrInterfaceTypeState::Interface)
            })
    }
}

fn is_used_overridden_in_graph(
    field: &ObjectTypeFieldState,
    field_state_in_graph: &FieldStateInGraph,
    interface_field_graphs_by_field_name: &IndexMap<String, IndexSet<String>>,
    graph_id: &str,
    helpers: ComposeHelpers<'_>,
) -> bool {
    let interface_graphs_for_field = interface_field_graphs_by_field_name.get(&field.name);
    let is_interface_field_in_graph = interface_graphs_for_field.is_some_and(|g| g.contains(graph_id));
    let is_used_as_non_external_key = field_state_in_graph.used_as_key && !field_state_in_graph.external;
    let is_overridden = field
        .r#override
        .as_deref()
        .filter(|o| !o.is_empty())
        .is_some_and(|o| helpers.graph_name_to_id(o).as_deref() == Some(graph_id));

    if is_overridden && (is_used_as_non_external_key || is_interface_field_in_graph) {
        return true;
    }

    false
}

fn to_field_coordinate(type_name: &str, field_name: &str) -> String {
    format!("{type_name}.{field_name}")
}

/// `toFieldCoordinate(typeName, fieldName)` written into a reused buffer.
/// PERF: the coordinate is only looked up in the requires usage index, once per field (and per
/// graph in `shouldKeepObjectTypeJoinTypeInGraph`), so it is not allocated each time.
fn write_field_coordinate<'b>(buffer: &'b mut String, type_name: &str, field_name: &str) -> &'b str {
    buffer.clear();
    buffer.push_str(type_name);
    buffer.push('.');
    buffer.push_str(field_name);
    buffer
}

fn get_requires_usage_state<'i>(
    requires_usage_index: &'i RequiresFieldUsageIndex,
    coordinate: &str,
    graph_id: &str,
) -> Option<&'i RequiresUsageStateInGraph> {
    requires_usage_index.get(coordinate).and_then(|m| m.get(graph_id))
}

fn is_field_coordinate_effectively_required(
    requires_usage_index: &RequiresFieldUsageIndex,
    coordinate: &str,
    graph_id: &str,
    fallback_required: bool,
) -> bool {
    let usage_in_graph = get_requires_usage_state(requires_usage_index, coordinate, graph_id);
    if usage_in_graph.is_some_and(|u| u.is_actively_required) {
        return true;
    }
    if usage_in_graph.is_some_and(|u| u.is_only_overridden_required) {
        return false;
    }
    fallback_required
}

fn has_only_overridden_requires_dependency(
    requires_usage_index: &RequiresFieldUsageIndex,
    coordinate: &str,
    graph_id: &str,
) -> bool {
    let usage_in_graph = get_requires_usage_state(requires_usage_index, coordinate, graph_id);
    usage_in_graph.is_some_and(|u| u.is_only_overridden_required && !u.is_actively_required)
}

/// The argument object of `shouldKeepObjectTypeJoinTypeInGraph`.
struct ShouldKeepObjectTypeJoinTypeInGraphInput<'i> {
    graph_id: &'i str,
    object_type: &'i ObjectTypeState,
    requires_usage_index: &'i RequiresFieldUsageIndex,
    interface_field_graphs_by_field_name: &'i IndexMap<String, IndexSet<String>>,
    helpers: ComposeHelpers<'i>,
}

fn should_keep_object_type_join_type_in_graph(input: ShouldKeepObjectTypeJoinTypeInGraphInput<'_>) -> bool {
    let ShouldKeepObjectTypeJoinTypeInGraphInput {
        graph_id,
        object_type,
        requires_usage_index,
        interface_field_graphs_by_field_name,
        helpers,
    } = input;

    let Some(object_type_in_graph) = object_type.by_graph.get(graph_id) else {
        return false;
    };

    if !object_type_in_graph.keys.is_empty() {
        return true;
    }

    if object_type_in_graph.has_definition {
        return true;
    }

    let mut field_coordinate_buffer = String::new();
    for field in object_type.fields.values() {
        let Some(field_in_graph) = field.by_graph.get(graph_id) else {
            continue;
        };

        let field_coordinate = write_field_coordinate(&mut field_coordinate_buffer, &object_type.name, &field.name);
        let is_effectively_required = is_field_coordinate_effectively_required(
            requires_usage_index,
            field_coordinate,
            graph_id,
            field_in_graph.required,
        );
        let has_only_overridden_requires =
            has_only_overridden_requires_dependency(requires_usage_index, field_coordinate, graph_id);

        if !field_in_graph.external
            || field_in_graph.provided
            || is_effectively_required
            || (object_type_in_graph.extension && !has_only_overridden_requires)
            || is_used_overridden_in_graph(
                field,
                field_in_graph,
                interface_field_graphs_by_field_name,
                graph_id,
                helpers,
            )
        {
            return true;
        }
    }

    false
}

fn collect_overridden_graphs_by_field(field: &ObjectTypeFieldState, helpers: ComposeHelpers<'_>) -> IndexSet<String> {
    // Collect source graphs that are overridden by this field via `@override`
    let mut overridden_graphs = IndexSet::default();

    for meta in field.by_graph.values() {
        let Some(override_name) = meta.r#override.as_deref().filter(|o| !o.is_empty()) else {
            continue;
        };

        if let Some(source_graph_id) = helpers.graph_name_to_id(override_name).filter(|id| !id.is_empty()) {
            overridden_graphs.insert(source_graph_id);
        }
    }

    overridden_graphs
}

/// `buildRequiresUsageIndex(supergraphState, graphNameToId)`. `parseFields` can throw.
fn build_requires_usage_index(helpers: ComposeHelpers<'_>) -> JsResult<RequiresFieldUsageIndex> {
    let supergraph_state = helpers.supergraph_state;
    let mut index: RequiresFieldUsageIndex = IndexMap::default();

    for object_type in supergraph_state.object_types.values() {
        for field in object_type.fields.values() {
            let overridden_graphs = collect_overridden_graphs_by_field(field, helpers);

            for (graph_id, meta) in &field.by_graph {
                let Some(requires) = meta.requires.as_deref().filter(|r| !r.is_empty()) else {
                    continue;
                };

                let Some(parsed_fields) = parse_fields(requires)? else {
                    continue;
                };

                mark_requires_usage_from_selection_set(
                    supergraph_state,
                    ObjectOrInterfaceTypeState::Object(object_type),
                    &parsed_fields,
                    graph_id,
                    overridden_graphs.contains(graph_id),
                    &mut index,
                );
            }
        }
    }

    Ok(index)
}

fn mark_requires_usage_from_selection_set(
    supergraph_state: &SupergraphState,
    current_type: ObjectOrInterfaceTypeState<'_>,
    selection_set: &SelectionSet,
    graph_id: &str,
    overridden_source_usage: bool,
    index: &mut RequiresFieldUsageIndex,
) {
    for selection in &selection_set.selections {
        match selection {
            Selection::Field(selection) => {
                let field_name = selection.name.value.as_str();
                let Some(field_type) = current_type.field_type(field_name) else {
                    continue;
                };

                mark_requires_usage_for_coordinate(
                    &to_field_coordinate(current_type.name(), field_name),
                    graph_id,
                    overridden_source_usage,
                    index,
                );

                let Some(sub_selection_set) = &selection.selection_set else {
                    continue;
                };

                let Some(output_type_name) = extract_named_type_name(field_type).filter(|n| !n.is_empty()) else {
                    continue;
                };

                let Some(next_type) = ObjectOrInterfaceTypeState::lookup(supergraph_state, &output_type_name) else {
                    continue;
                };

                mark_requires_usage_from_selection_set(
                    supergraph_state,
                    next_type,
                    sub_selection_set,
                    graph_id,
                    overridden_source_usage,
                    index,
                );
            }
            Selection::InlineFragment(selection) => {
                let target_type = match &selection.type_condition {
                    Some(type_condition) => {
                        ObjectOrInterfaceTypeState::lookup(supergraph_state, &type_condition.name.value)
                    }
                    None => Some(current_type),
                };

                let Some(target_type) = target_type else {
                    continue;
                };

                mark_requires_usage_from_selection_set(
                    supergraph_state,
                    target_type,
                    &selection.selection_set,
                    graph_id,
                    overridden_source_usage,
                    index,
                );
            }
            // `selection.kind !== Kind.INLINE_FRAGMENT`
            Selection::FragmentSpread(_) => continue,
        }
    }
}

fn mark_requires_usage_for_coordinate(
    coordinate: &str,
    graph_id: &str,
    overridden_source_usage: bool,
    index: &mut RequiresFieldUsageIndex,
) {
    let usage_by_graph = get_or_insert_with(index, coordinate, IndexMap::default);

    let usage_in_graph = get_or_insert_with(usage_by_graph, graph_id, RequiresUsageStateInGraph::default);

    if overridden_source_usage {
        usage_in_graph.is_only_overridden_required = true;
    } else {
        usage_in_graph.is_actively_required = true;
    }
}

/// `ObjectTypeState` (`kind: "object"`).
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub contexts: IndexSet<String>,
    pub cost: Option<f64>,
    pub has_definition: bool,
    pub by_graph: MapByGraph<ObjectTypeStateInGraph>,
    pub interfaces: IndexSet<String>,
    pub fields: IndexMap<String, ObjectTypeFieldState>,
    pub is_entity: bool,
    pub description: Option<Description>,
    pub ast: Ast,
}

/// `ObjectTypeFieldState`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeFieldState {
    pub name: String,
    pub r#type: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub is_leaf: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub list_size: Option<ListSize>,
    pub used_as_key: bool,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub by_graph: MapByGraph<FieldStateInGraph>,
    pub args: IndexMap<String, ObjectTypeFieldArgState>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
    /// `internal.seenNonExternal`.
    pub internal_seen_non_external: bool,
}

/// `ObjectTypeFieldArgState`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeFieldArgState {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub cost: Option<f64>,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub by_graph: MapByGraph<ArgStateInGraph>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub ast: Ast,
}

/// `ObjectTypeStateInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTypeStateInGraph {
    pub has_definition: bool,
    pub extension: bool,
    pub extension_type: Option<ExtensionType>,
    pub external: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub keys: Vec<Key>,
    pub interfaces: IndexSet<String>,
    pub inaccessible: bool,
    pub shareable: bool,
    pub version: FederationVersion,
}

/// `FieldStateInGraph` (object fields).
#[derive(Clone, Debug, PartialEq)]
pub struct FieldStateInGraph {
    pub r#type: String,
    pub external: bool,
    pub description: Option<Description>,
    pub inaccessible: bool,
    pub used: bool,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub provides: Option<String>,
    pub requires: Option<String>,
    pub provided: bool,
    pub required: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub shareable: bool,
    pub used_as_key: bool,
    pub extension: bool,
    pub version: FederationVersion,
}

/// `ArgStateInGraph` (object field arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct ArgStateInGraph {
    pub r#type: String,
    pub kind: ArgumentKind,
    pub inaccessible: bool,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub version: FederationVersion,
}

fn get_or_create_object_type<'s>(state: &'s mut IndexMap<String, ObjectTypeState>, type_name: &str) -> &'s mut ObjectTypeState {
    get_or_insert_with(state, type_name, || ObjectTypeState {
        name: type_name.to_string(),
        tags: IndexSet::default(),
        has_definition: false,
        is_entity: false,
        inaccessible: false,
        authenticated: false,
        policies: Vec::new(),
        scopes: Vec::new(),
        contexts: IndexSet::default(),
        cost: None,
        interfaces: IndexSet::default(),
        by_graph: IndexMap::default(),
        fields: IndexMap::default(),
        description: None,
        ast: Ast::default(),
    })
}

fn get_or_create_field<'s>(
    object_type_state: &'s mut ObjectTypeState,
    field_name: &str,
    field_type: &str,
) -> &'s mut ObjectTypeFieldState {
    get_or_insert_with(&mut object_type_state.fields, field_name, || ObjectTypeFieldState {
        name: field_name.to_string(),
        r#type: field_type.to_string(),
        is_leaf: false,
        tags: IndexSet::default(),
        inaccessible: false,
        authenticated: false,
        policies: Vec::new(),
        scopes: Vec::new(),
        cost: None,
        list_size: None,
        used_as_key: false,
        r#override: None,
        override_label: None,
        by_graph: IndexMap::default(),
        args: IndexMap::default(),
        description: None,
        deprecated: None,
        ast: Ast::default(),
        internal_seen_non_external: false,
    })
}

fn get_or_create_arg<'s>(
    field_state: &'s mut ObjectTypeFieldState,
    arg_name: &str,
    arg_type: &str,
    arg_kind: ArgumentKind,
) -> &'s mut ObjectTypeFieldArgState {
    get_or_insert_with(&mut field_state.args, arg_name, || ObjectTypeFieldArgState {
        name: arg_name.to_string(),
        r#type: arg_type.to_string(),
        kind: arg_kind,
        tags: IndexSet::default(),
        inaccessible: false,
        cost: None,
        default_value: None,
        from_context: None,
        by_graph: IndexMap::default(),
        description: None,
        deprecated: None,
        ast: Ast::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_in_graph(extension: bool, extension_type: Option<ExtensionType>, has_definition: bool) -> ObjectTypeStateInGraph {
        ObjectTypeStateInGraph {
            has_definition,
            extension,
            extension_type,
            external: false,
            authenticated: false,
            policies: Vec::new(),
            scopes: Vec::new(),
            keys: Vec::new(),
            interfaces: IndexSet::default(),
            inaccessible: false,
            shareable: false,
            version: FederationVersion::V1_0,
        }
    }

    #[test]
    fn federation_extension_rules() {
        let v1 = FederationVersion::V1_0;
        let v2 = FederationVersion::V2_0;
        // not an extension
        assert!(!is_federation_extension(&type_in_graph(false, Some(ExtensionType::Extends), false), v2));
        // v1 `extend type` is ignored
        assert!(!is_federation_extension(&type_in_graph(true, Some(ExtensionType::Extend), false), v1));
        assert!(!is_federation_extension(&type_in_graph(true, None, false), v1));
        // `@extends` always counts
        assert!(is_federation_extension(&type_in_graph(true, Some(ExtensionType::Extends), true), v1));
        assert!(is_federation_extension(&type_in_graph(true, Some(ExtensionType::Extends), true), v2));
        // v2 `extend type`: only without a definition
        assert!(!is_federation_extension(&type_in_graph(true, Some(ExtensionType::Extend), true), v2));
        assert!(is_federation_extension(&type_in_graph(true, Some(ExtensionType::Extend), false), v2));
    }

    #[test]
    fn last_index_of_matches_js() {
        assert_eq!(last_index_of_bang("String"), -1);
        assert_eq!(last_index_of_bang("String!"), 6);
        assert_eq!(last_index_of_bang("[A!]"), 2);
        assert_eq!(last_index_of_bang("[A!]!"), 4);
        // Least nullable wins: existing `[A!]!` vs incoming `[A!]` changes the type.
        assert!(last_index_of_bang("[A!]!") > last_index_of_bang("[A!]"));
        // existing `[A!]` vs incoming `[A!]!` keeps it.
        assert!(last_index_of_bang("[A!]") <= last_index_of_bang("[A!]!"));
        // existing `[A]!` vs incoming `[A!]`: 3 > 2, changes to `[A!]`.
        assert!(last_index_of_bang("[A]!") > last_index_of_bang("[A!]"));
    }

    #[test]
    fn requires_usage_marks_and_queries() {
        let mut index = RequiresFieldUsageIndex::default();
        mark_requires_usage_for_coordinate("User.id", "A", true, &mut index);
        assert!(!is_field_coordinate_effectively_required(&index, "User.id", "A", true));
        assert!(has_only_overridden_requires_dependency(&index, "User.id", "A"));
        // unknown coordinate/graph falls back
        assert!(is_field_coordinate_effectively_required(&index, "User.id", "B", true));
        assert!(!is_field_coordinate_effectively_required(&index, "User.name", "A", false));
        assert!(!has_only_overridden_requires_dependency(&index, "User.id", "B"));

        mark_requires_usage_for_coordinate("User.id", "A", false, &mut index);
        assert!(is_field_coordinate_effectively_required(&index, "User.id", "A", false));
        assert!(!has_only_overridden_requires_dependency(&index, "User.id", "A"));

        mark_requires_usage_for_coordinate("User.id", "B", false, &mut index);
        mark_requires_usage_for_coordinate("Book.id", "A", false, &mut index);
        assert_eq!(index.keys().collect::<Vec<_>>(), vec!["User.id", "Book.id"]);
        assert_eq!(index["User.id"].keys().collect::<Vec<_>>(), vec!["A", "B"]);
    }

    fn add_field(state: &mut IndexMap<String, ObjectTypeState>, type_name: &str, field_name: &str, field_type: &str) {
        let object_type = get_or_create_object_type(state, type_name);
        get_or_create_field(object_type, field_name, field_type);
    }

    fn selection_set(source: &str) -> SelectionSet {
        let document = graphql_js::parse(&format!("query {source}")).expect("valid selection");
        match document.definitions.into_iter().next() {
            Some(Definition::OperationDefinition(op)) => op.selection_set,
            _ => panic!("expected an operation"),
        }
    }

    #[test]
    fn requires_usage_from_selection_set_walks_fields_and_fragments() {
        let mut supergraph_state = SupergraphState::default();
        add_field(&mut supergraph_state.object_types, "User", "id", "ID!");
        add_field(&mut supergraph_state.object_types, "User", "profile", "Profile");
        add_field(&mut supergraph_state.object_types, "User", "pets", "[Pet!]!");
        add_field(&mut supergraph_state.object_types, "Profile", "name", "String");
        add_field(&mut supergraph_state.object_types, "Dog", "barks", "Boolean");

        let selection =
            selection_set("{ id unknown profile { name } pets { ... on Dog { barks } ... on Cat { meows } } ... { id } }");
        let mut index = RequiresFieldUsageIndex::default();
        let user = ObjectOrInterfaceTypeState::Object(&supergraph_state.object_types["User"]);
        mark_requires_usage_from_selection_set(&supergraph_state, user, &selection, "A", false, &mut index);

        // `Pet` is unknown, so its selection set is not walked; `unknown` is skipped.
        assert_eq!(
            index.keys().collect::<Vec<_>>(),
            vec!["User.id", "User.profile", "Profile.name", "User.pets"]
        );
        assert!(index.values().all(|by_graph| by_graph["A"].is_actively_required));
    }

    #[test]
    fn requires_usage_inline_fragment_with_known_type_condition() {
        let mut supergraph_state = SupergraphState::default();
        add_field(&mut supergraph_state.object_types, "User", "id", "ID!");
        add_field(&mut supergraph_state.object_types, "Dog", "barks", "Boolean");

        let selection = selection_set("{ ... on Dog { barks } ... on Cat { meows } }");
        let mut index = RequiresFieldUsageIndex::default();
        let user = ObjectOrInterfaceTypeState::Object(&supergraph_state.object_types["User"]);
        mark_requires_usage_from_selection_set(&supergraph_state, user, &selection, "B", true, &mut index);

        assert_eq!(index.keys().collect::<Vec<_>>(), vec!["Dog.barks"]);
        assert!(index["Dog.barks"]["B"].is_only_overridden_required);
        assert!(!index["Dog.barks"]["B"].is_actively_required);
    }

    #[test]
    fn empty_join_field_follows_js_truthiness() {
        assert!(is_empty_join_field(&JoinFieldAst::default()));
        assert!(is_empty_join_field(&JoinFieldAst {
            graph: Some("A".into()),
            external: Some(false),
            used_overridden: Some(false),
            r#override: Some(String::new()),
            context_arguments: Some(Vec::new()),
            ..Default::default()
        }));
        assert!(!is_empty_join_field(&JoinFieldAst {
            external: Some(true),
            ..Default::default()
        }));
        assert!(!is_empty_join_field(&JoinFieldAst {
            used_overridden: Some(true),
            ..Default::default()
        }));
        assert!(!is_empty_join_field(&JoinFieldAst {
            r#type: Some("String".into()),
            ..Default::default()
        }));
        assert!(!is_empty_join_field(&JoinFieldAst {
            context_arguments: Some(vec![JoinFieldContextArgumentAst::default()]),
            ..Default::default()
        }));
    }

    #[test]
    fn differences_some() {
        assert!(!DifferencesBetweenGraphs::default().some());
        assert!(DifferencesBetweenGraphs {
            context_arguments: true,
            ..Default::default()
        }
        .some());
    }

    #[test]
    fn get_or_create_keeps_first_insertion() {
        let mut state = IndexMap::default();
        add_field(&mut state, "User", "id", "ID!");
        add_field(&mut state, "User", "id", "ID");
        add_field(&mut state, "Book", "id", "ID");
        assert_eq!(state.keys().collect::<Vec<_>>(), vec!["User", "Book"]);
        assert_eq!(state["User"].fields["id"].r#type, "ID!");
        assert!(state["User"].cost.is_none());
    }
}
