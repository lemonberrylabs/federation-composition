//! Port of `src/supergraph/composition/common.ts`.

use crate::utils::collections::IndexMap;
use graphql_rs::{ConstDirective, Definition, Directive as DirectiveNode};

use crate::supergraph::state::{SubgraphStates, SupergraphState};
use crate::utils::js::JsResult;

/// `MapByGraph<T> = Map<string, T>` keyed by graph id.
pub type MapByGraph<T> = IndexMap<String, T>;

/// `Key`: same shape as the subgraph state's key.
pub use crate::subgraph::state::Key;

/// `Graph`: `{ name, id, version, url? }`, the subgraph state's `graph`.
pub type Graph = crate::subgraph::state::SubgraphGraph;

/// The `helpers` argument of `composeSupergraphState`/`composeSupergraphNode`:
/// `{ graphNameToId(graphName), supergraphState }`.
#[derive(Clone, Copy)]
pub struct ComposeHelpers<'s> {
    pub supergraph_state: &'s SupergraphState,
    /// `graphNameToIdMap` of the supergraph state builder.
    pub graph_name_to_id_map: &'s IndexMap<String, String>,
}

impl ComposeHelpers<'_> {
    /// `graphNameToId(graphName)`: `graphNameToIdMap[graphName] ?? null`.
    /// NOTE(port): a plain object in TS; graph names are user input and could be integer-like,
    /// but it is only looked up, never iterated.
    pub fn graph_name_to_id(&self, graph_name: &str) -> Option<String> {
        self.graph_name_to_id_map.get(graph_name).cloned()
    }
}

/// `TypeBuilder<T, S>`: a builder merging subgraph types `T` into supergraph type states `S`.
/// `composeSupergraphState?` is an inherent method of the two builders that have it
/// (objects and interfaces), with the parts of the supergraph state it reads passed
/// explicitly, because the type being composed lives inside the supergraph state.
pub trait TypeBuilder<T, S> {
    fn visit_subgraph_state(&mut self, graph: &Graph, state: &mut IndexMap<String, S>, type_name: &str, ty: &T);

    /// Returns a `TypeDefinitionNode | DirectiveDefinitionNode`. Fallible because the TS
    /// bodies call `ensureValue` and can throw.
    fn compose_supergraph_node(
        &mut self,
        ty: &S,
        graphs: &SubgraphStates,
        helpers: ComposeHelpers<'_>,
    ) -> JsResult<Definition>;
}

/// The `getOrCreate*` helpers' `map.get(key) ?? map.set(key, create()).get(key)`.
/// PERF: looks the key up before allocating it; `entry(key.to_string())` allocated a `String`
/// on every visit, and the key is already present for every subgraph after the first. A new
/// entry is appended, as with `entry`.
pub fn get_or_insert_with<'m, V>(map: &'m mut IndexMap<String, V>, key: &str, create: impl FnOnce() -> V) -> &'m mut V {
    let index = match map.get_index_of(key) {
        Some(index) => index,
        None => map.insert_full(key.to_string(), create()).0,
    };
    &mut map[index]
}

/// `convertToConst(nodes)`: a type cast in TS; directives are already const here.
pub fn convert_to_const(nodes: &[DirectiveNode]) -> Vec<ConstDirective> {
    nodes.to_vec()
}
