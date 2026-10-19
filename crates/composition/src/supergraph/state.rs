//! Port of `src/supergraph/state.ts`.
//!
//! `createSupergraphStateBuilder()` returns closures over `state`, the type builders,
//! `subgraphStates` and `graphNameToIdMap`; here they are the fields of
//! [`SupergraphStateBuilder`] and the closures are its methods.
//! Subgraph states are immutable once validated and are shared by the validation context, the
//! supergraph state and the builder, so they are `Rc<SubgraphState>`.

use std::rc::Rc;

use graphql_js::{strip_ignored_characters, Definition, Selection, SelectionSet};
use indexmap::{IndexMap, IndexSet};

use super::composition::ast::{create_join_graph_enum_type_node, JoinGraphAst};
use super::composition::common::{ComposeHelpers, TypeBuilder};
use super::composition::directive::{
    directive_builder, is_argument_type_identical_in_every_subgraph, DirectiveBuilder, DirectiveState,
};
use super::composition::enum_type::{enum_type_builder, EnumTypeBuilder, EnumTypeState};
use super::composition::input_object_type::{input_object_type_builder, InputObjectTypeBuilder, InputObjectTypeState};
use super::composition::interface_type::{interface_type_builder, InterfaceTypeBuilder, InterfaceTypeState};
use super::composition::object_type::{object_type_builder, ObjectTypeBuilder, ObjectTypeState};
use super::composition::scalar_type::{scalar_type_builder, ScalarTypeBuilder, ScalarTypeState};
use super::composition::union_type::{union_type_builder, UnionTypeBuilder, UnionTypeState};
use crate::specifications::link::Link;
use crate::graphql::printer::print;
use crate::subgraph::helpers::parse_fields;
use crate::subgraph::state::{is_executable_directive_location, CostSpec, CostSpecNames, SubgraphState, SubgraphType};
use crate::utils::js::{JsError, JsResult};

/// Subgraph states by graph id (`Map<string, SubgraphState>`).
pub type SubgraphStates = IndexMap<String, Rc<SubgraphState>>;

/// `Link & { graph: string }`. `linkWithGraph` mutates the subgraph's link object in TS; the
/// port copies it (nothing reads `graph` through the subgraph state).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkWithGraph {
    pub link: Link,
    pub graph: String,
}

/// `SupergraphState["specs"]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SupergraphSpecs {
    pub tag: bool,
    pub context: bool,
    pub cost: CostSpec,
    pub inaccessible: bool,
    pub authenticated: bool,
    pub requires_scopes: bool,
    pub policy: bool,
}

/// `SupergraphState`.
#[derive(Clone, Debug, Default)]
pub struct SupergraphState {
    pub subgraphs: SubgraphStates,
    pub scalar_types: IndexMap<String, ScalarTypeState>,
    pub object_types: IndexMap<String, ObjectTypeState>,
    pub interface_types: IndexMap<String, InterfaceTypeState>,
    pub union_types: IndexMap<String, UnionTypeState>,
    pub enum_types: IndexMap<String, EnumTypeState>,
    pub input_object_types: IndexMap<String, InputObjectTypeState>,
    pub directives: IndexMap<String, DirectiveState>,
    pub links: Vec<LinkWithGraph>,
    pub specs: SupergraphSpecs,
}

/// `SupergraphStateBuilder = ReturnType<typeof createSupergraphStateBuilder>`.
#[derive(Debug)]
pub struct SupergraphStateBuilder {
    state: SupergraphState,
    directive: DirectiveBuilder,
    scalar_type: ScalarTypeBuilder,
    enum_type: EnumTypeBuilder,
    input_object_type: InputObjectTypeBuilder,
    interface_type: InterfaceTypeBuilder,
    object_type: ObjectTypeBuilder,
    union_type: UnionTypeBuilder,
    subgraph_states: SubgraphStates,
    /// `graphNameToIdMap` (a plain object in TS; only looked up).
    graph_name_to_id_map: IndexMap<String, String>,
}

/// `createSupergraphStateBuilder()`.
pub fn create_supergraph_state_builder() -> SupergraphStateBuilder {
    SupergraphStateBuilder {
        state: SupergraphState {
            subgraphs: IndexMap::new(),
            scalar_types: IndexMap::new(),
            object_types: IndexMap::new(),
            interface_types: IndexMap::new(),
            union_types: IndexMap::new(),
            enum_types: IndexMap::new(),
            input_object_types: IndexMap::new(),
            directives: IndexMap::new(),
            links: vec![],
            specs: SupergraphSpecs {
                tag: false,
                context: false,
                cost: CostSpec {
                    used: false,
                    names: CostSpecNames {
                        cost: None,
                        list_size: None,
                    },
                },
                inaccessible: false,
                policy: false,
                requires_scopes: false,
                authenticated: false,
            },
        },
        directive: directive_builder(),
        scalar_type: scalar_type_builder(),
        enum_type: enum_type_builder(),
        input_object_type: input_object_type_builder(),
        interface_type: interface_type_builder(),
        object_type: object_type_builder(),
        union_type: union_type_builder(),
        subgraph_states: IndexMap::new(),
        graph_name_to_id_map: IndexMap::new(),
    }
}

impl SupergraphStateBuilder {
    /// `addSubgraph(subgraph)`. Throws `Graph with ID "X" already exists`.
    pub fn add_subgraph(&mut self, subgraph: Rc<SubgraphState>) -> JsResult<()> {
        if self.state.subgraphs.contains_key(&subgraph.graph.id) {
            return Err(JsError::error(format!(
                "Graph with ID \"{}\" already exists",
                subgraph.graph.id
            )));
        }

        self.graph_name_to_id_map
            .insert(subgraph.graph.name.clone(), subgraph.graph.id.clone());
        self.state.subgraphs.insert(subgraph.graph.id.clone(), subgraph);
        Ok(())
    }

    pub fn get_graph(&self, id: &str) -> Option<&Rc<SubgraphState>> {
        self.state.subgraphs.get(id)
    }

    /// `composeSupergraphState()`: runs the builders' `composeSupergraphState` over every
    /// type state (objects, then interfaces; the other builders have none).
    pub fn compose_supergraph_state(&mut self) -> JsResult<()> {
        // Directive, Scalar, Enum, InputObject: those builders have no `composeSupergraphState`.
        // Object
        for s in self.state.object_types.values_mut() {
            self.object_type
                .compose_supergraph_state(s, &self.state.subgraphs, &self.state.interface_types)?;
        }
        // Interface
        for s in self.state.interface_types.values_mut() {
            self.interface_type
                .compose_supergraph_state(s, &self.state.subgraphs, &self.state.object_types)?;
        }
        // Union: no `composeSupergraphState`.
        Ok(())
    }

    pub fn visit_subgraph_state(&mut self, subgraph_state: Rc<SubgraphState>) {
        self.subgraph_states
            .insert(subgraph_state.graph.id.clone(), subgraph_state.clone());
        for link in &subgraph_state.links {
            self.state
                .links
                .push(link_with_graph(link, &subgraph_state.graph.id));
        }

        let specs = &mut self.state.specs;
        let subgraph_specs = &subgraph_state.specs;

        if subgraph_specs.tag {
            specs.tag = true;
        }

        if subgraph_specs.context {
            specs.context = true;
        }

        if subgraph_specs.inaccessible {
            specs.inaccessible = true;
        }

        if subgraph_specs.requires_scopes {
            specs.requires_scopes = true;
        }

        if subgraph_specs.authenticated {
            specs.authenticated = true;
        }

        if subgraph_specs.policy {
            specs.policy = true;
        }

        if subgraph_specs.cost.used {
            specs.cost.used = true;

            // Truthiness checks on strings: `""` is falsy.
            if subgraph_specs.cost.names.cost.as_deref().is_some_and(|n| !n.is_empty()) {
                specs.cost.names.cost = subgraph_specs.cost.names.cost.clone();
            }

            if subgraph_specs.cost.names.list_size.as_deref().is_some_and(|n| !n.is_empty()) {
                specs.cost.names.list_size = subgraph_specs.cost.names.list_size.clone();
            }
        }

        let all_federation_imports: IndexSet<&str> = subgraph_state
            .federation
            .imports
            .iter()
            .map(|i| i.alias.as_deref().unwrap_or(&i.name))
            .collect();

        let graph = &subgraph_state.graph;
        for (type_name, ty) in &subgraph_state.types {
            // Federation Types that are imported, but also declared within th subgraph (most-likely for IDE support)
            // should not be added to the supergraph.
            if all_federation_imports.contains(type_name.as_str()) {
                continue;
            }

            match ty {
                SubgraphType::Object(ty) => {
                    self.object_type
                        .visit_subgraph_state(graph, &mut self.state.object_types, type_name, ty);
                }
                SubgraphType::Interface(ty) => {
                    self.interface_type
                        .visit_subgraph_state(graph, &mut self.state.interface_types, type_name, ty);
                }
                SubgraphType::Union(ty) => {
                    self.union_type
                        .visit_subgraph_state(graph, &mut self.state.union_types, type_name, ty);
                }
                SubgraphType::Enum(ty) => {
                    self.enum_type
                        .visit_subgraph_state(graph, &mut self.state.enum_types, type_name, ty);
                }
                SubgraphType::InputObject(ty) => {
                    self.input_object_type.visit_subgraph_state(
                        graph,
                        &mut self.state.input_object_types,
                        type_name,
                        ty,
                    );
                }
                SubgraphType::Directive(ty) => {
                    self.directive
                        .visit_subgraph_state(graph, &mut self.state.directives, type_name, ty);
                }
                SubgraphType::Scalar(ty) => {
                    self.scalar_type
                        .visit_subgraph_state(graph, &mut self.state.scalar_types, type_name, ty);
                }
            }
        }
    }

    pub fn get_supergraph_state(&self) -> &SupergraphState {
        &self.state
    }

    pub fn get_subgraph_state(&self, graph_id: &str) -> Option<&Rc<SubgraphState>> {
        self.subgraph_states.get(graph_id)
    }

    /// `links()`: `mergeLinks(state.links)`.
    pub fn links(&self) -> Vec<Link> {
        crate::specifications::link::merge_links(self.state.links.iter().map(|l| &l.link))
    }

    /// `build()`: the supergraph type and directive definitions.
    /// NOTE(port): the TS transforms each object type (`transformFields(objectTypeState)`, which
    /// mutates its fields' `requires`/`provides`) right before composing it, and the object
    /// builder lazily indexes `@requires` usage across all object types on the first compose;
    /// keep that order. Also strips executable directives not identical in every subgraph
    /// (removes from `state.directives` while iterating it; use `shift_remove` after the loop
    /// or iterate over a snapshot of names, preserving the TS outcome).
    pub fn build(&mut self) -> JsResult<Vec<Definition>> {
        // Strip out all executable directives that are not defined or identical every supergraph
        // NOTE(port): the TS deletes the current entry while iterating `state.directives`
        // (JS map iteration continues with the next entry), so iterating a snapshot of the
        // names and removing with `shift_remove` gives the same result.
        let directive_names: Vec<String> = self.state.directives.keys().cloned().collect();
        let total_subgraph_size = self.state.subgraphs.len();
        for directive_name in directive_names {
            let directive_state = &self.state.directives[&directive_name];
            if !directive_state.is_executable || directive_state.by_graph.is_empty() {
                continue;
            }

            let is_all_directives_identical = directive_state
                .args
                .values()
                .all(|directive_arg_state| {
                    is_argument_type_identical_in_every_subgraph(directive_arg_state, total_subgraph_size)
                });

            if !is_all_directives_identical {
                self.state.directives.shift_remove(&directive_name);
                continue;
            }

            let directive_state = self
                .state
                .directives
                .get_mut(&directive_name)
                .expect("directive state exists");

            // Remove all executable locations not shared by all subgraphs
            // NOTE(port): `locations.forEach` deletes the current element while iterating, which
            // JS sets allow; a snapshot of the locations gives the same result.
            let locations: Vec<String> = directive_state.locations.iter().cloned().collect();
            for location in locations {
                // if it is not an executable location -> remove
                if !is_executable_directive_location(&location) {
                    // If it is a compose directive we want to retain the schema location.
                    if directive_state.composed {
                        continue;
                    }

                    directive_state.locations.shift_remove(&location);
                    continue;
                }

                let missing_in_some_graph = directive_state
                    .by_graph
                    .values()
                    .any(|directive_state_in_graph| !directive_state_in_graph.locations.contains(&location));
                if missing_in_some_graph {
                    directive_state.locations.shift_remove(&location);
                }
            }

            // If there is no location left, we need to remove this executable directive
            if directive_state.locations.is_empty() {
                self.state.directives.shift_remove(&directive_name);
            }
        }

        // It's important to keep that number correct.
        let number_of_expected_nodes = self.state.directives.len()
            + self.state.scalar_types.len()
            + self.state.object_types.len()
            + self.state.interface_types.len()
            + self.state.union_types.len()
            + self.state.enum_types.len()
            + self.state.input_object_types.len()
            + 1;
        // Allocate the elements in the array for performance
        let mut nodes: Vec<Definition> = Vec::with_capacity(number_of_expected_nodes);

        nodes.push(Definition::EnumTypeDefinition(create_join_graph_enum_type_node(
            &self
                .state
                .subgraphs
                .values()
                .map(|subgraph| JoinGraphAst {
                    name: subgraph.graph.name.clone(),
                    enum_value: subgraph.graph.id.clone(),
                    url: Some(subgraph.graph.url.clone().unwrap_or_default()),
                })
                .collect::<Vec<_>>(),
        )));

        for directive_state in self.state.directives.values() {
            nodes.push(self.directive.compose_supergraph_node(
                directive_state,
                &self.state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &self.state,
                    graph_name_to_id_map: &self.graph_name_to_id_map,
                },
            )?);
        }

        for scalar_type_state in self.state.scalar_types.values() {
            nodes.push(self.scalar_type.compose_supergraph_node(
                scalar_type_state,
                &self.state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &self.state,
                    graph_name_to_id_map: &self.graph_name_to_id_map,
                },
            )?);
        }

        // `transformFields(objectTypeState)` runs right before each object type is composed.
        for index in 0..self.state.object_types.len() {
            let object_type_name = self
                .state
                .object_types
                .get_index(index)
                .map(|(name, _)| name.clone())
                .expect("object type exists");
            visit_type_state(&mut self.state, &object_type_name)?;
            let object_type_state = &self.state.object_types[index];
            nodes.push(self.object_type.compose_supergraph_node(
                object_type_state,
                &self.state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &self.state,
                    graph_name_to_id_map: &self.graph_name_to_id_map,
                },
            )?);
        }

        for interface_type_state in self.state.interface_types.values() {
            nodes.push(self.interface_type.compose_supergraph_node(
                interface_type_state,
                &self.state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &self.state,
                    graph_name_to_id_map: &self.graph_name_to_id_map,
                },
            )?);
        }

        for union_type_state in self.state.union_types.values() {
            nodes.push(self.union_type.compose_supergraph_node(
                union_type_state,
                &self.state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &self.state,
                    graph_name_to_id_map: &self.graph_name_to_id_map,
                },
            )?);
        }

        for enum_type_state in self.state.enum_types.values() {
            nodes.push(self.enum_type.compose_supergraph_node(
                enum_type_state,
                &self.state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &self.state,
                    graph_name_to_id_map: &self.graph_name_to_id_map,
                },
            )?);
        }

        for input_object_type_state in self.state.input_object_types.values() {
            nodes.push(self.input_object_type.compose_supergraph_node(
                input_object_type_state,
                &self.state.subgraphs,
                ComposeHelpers {
                    supergraph_state: &self.state,
                    graph_name_to_id_map: &self.graph_name_to_id_map,
                },
            )?);
        }

        Ok(nodes)
    }
}

/// `linkWithGraph(link, graph)`.
fn link_with_graph(link: &Link, graph: &str) -> LinkWithGraph {
    LinkWithGraph {
        link: link.clone(),
        graph: graph.to_string(),
    }
}

fn visit_selection_set_node(selection_set: &SelectionSet, locations: &mut IndexSet<String>, current_path: &[String]) {
    for selection in &selection_set.selections {
        if let Selection::Field(selection) = selection {
            let mut path = current_path.to_vec();
            path.push(selection.name.value.clone());
            if let Some(child) = &selection.selection_set {
                visit_selection_set_node(child, locations, &path);
            } else {
                locations.insert(path.join("."));
            }
        }
    }
}

fn create_fields_set(selection_set: &SelectionSet) -> IndexSet<String> {
    let mut locations = IndexSet::new();
    visit_selection_set_node(selection_set, &mut locations, &[]);
    locations
}

/// `filterSelectionSetNode(selectionSet, removableFields, currentPath)`: removes removable
/// leaf fields in place.
fn filter_selection_set_node(selection_set: &mut SelectionSet, removable_fields: &IndexSet<String>, current_path: &str) {
    let mut fields_to_delete: Vec<String> = Vec::new();
    for index in 0..selection_set.selections.len() {
        let Selection::Field(selection) = &mut selection_set.selections[index] else {
            continue;
        };
        let name = selection.name.value.clone();
        if let Some(child) = &mut selection.selection_set {
            // NOTE(port): `currentPath + "." + name` even when `currentPath` is `""`, so nested
            // paths start with a dot and never match the dot-joined paths of `createFieldsSet`;
            // kept.
            filter_selection_set_node(child, removable_fields, &format!("{current_path}.{name}"));

            // NOTE(port): the TS checks the parent's selections here, not the child's (so this
            // is never true while iterating them); kept.
            if selection_set.selections.is_empty() {
                fields_to_delete.push(name);
            }
        } else {
            let looking_for = if current_path.is_empty() {
                name.clone()
            } else {
                format!("{current_path}.{name}")
            };
            if removable_fields.contains(&looking_for) {
                fields_to_delete.push(name);
            }
        }
    }

    for field_name in fields_to_delete {
        selection_set.selections.retain(|selection| match selection {
            Selection::Field(f) => f.name.value != field_name,
            _ => true,
        });
    }
}

/// `createFieldsTransformer(state)` returns `visitTypeState`; its inner functions
/// `transformFields` and `mergeKeyFieldsCollection` are the free functions below.
/// The returned function mutates the object type state at `object_type_name` in `state`.
/// NOTE(port): the TS throws `Referenced type not found: X` for a `@provides` on a field
/// whose output type is unknown.
fn visit_type_state(state: &mut SupergraphState, object_type_name: &str) -> JsResult<()> {
    let type_index = state
        .object_types
        .get_index_of(object_type_name)
        .expect("object type exists");
    let field_count = state.object_types[type_index].fields.len();
    for field_index in 0..field_count {
        let graph_count = state.object_types[type_index].fields[field_index].by_graph.len();
        for graph_index in 0..graph_count {
            let type_state = &state.object_types[type_index];
            let field_state = &type_state.fields[field_index];
            let (graph_id, field_state_in_graph) = field_state
                .by_graph
                .get_index(graph_index)
                .expect("graph index in range");
            let graph_id = graph_id.clone();

            if let Some(requires) = field_state_in_graph.requires.clone().filter(|r| !r.is_empty()) {
                let keys: Vec<String> = type_state
                    .by_graph
                    .get(&graph_id)
                    .ok_or_else(|| JsError::type_error("Cannot read properties of undefined (reading 'keys')"))?
                    .keys
                    .iter()
                    .map(|key| key.fields.clone())
                    .collect();
                let key_fields = merge_key_fields_collection(&keys)?;

                let new_requires = transform_fields(&key_fields, &requires)?;
                let field_state_in_graph = &mut state.object_types[type_index].fields[field_index].by_graph[graph_index];
                // NOTE(port): JS `trim()` also strips U+FEFF; the printed selection set never
                // contains it.
                field_state_in_graph.requires = if new_requires.trim().is_empty() {
                    None
                } else {
                    Some(new_requires)
                };
            }

            let type_state = &state.object_types[type_index];
            let field_state = &type_state.fields[field_index];
            let field_state_in_graph = &field_state.by_graph[graph_index];
            if let Some(provides) = field_state_in_graph.provides.clone().filter(|p| !p.is_empty()) {
                // `fieldState.type.replace(/[!\[\]]+/g, "")`
                let referenced_type_name: String = field_state
                    .r#type
                    .chars()
                    .filter(|c| !matches!(c, '!' | '[' | ']'))
                    .collect();

                // `objectTypes.get(name) ?? interfaceTypes.get(name) ?? unionTypes.get(name)`
                let key_fields = if let Some(referenced_type) = state.object_types.get(&referenced_type_name) {
                    let referenced_type_in_graph = referenced_type
                        .by_graph
                        .get(&graph_id)
                        .ok_or_else(|| JsError::type_error("Cannot read properties of undefined (reading 'extension')"))?;
                    if referenced_type_in_graph.extension {
                        // removes key fields if the referenced type is an extension
                        let keys: Vec<String> =
                            referenced_type_in_graph.keys.iter().map(|key| key.fields.clone()).collect();
                        merge_key_fields_collection(&keys)?
                    } else {
                        // preserve all fields if the referenced type is a definition
                        IndexSet::new()
                    }
                } else if let Some(referenced_type) = state.interface_types.get(&referenced_type_name) {
                    let referenced_type_in_graph = referenced_type
                        .by_graph
                        .get(&graph_id)
                        .ok_or_else(|| JsError::type_error("Cannot read properties of undefined (reading 'extension')"))?;
                    if referenced_type_in_graph.extension {
                        // removes key fields if the referenced type is an extension
                        let keys: Vec<String> =
                            referenced_type_in_graph.keys.iter().map(|key| key.fields.clone()).collect();
                        merge_key_fields_collection(&keys)?
                    } else {
                        // preserve all fields if the referenced type is a definition
                        IndexSet::new()
                    }
                } else if state.union_types.contains_key(&referenced_type_name) {
                    // Nothing to transform on union types
                    continue;
                } else {
                    return Err(JsError::error(format!(
                        "Referenced type not found: {referenced_type_name}"
                    )));
                };

                let new_provides = transform_fields(&key_fields, &provides)?;
                state.object_types[type_index].fields[field_index].by_graph[graph_index].provides = Some(new_provides);
            }
        }
    }

    Ok(())
}

/// `transformFields(removableFields, fields)`. `parseFields` can throw.
fn transform_fields(removable_fields: &IndexSet<String>, fields: &str) -> JsResult<String> {
    let Some(mut parsed_fields) = parse_fields(fields)? else {
        return Ok(fields.to_string());
    };

    filter_selection_set_node(&mut parsed_fields, removable_fields, "");

    let printed = strip_ignored_characters(&print(&parsed_fields))?;

    // `printed.replace(/^\{/, "").replace(/\}$/, "")`
    let printed = printed.strip_prefix('{').unwrap_or(&printed);
    let printed = printed.strip_suffix('}').unwrap_or(printed);
    Ok(printed.to_string())
}

/// `mergeKeyFieldsCollection(fieldsCollection)`: the fields common to every key.
fn merge_key_fields_collection(fields_collection: &[String]) -> JsResult<IndexSet<String>> {
    let mut fields_sets: Vec<IndexSet<String>> = Vec::new();
    for fields in fields_collection {
        let Some(parsed_fields) = parse_fields(fields)? else {
            continue;
        };

        fields_sets.push(create_fields_set(&parsed_fields));
    }

    // merge fieldsMaps into a fieldMap that contains fields in common
    // if a field is not in common, remove it from the fieldsMap
    // if a field is in common, keep it in the fieldsMap
    let mut common_fields: Option<IndexSet<String>> = None;
    for fields_set in fields_sets {
        let Some(common) = &mut common_fields else {
            common_fields = Some(fields_set);
            continue;
        };

        // Deleting the current element while iterating a JS set: a `retain`.
        common.retain(|field| fields_set.contains(field));
    }

    Ok(common_fields.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specifications::federation::FederationVersion;
    use crate::specifications::link::{LinkImport, LinkImportKind};
    use crate::subgraph::state::{
        Argument, ArgumentKind, Ast, Directive, SubgraphFederation, SubgraphGraph, SubgraphSchema, SubgraphSpecs,
        UnionType,
    };

    fn selection_set(source: &str) -> SelectionSet {
        let doc = graphql_js::parse(source).unwrap();
        match doc.definitions.into_iter().next().unwrap() {
            Definition::OperationDefinition(op) => op.selection_set,
            _ => unreachable!(),
        }
    }

    fn subgraph(id: &str, name: &str, types: Vec<SubgraphType>) -> SubgraphState {
        SubgraphState {
            graph: SubgraphGraph {
                id: id.into(),
                name: name.into(),
                version: FederationVersion::V2_3,
                url: None,
            },
            types: types.into_iter().map(|t| (t.name().to_string(), t)).collect(),
            schema: SubgraphSchema::default(),
            links: vec![],
            specs: SubgraphSpecs::default(),
            federation: SubgraphFederation {
                version: FederationVersion::V2_3,
                imports: vec![],
            },
        }
    }

    fn directive(name: &str, locations: &[&str], arg_type: &str) -> SubgraphType {
        SubgraphType::Directive(Directive {
            name: name.into(),
            composed: false,
            repeatable: false,
            locations: locations.iter().map(|l| l.to_string()).collect(),
            args: [(
                "a".to_string(),
                Argument {
                    name: "a".into(),
                    r#type: arg_type.into(),
                    kind: ArgumentKind::Scalar,
                    inaccessible: false,
                    tags: IndexSet::new(),
                    cost: None,
                    default_value: None,
                    from_context: None,
                    description: None,
                    deprecated: None,
                    ast: Ast::default(),
                },
            )]
            .into_iter()
            .collect(),
        })
    }

    #[test]
    fn fields_set_collects_leaf_paths() {
        let set = create_fields_set(&selection_set("{ id author { id name { first } } ... on A { x } }"));
        assert_eq!(
            set.into_iter().collect::<Vec<_>>(),
            vec!["id", "author.id", "author.name.first"]
        );
    }

    #[test]
    fn filter_selection_set_removes_leaf_fields() {
        let mut set = selection_set("{ id author { id name } upc id }");
        let removable: IndexSet<String> = ["id", "author.id", "upc"].iter().map(|s| s.to_string()).collect();
        filter_selection_set_node(&mut set, &removable, "");
        // Nested paths are built as `"" + "." + "author"`, so they start with a dot and
        // `author.id` never matches: only top-level fields are removed, as in the TS.
        assert_eq!(graphql_js::print(&set), "{\n  author {\n    id\n    name\n  }\n}");
        let removable: IndexSet<String> = [".author.id"].iter().map(|s| s.to_string()).collect();
        filter_selection_set_node(&mut set, &removable, "");
        assert_eq!(graphql_js::print(&set), "{\n  author {\n    name\n  }\n}");

        // A nested selection set emptied by the filter is kept: the TS checks the parent's
        // selections, not the child's.
        let mut set = selection_set("{ author { id } x }");
        let removable: IndexSet<String> = [".author.id"].iter().map(|s| s.to_string()).collect();
        filter_selection_set_node(&mut set, &removable, "");
        match &set.selections[0] {
            Selection::Field(f) => assert!(f.selection_set.as_ref().unwrap().selections.is_empty()),
            _ => unreachable!(),
        }
        assert_eq!(set.selections.len(), 2);
    }

    #[test]
    fn add_subgraph_rejects_duplicate_ids() {
        let mut builder = create_supergraph_state_builder();
        builder.add_subgraph(Rc::new(subgraph("A", "a", vec![]))).unwrap();
        let err = builder.add_subgraph(Rc::new(subgraph("A", "other", vec![]))).unwrap_err();
        assert_eq!(err.to_js_string(), "Error: Graph with ID \"A\" already exists");
        assert_eq!(builder.get_graph("A").unwrap().graph.name, "a");
    }

    #[test]
    fn visit_subgraph_state_merges_specs_links_and_types() {
        let mut builder = create_supergraph_state_builder();
        let mut a = subgraph(
            "A",
            "a",
            vec![
                directive("exec", &["FIELD", "OBJECT"], "Int"),
                directive("schemaOnly", &["OBJECT"], "Int"),
                SubgraphType::Union(UnionType {
                    name: "_Entity".into(),
                    members: IndexSet::new(),
                    contexts: IndexSet::new(),
                    tags: IndexSet::new(),
                    inaccessible: false,
                    is_definition: true,
                    description: None,
                    ast: Ast::default(),
                }),
            ],
        );
        a.specs.tag = true;
        a.specs.cost.used = true;
        a.specs.cost.names.cost = Some("".into());
        a.specs.cost.names.list_size = Some("listSize".into());
        a.federation.imports.push(LinkImport {
            kind: LinkImportKind::Type,
            name: "_Any".into(),
            alias: Some("_Entity".into()),
        });
        a.links.push(Link {
            name: Some("link".into()),
            version: Some("v1.0".into()),
            identity: "https://specs.apollo.dev/link".into(),
            imports: vec![],
        });
        builder.visit_subgraph_state(Rc::new(a));

        let state = builder.get_supergraph_state();
        assert!(state.specs.tag);
        assert!(state.specs.cost.used);
        // `""` is falsy: the name is not taken.
        assert_eq!(state.specs.cost.names.cost, None);
        assert_eq!(state.specs.cost.names.list_size.as_deref(), Some("listSize"));
        assert_eq!(state.links.len(), 1);
        assert_eq!(state.links[0].graph, "A");
        // `_Entity` is a federation import (by alias) and is skipped.
        assert!(state.union_types.is_empty());
        // Only executable or composed directives are added.
        assert_eq!(state.directives.keys().collect::<Vec<_>>(), vec!["exec"]);
        assert!(state.directives["exec"].is_executable);
        assert!(builder.get_subgraph_state("A").is_some());
    }
}
