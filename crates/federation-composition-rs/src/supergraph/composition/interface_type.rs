//! Port of `src/supergraph/composition/interface-type.ts`.

use graphql_rs::Definition;
use crate::utils::collections::{IndexMap, IndexSet};

use super::ast::{
    create_interface_type_node, CostAst, FieldArgumentAst, FieldAst, InterfaceTypeAst, JoinAst, JoinFieldAst,
    JoinImplementsAst, JoinTypeAst, ListSizeAst,
};
use super::common::{convert_to_const, get_or_insert_with, ComposeHelpers, Graph, Key, MapByGraph, TypeBuilder};
use super::object_type::ObjectTypeState;
use crate::specifications::federation::FederationVersion;
use crate::subgraph::state::{Ast, ArgumentKind, Deprecated, Description, FromContext, InterfaceType, ListSize};
use crate::supergraph::state::SubgraphStates;
use crate::utils::auth::merge_scope_policies;
use crate::utils::helpers::{ensure_value, math_max, math_max_nullable, nullable_array_union};
use crate::utils::js::JsResult;

/// `interfaceTypeBuilder()`.
#[derive(Clone, Debug, Default)]
pub struct InterfaceTypeBuilder;

/// `interfaceTypeBuilder()`.
pub fn interface_type_builder() -> InterfaceTypeBuilder {
    InterfaceTypeBuilder
}

impl InterfaceTypeBuilder {
    /// `composeSupergraphState(interfaceType, graphs, { supergraphState })`: collects
    /// `@requiresScopes`/`@policy`/`@authenticated` from the implementing object types. Only
    /// `supergraphState.objectTypes` is read, so that is what it receives.
    pub fn compose_supergraph_state(
        &mut self,
        interface_type: &mut InterfaceTypeState,
        _graphs: &SubgraphStates,
        object_types: &IndexMap<String, ObjectTypeState>,
    ) -> JsResult<()> {
        // Collect @requiresScopes and @policy from object types implementing this interface
        let implementors: Vec<&ObjectTypeState> = interface_type
            .implemented_by
            .iter()
            .filter_map(|type_name| object_types.get(type_name))
            .collect();
        for implementor in &implementors {
            interface_type.scopes = merge_scope_policies(&interface_type.scopes, &implementor.scopes);
            interface_type.policies = merge_scope_policies(&interface_type.policies, &implementor.policies);
            if implementor.authenticated {
                interface_type.authenticated = true;
            }
        }
        for field in interface_type.fields.values_mut() {
            for implementor in &implementors {
                let Some(implementor_field) = implementor.fields.get(&field.name) else {
                    continue;
                };
                // When a field comes from an interface object, we need to look for the implementation
                field.scopes = merge_scope_policies(&field.scopes, &implementor_field.scopes);
                field.policies = merge_scope_policies(&field.policies, &implementor_field.policies);
                if implementor_field.authenticated {
                    field.authenticated = true;
                }
            }
        }

        Ok(())
    }
}

impl TypeBuilder<InterfaceType, InterfaceTypeState> for InterfaceTypeBuilder {
    fn visit_subgraph_state(
        &mut self,
        graph: &Graph,
        state: &mut IndexMap<String, InterfaceTypeState>,
        type_name: &str,
        ty: &InterfaceType,
    ) {
        let interface_type_state = get_or_create_interface_type(state, type_name);

        for tag in &ty.tags {
            interface_type_state.tags.insert(tag.clone());
        }
        for context_name in &ty.contexts {
            interface_type_state.contexts.insert(format!("{}__{}", graph.name, context_name));
        }

        if ty.inaccessible {
            interface_type_state.inaccessible = true;
        }

        if ty.is_definition {
            interface_type_state.has_definition = true;
        }

        if ty.is_interface_object {
            interface_type_state.has_interface_object = true;
        }

        // First description wins
        if ty.description.is_some() && interface_type_state.description.is_none() {
            interface_type_state.description = ty.description.clone();
        }

        for directive in &ty.ast.directives {
            interface_type_state.ast.directives.push(directive.clone());
        }

        for interface_name in &ty.interfaces {
            interface_type_state.interfaces.insert(interface_name.clone());
        }
        for object_type_name in &ty.implemented_by {
            interface_type_state.implemented_by.insert(object_type_name.clone());
        }

        if !ty.keys.is_empty() {
            interface_type_state.is_entity = true;
        }

        // NOTE(port): the TS stores the subgraph's `keys`/`interfaces`/`implementedBy` by
        // reference; subgraph states are read-only by now, so copies are equivalent.
        interface_type_state.by_graph.insert(
            graph.id.clone(),
            InterfaceTypeInGraph {
                extension: ty.extension,
                keys: ty.keys.clone(),
                interfaces: ty.interfaces.clone(),
                implemented_by: ty.implemented_by.clone(),
                is_interface_object: ty.is_interface_object,
                version: graph.version,
            },
        );

        // PERF: a new type state gets room for the fields up front (only the capacity changes).
        if interface_type_state.fields.is_empty() {
            interface_type_state.fields.reserve(ty.fields.len());
        }

        for field in ty.fields.values() {
            let field_state = get_or_create_interface_field(interface_type_state, &field.name, &field.r#type);

            for tag in &field.tags {
                field_state.tags.insert(tag.clone());
            }

            if !field.r#type.ends_with('!') && field_state.r#type.ends_with('!') {
                // Replace the non-null type with a nullable type
                field_state.r#type = field.r#type.clone();
            }

            if field.is_leaf {
                field_state.is_leaf = true;
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

            // First deprecation wins
            if field.deprecated.is_some() && field_state.deprecated.is_none() {
                field_state.deprecated = field.deprecated.clone();
            }

            // First description wins
            if field.description.is_some() && field_state.description.is_none() {
                field_state.description = field.description.clone();
            }

            for directive in &field.ast.directives {
                field_state.ast.directives.push(directive.clone());
            }

            let used_as_key = ty.fields_used_as_keys.contains(&field.name);

            if used_as_key {
                field_state.used_as_key = true;
            }

            field_state.by_graph.insert(
                graph.id.clone(),
                FieldStateInGraph {
                    r#type: field.r#type.clone(),
                    r#override: field.r#override.clone(),
                    override_label: field.override_label.clone(),
                    provides: field.provides.clone(),
                    requires: field.requires.clone(),
                    version: graph.version,
                    external: field.external,
                    shareable: field.shareable,
                    authenticated: field.authenticated,
                    policies: field.policies.clone(),
                    scopes: field.scopes.clone(),
                    used_as_key,
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

                if arg.deprecated.is_some() && arg_state.deprecated.is_none() {
                    arg_state.deprecated = arg.deprecated.clone();
                }

                // First description wins
                if arg.description.is_some() && arg_state.description.is_none() {
                    arg_state.description = arg.description.clone();
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

                for directive in &arg.ast.directives {
                    arg_state.ast.directives.push(directive.clone());
                }

                arg_state.kind = arg.kind;

                arg_state.by_graph.insert(
                    graph.id.clone(),
                    ArgStateInGraph {
                        r#type: arg.r#type.clone(),
                        kind: arg.kind,
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
        interface_type: &InterfaceTypeState,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition> {
        let supergraph_state = helpers.supergraph_state;

        let mut fields: Vec<FieldAst> = Vec::with_capacity(interface_type.fields.len());
        for field in interface_type.fields.values() {
            let mut non_empty_join_field = false;
            let mut join_fields: Vec<JoinFieldAst> = Vec::new();

            if field.by_graph.len() != interface_type.by_graph.len() {
                for (graph_id, meta) in &field.by_graph {
                    if meta.r#type != field.r#type
                        || truthy(&meta.r#override)
                        || truthy(&meta.provides)
                        || truthy(&meta.requires)
                        || meta.external
                    {
                        non_empty_join_field = true;
                    }

                    join_fields.push(JoinFieldAst {
                        graph: Some(graph_id.clone()),
                        r#type: if meta.r#type == field.r#type {
                            None
                        } else {
                            Some(meta.r#type.clone())
                        },
                        r#override: meta.r#override.clone(),
                        provides: meta.provides.clone(),
                        requires: meta.requires.clone(),
                        external: Some(meta.external),
                        ..Default::default()
                    });
                }
            }
            // NOTE(port): `nonEmptyJoinField` is computed but never read in the TS.
            let _ = non_empty_join_field;

            // The object literal's properties are evaluated in order: `cost`, `listSize`, then
            // `arguments` (each `ensureValue` can throw).
            let cost = match field.cost {
                Some(cost) => Some(CostAst {
                    cost,
                    directive_name: ensure_value(
                        supergraph_state.specs.cost.names.cost.clone(),
                        "Directive name of @cost is not defined",
                    )?,
                }),
                None => None,
            };
            let list_size = match &field.list_size {
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
                    tags: arg.tags.iter().cloned().collect(),
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
                    default_value: arg.default_value.clone(),
                    deprecated: arg.deprecated.clone(),
                    description: arg.description.clone(),
                    ast: convert_to_const(&arg.ast.directives),
                    // not passed in the TS (`undefined`)
                    inaccessible: false,
                });
            }

            fields.push(FieldAst {
                name: field.name.clone(),
                r#type: field.r#type.clone(),
                inaccessible: field.inaccessible,
                authenticated: field.authenticated,
                policies: field.policies.clone(),
                scopes: field.scopes.clone(),
                cost,
                list_size,
                tags: field.tags.iter().cloned().collect(),
                deprecated: field.deprecated.clone(),
                description: field.description.clone(),
                ast: convert_to_const(&field.ast.directives),
                arguments,
                join: JoinAst {
                    field: join_fields,
                    ..Default::default()
                },
            });
        }

        let mut join_types: Vec<JoinTypeAst> = Vec::new();
        for (graph_id, meta) in &interface_type.by_graph {
            if !meta.keys.is_empty() && graph_federation_version(graphs, graph_id) != FederationVersion::V1_0 {
                for key in &meta.keys {
                    join_types.push(JoinTypeAst {
                        graph: graph_id.clone(),
                        key: Some(key.fields.clone()),
                        extension: Some(meta.extension),
                        is_interface_object: Some(meta.is_interface_object),
                        resolvable: Some(key.resolvable),
                    });
                }
                continue;
            }

            join_types.push(JoinTypeAst {
                graph: graph_id.clone(),
                ..Default::default()
            });
        }

        let mut implements: Vec<JoinImplementsAst> = Vec::new();
        if !interface_type.interfaces.is_empty() {
            for (graph_id, meta) in &interface_type.by_graph {
                for iface in &meta.interfaces {
                    implements.push(JoinImplementsAst {
                        graph: graph_id.clone(),
                        interface: iface.clone(),
                    });
                }
            }
        }

        Ok(Definition::InterfaceTypeDefinition(create_interface_type_node(InterfaceTypeAst {
            name: interface_type.name.clone(),
            fields,
            tags: interface_type.tags.iter().cloned().collect(),
            inaccessible: interface_type.inaccessible,
            authenticated: interface_type.authenticated,
            policies: interface_type.policies.clone(),
            scopes: interface_type.scopes.clone(),
            contexts: interface_type.contexts.iter().cloned().collect(),
            description: interface_type.description.clone(),
            interfaces: interface_type.interfaces.iter().cloned().collect(),
            ast: convert_to_const(&interface_type.ast.directives),
            join: JoinAst {
                r#type: join_types,
                implements,
                ..Default::default()
            },
        })))
    }
}

/// JS truthiness of a `string | null` value.
pub(crate) fn truthy(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|v| !v.is_empty())
}

/// `graphs.get(graphId)!.federation.version`.
/// NOTE(port): every graph id in a type's `byGraph` belongs to a subgraph added to the
/// supergraph state; the TS would throw a `TypeError` otherwise.
pub(crate) fn graph_federation_version(graphs: &SubgraphStates, graph_id: &str) -> FederationVersion {
    graphs
        .get(graph_id)
        .expect("Cannot read properties of undefined (reading 'federation')")
        .federation
        .version
}

/// The `listSize` merge of `visitSubgraphState` (the object builder has the same code):
/// `{ printRequireOneSlicingArgument: false, assumedSize: mathMaxNullable(..), ... }`.
pub(crate) fn merge_list_size(existing: Option<&ListSize>, incoming: &ListSize) -> ListSize {
    ListSize {
        print_require_one_slicing_argument: false,
        assumed_size: math_max_nullable(existing.and_then(|l| l.assumed_size), incoming.assumed_size),
        // prefer `false`
        require_one_slicing_argument: existing.map_or(true, |l| l.require_one_slicing_argument)
            && incoming.require_one_slicing_argument,
        slicing_arguments: nullable_array_union(
            existing.and_then(|l| l.slicing_arguments.as_deref()),
            incoming.slicing_arguments.as_deref(),
        ),
        sized_fields: nullable_array_union(
            existing.and_then(|l| l.sized_fields.as_deref()),
            incoming.sized_fields.as_deref(),
        ),
    }
}

/// `{ ...field.listSize, directiveName }`.
pub(crate) fn list_size_ast(list_size: &ListSize, directive_name: String) -> ListSizeAst {
    ListSizeAst {
        assumed_size: list_size.assumed_size,
        slicing_arguments: list_size.slicing_arguments.clone(),
        sized_fields: list_size.sized_fields.clone(),
        require_one_slicing_argument: list_size.require_one_slicing_argument,
        print_require_one_slicing_argument: list_size.print_require_one_slicing_argument,
        directive_name,
    }
}

/// `InterfaceTypeState` (`kind: "interface"`).
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeState {
    pub name: String,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub contexts: IndexSet<String>,
    pub has_definition: bool,
    pub description: Option<Description>,
    pub by_graph: MapByGraph<InterfaceTypeInGraph>,
    pub interfaces: IndexSet<String>,
    pub implemented_by: IndexSet<String>,
    pub fields: IndexMap<String, InterfaceTypeFieldState>,
    pub has_interface_object: bool,
    pub is_entity: bool,
    pub ast: Ast,
}

/// `InterfaceTypeFieldState`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeFieldState {
    pub name: String,
    pub r#type: String,
    pub is_leaf: bool,
    pub tags: IndexSet<String>,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
    pub cost: Option<f64>,
    pub list_size: Option<ListSize>,
    pub deprecated: Option<Deprecated>,
    pub description: Option<Description>,
    pub used_as_key: bool,
    pub by_graph: MapByGraph<FieldStateInGraph>,
    pub args: IndexMap<String, InterfaceTypeFieldArgState>,
    pub ast: Ast,
}

/// `InterfaceTypeFieldArgState`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeFieldArgState {
    pub name: String,
    pub r#type: String,
    pub kind: ArgumentKind,
    pub tags: IndexSet<String>,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub cost: Option<f64>,
    pub description: Option<Description>,
    pub deprecated: Option<Deprecated>,
    pub by_graph: MapByGraph<ArgStateInGraph>,
    pub ast: Ast,
}

/// `InterfaceTypeInGraph`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceTypeInGraph {
    pub extension: bool,
    pub keys: Vec<Key>,
    pub interfaces: IndexSet<String>,
    pub implemented_by: IndexSet<String>,
    pub is_interface_object: bool,
    pub version: FederationVersion,
}

/// `FieldStateInGraph` (interface fields).
#[derive(Clone, Debug, PartialEq)]
pub struct FieldStateInGraph {
    pub r#type: String,
    pub r#override: Option<String>,
    pub override_label: Option<String>,
    pub provides: Option<String>,
    pub shareable: bool,
    pub used_as_key: bool,
    pub external: bool,
    pub requires: Option<String>,
    pub version: FederationVersion,
    pub authenticated: bool,
    pub policies: Vec<Vec<String>>,
    pub scopes: Vec<Vec<String>>,
}

/// `ArgStateInGraph` (interface field arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct ArgStateInGraph {
    pub r#type: String,
    pub kind: ArgumentKind,
    pub default_value: Option<String>,
    pub from_context: Option<FromContext>,
    pub version: FederationVersion,
}

fn get_or_create_interface_type<'s>(
    state: &'s mut IndexMap<String, InterfaceTypeState>,
    type_name: &str,
) -> &'s mut InterfaceTypeState {
    get_or_insert_with(state, type_name, || InterfaceTypeState {
        name: type_name.to_string(),
        tags: IndexSet::default(),
        inaccessible: false,
        authenticated: false,
        policies: Vec::new(),
        scopes: Vec::new(),
        contexts: IndexSet::default(),
        has_definition: false,
        has_interface_object: false,
        is_entity: false,
        by_graph: IndexMap::default(),
        fields: IndexMap::default(),
        interfaces: IndexSet::default(),
        implemented_by: IndexSet::default(),
        description: None,
        ast: Ast::default(),
    })
}

fn get_or_create_interface_field<'s>(
    interface_type_state: &'s mut InterfaceTypeState,
    field_name: &str,
    field_type: &str,
) -> &'s mut InterfaceTypeFieldState {
    get_or_insert_with(&mut interface_type_state.fields, field_name, || InterfaceTypeFieldState {
        name: field_name.to_string(),
        r#type: field_type.to_string(),
        is_leaf: false,
        used_as_key: false,
        tags: IndexSet::default(),
        inaccessible: false,
        authenticated: false,
        policies: Vec::new(),
        scopes: Vec::new(),
        cost: None,
        list_size: None,
        deprecated: None,
        description: None,
        by_graph: IndexMap::default(),
        args: IndexMap::default(),
        ast: Ast::default(),
    })
}

fn get_or_create_arg<'s>(
    field_state: &'s mut InterfaceTypeFieldState,
    arg_name: &str,
    arg_type: &str,
    arg_kind: ArgumentKind,
) -> &'s mut InterfaceTypeFieldArgState {
    get_or_insert_with(&mut field_state.args, arg_name, || InterfaceTypeFieldArgState {
        name: arg_name.to_string(),
        r#type: arg_type.to_string(),
        kind: arg_kind,
        tags: IndexSet::default(),
        default_value: None,
        from_context: None,
        cost: None,
        description: None,
        deprecated: None,
        by_graph: IndexMap::default(),
        ast: Ast::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_size(
        assumed_size: Option<f64>,
        slicing_arguments: Option<Vec<&str>>,
        sized_fields: Option<Vec<&str>>,
        require_one_slicing_argument: bool,
    ) -> ListSize {
        ListSize {
            print_require_one_slicing_argument: true,
            assumed_size,
            slicing_arguments: slicing_arguments.map(|v| v.into_iter().map(String::from).collect()),
            sized_fields: sized_fields.map(|v| v.into_iter().map(String::from).collect()),
            require_one_slicing_argument,
        }
    }

    #[test]
    fn merge_list_size_first_visit() {
        let incoming = list_size(Some(5.0), Some(vec!["first"]), None, true);
        let merged = merge_list_size(None, &incoming);
        // `fieldState.listSize?.requireOneSlicingArgument ?? true` is `true` on the first visit.
        assert!(merged.require_one_slicing_argument);
        assert!(!merged.print_require_one_slicing_argument);
        assert_eq!(merged.assumed_size, Some(5.0));
        assert_eq!(merged.slicing_arguments, Some(vec!["first".to_string()]));
        assert_eq!(merged.sized_fields, None);
    }

    #[test]
    fn merge_list_size_prefers_false_and_unions() {
        let existing = list_size(Some(3.0), Some(vec!["first", "last"]), Some(vec!["edges"]), false);
        let incoming = list_size(Some(7.0), Some(vec!["last", "after"]), None, true);
        let merged = merge_list_size(Some(&existing), &incoming);
        assert!(!merged.require_one_slicing_argument);
        assert_eq!(merged.assumed_size, Some(7.0));
        assert_eq!(
            merged.slicing_arguments,
            Some(vec!["first".to_string(), "last".to_string(), "after".to_string()])
        );
        assert_eq!(merged.sized_fields, Some(vec!["edges".to_string()]));
    }

    #[test]
    fn merge_list_size_assumed_size_nullable() {
        let existing = list_size(None, None, None, true);
        let incoming = list_size(Some(2.0), None, None, true);
        assert_eq!(merge_list_size(Some(&existing), &incoming).assumed_size, Some(2.0));
        let incoming = list_size(None, None, None, true);
        assert_eq!(merge_list_size(Some(&existing), &incoming).assumed_size, None);
    }

    #[test]
    fn truthy_matches_js_strings() {
        assert!(!truthy(&None));
        assert!(!truthy(&Some(String::new())));
        assert!(truthy(&Some("a".to_string())));
    }
}
