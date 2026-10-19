//! Port of `src/supergraph/validation/validation-context.ts`.

use graphql_js::GraphQLError;
use indexmap::IndexMap;

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
}

/// `createSupergraphValidationContext(subgraphStates)`.
pub fn create_supergraph_validation_context(subgraph_states: SubgraphStates) -> SupergraphValidationContext {
    todo!()
}

impl SupergraphValidationContext {
    /// `graphIdToName(id)`. Throws `Could not find subgraph with id X`.
    pub fn graph_id_to_name(&self, id: &str) -> JsResult<String> {
        self.subgraph_states
            .get(id)
            .map(|s| s.graph.name.clone())
            .ok_or_else(|| JsError::error(format!("Could not find subgraph with id {id}")))
    }

    /// `graphNameToId(name)`.
    pub fn graph_name_to_id(&self, name: &str) -> Option<String> {
        self.subgraph_name_to_id_map.get(name).cloned()
    }

    pub fn report_error(&mut self, error: GraphQLError) {
        self.reported_errors.push(error);
    }

    pub fn collect_reported_errors(&mut self) -> Vec<GraphQLError> {
        std::mem::take(&mut self.reported_errors)
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
