//! Port of `src/supergraph/validation/validation-context.ts`.

use crate::utils::collections::IndexMap;
use graphql_core::GraphQLError;

use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::{SubgraphStates, SupergraphState};
use crate::utils::js::{JsError, JsResult};

/// `SupergraphValidationContext = ReturnType<typeof createSupergraphValidationContext>`.
#[derive(Debug)]
pub struct SupergraphValidationContext {
    pub subgraph_states: SubgraphStates,
    reported_errors: Vec<GraphQLError>,
    /// `subgraphNameToIdMap` (a plain object in TS; only looked up).
    subgraph_name_to_id_map: IndexMap<String, String>,
    /// Writes to the supergraph state requested by the rules while it is visited.
    /// NOTE(port): not in the TS. `ListSizeSlicingArgumentsRule` mutates the visited field
    /// states in place (`fieldState.listSize.slicingArguments = ...`,
    /// `printRequireOneSlicingArgument = true`), which later shows in the supergraph SDL. The
    /// visitors hold shared references to the state, so the rule records the writes here and
    /// `validate_supergraph` applies them once the visit is over. No rule reads the written
    /// values in a way that differs between the two orders (see that rule).
    pending_state_updates: Vec<SupergraphStateUpdate>,
}

/// A write to the supergraph state deferred until the end of a visit (see
/// `SupergraphValidationContext::pending_state_updates`).
#[derive(Clone, Debug, PartialEq)]
pub enum SupergraphStateUpdate {
    /// `fieldState.listSize.slicingArguments = slicingArguments` on an object type field.
    ObjectFieldSlicingArguments {
        type_name: String,
        field_name: String,
        slicing_arguments: Vec<String>,
    },
    /// `fieldState.listSize.slicingArguments = slicingArguments` on an interface field.
    InterfaceFieldSlicingArguments {
        type_name: String,
        field_name: String,
        slicing_arguments: Vec<String>,
    },
    /// `fieldState.listSize.printRequireOneSlicingArgument = true` on an interface field.
    InterfaceFieldPrintRequireOneSlicingArgument { type_name: String, field_name: String },
}

impl SupergraphStateUpdate {
    /// Performs the write. The field and its `listSize` existed when the write was recorded.
    pub fn apply(self, state: &mut SupergraphState) {
        match self {
            SupergraphStateUpdate::ObjectFieldSlicingArguments {
                type_name,
                field_name,
                slicing_arguments,
            } => {
                if let Some(list_size) = state
                    .object_types
                    .get_mut(&type_name)
                    .and_then(|t| t.fields.get_mut(&field_name))
                    .and_then(|f| f.list_size.as_mut())
                {
                    list_size.slicing_arguments = Some(slicing_arguments);
                }
            }
            SupergraphStateUpdate::InterfaceFieldSlicingArguments {
                type_name,
                field_name,
                slicing_arguments,
            } => {
                if let Some(list_size) = state
                    .interface_types
                    .get_mut(&type_name)
                    .and_then(|t| t.fields.get_mut(&field_name))
                    .and_then(|f| f.list_size.as_mut())
                {
                    list_size.slicing_arguments = Some(slicing_arguments);
                }
            }
            SupergraphStateUpdate::InterfaceFieldPrintRequireOneSlicingArgument { type_name, field_name } => {
                if let Some(list_size) = state
                    .interface_types
                    .get_mut(&type_name)
                    .and_then(|t| t.fields.get_mut(&field_name))
                    .and_then(|f| f.list_size.as_mut())
                {
                    list_size.print_require_one_slicing_argument = true;
                }
            }
        }
    }
}

/// `createSupergraphValidationContext(subgraphStates)`.
pub fn create_supergraph_validation_context(subgraph_states: SubgraphStates) -> SupergraphValidationContext {
    let mut subgraph_name_to_id_map = IndexMap::default();

    for (id, state) in &subgraph_states {
        subgraph_name_to_id_map.insert(state.graph.name.clone(), id.clone());
    }

    SupergraphValidationContext {
        subgraph_states,
        reported_errors: Vec::new(),
        subgraph_name_to_id_map,
        pending_state_updates: Vec::new(),
    }
}

impl SupergraphValidationContext {
    /// `graphIdToName(id)`. Throws `Could not find subgraph with id X`.
    pub fn graph_id_to_name(&self, id: &str) -> JsResult<String> {
        self.subgraph_states
            .get(id)
            .map(|s| s.graph.name.clone())
            .ok_or_else(|| JsError::error(format!("Could not find subgraph with id {id}")))
    }

    /// `ids.map(context.graphIdToName)`: throws at the first unknown id.
    pub fn graph_ids_to_names<'i>(&self, ids: impl IntoIterator<Item = &'i String>) -> JsResult<Vec<String>> {
        ids.into_iter().map(|id| self.graph_id_to_name(id)).collect()
    }

    /// `graphNameToId(name)`:`subgraphNameToIdMap[name] ?? null`.
    /// NOTE(port): the TS looks the name up in a plain object, so a name such as
    /// `constructor` would find an `Object.prototype` member; such names are not reproduced.
    pub fn graph_name_to_id(&self, name: &str) -> Option<String> {
        self.subgraph_name_to_id_map.get(name).cloned()
    }

    pub fn report_error(&mut self, error: GraphQLError) {
        self.reported_errors.push(error);
    }

    pub fn collect_reported_errors(&mut self) -> Vec<GraphQLError> {
        std::mem::take(&mut self.reported_errors)
    }

    /// Records a write to the supergraph state, applied after the current visit.
    pub fn defer_state_update(&mut self, update: SupergraphStateUpdate) {
        self.pending_state_updates.push(update);
    }

    /// The writes recorded since the last call, in the order they were requested.
    pub fn take_state_updates(&mut self) -> Vec<SupergraphStateUpdate> {
        std::mem::take(&mut self.pending_state_updates)
    }
}

/// A post-supergraph rule: TS `(context, supergraph) => SupergraphVisitorMap`.
///
/// The struct holds the locals of the TS factory, built by `new` (the factory body before
/// `return { ... }`, which may report errors or throw), and implements the visitor callbacks.
/// `NAME` is the TS function name, matched against `__internal.disableValidationRules`.
pub trait SupergraphRule<'s>: SupergraphVisitor<'s> + Sized + 's {
    const NAME: &'static str;
    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self>;
}
