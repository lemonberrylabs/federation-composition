//! Port of `src/utils/dependency-graph.ts` (copied in TS from github:kamilkisiela/dependency-graph).
//!
//! NOTE(port): nothing in `src/` uses `DepGraph`; it is ported for completeness.
//! NOTE(port): the TS keeps nodes and edges in plain objects iterated with `Object.keys`, so
//! integer-like node names would come first in numeric order. Port with `IndexMap` and keep
//! that ordering rule in mind if node names can be integer-like.

use indexmap::IndexMap;

use crate::utils::js::{JsError, JsResult};

/// `Options`.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub circular: Option<bool>,
}

/// The node data: the TS stores either the given data or, when `addNode` is called with one
/// argument, the node name itself.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeData<T> {
    Data(Option<T>),
    Name(String),
}

/// `createDFS(edges, leavesOnly, result, circular)` returns a closure keeping `visited` across
/// calls. Ported as a struct; `run(start)` is the returned function.
struct Dfs<'g> {
    edges: &'g IndexMap<String, Vec<String>>,
    leaves_only: bool,
    circular: bool,
    visited: IndexMap<String, bool>,
}

fn create_dfs<'g>(
    edges: &'g IndexMap<String, Vec<String>>,
    leaves_only: Option<bool>,
    circular: bool,
) -> Dfs<'g> {
    Dfs {
        edges,
        leaves_only: leaves_only.unwrap_or(false),
        circular,
        visited: IndexMap::new(),
    }
}

impl Dfs<'_> {
    /// The function returned by `createDFS`; pushes into `result`. Throws `DepGraphCycleError`.
    fn run(&mut self, start: &str, result: &mut Vec<String>) -> Result<(), DepGraphCycleError> {
        todo!()
    }
}

/// `DepGraph<T>`.
#[derive(Clone, Debug)]
pub struct DepGraph<T> {
    nodes: IndexMap<String, NodeData<T>>,
    outgoing_edges: IndexMap<String, Vec<String>>,
    incoming_edges: IndexMap<String, Vec<String>>,
    circular: bool,
}

impl<T: Clone> DepGraph<T> {
    pub fn new(opts: Option<Options>) -> Self {
        DepGraph {
            nodes: IndexMap::new(),
            outgoing_edges: IndexMap::new(),
            incoming_edges: IndexMap::new(),
            circular: opts.and_then(|o| o.circular).unwrap_or(false),
        }
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// `addNode(name, data?)`. `data` is `Some` when the TS call passes a second argument
    /// (even `undefined`).
    pub fn add_node(&mut self, name: &str, data: Option<Option<T>>) {
        todo!()
    }

    pub fn remove_node(&mut self, name: &str) {
        todo!()
    }

    pub fn has_node(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    pub fn get_node_data(&self, name: &str) -> JsResult<&NodeData<T>> {
        todo!()
    }

    pub fn set_node_data(&mut self, name: &str, data: Option<T>) -> JsResult<()> {
        todo!()
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) -> JsResult<bool> {
        todo!()
    }

    pub fn remove_dependency(&mut self, from: &str, to: &str) {
        todo!()
    }

    pub fn direct_dependencies_of(&self, name: &str) -> JsResult<Vec<String>> {
        todo!()
    }

    pub fn direct_dependants_of(&self, name: &str) -> JsResult<Vec<String>> {
        todo!()
    }

    pub fn dependencies_of(&self, name: &str, leaves_only: Option<bool>) -> JsResult<Vec<String>> {
        todo!()
    }

    pub fn dependants_of(&self, name: &str, leaves_only: Option<bool>) -> JsResult<Vec<String>> {
        todo!()
    }

    pub fn overall_order(&self, leaves_only: Option<bool>) -> JsResult<Vec<String>> {
        todo!()
    }

    pub fn entry_nodes(&self) -> Vec<String> {
        todo!()
    }

    /// Alias of `direct_dependants_of`.
    pub fn direct_dependents_of(&self, name: &str) -> JsResult<Vec<String>> {
        self.direct_dependants_of(name)
    }

    /// Alias of `dependants_of`.
    pub fn dependents_of(&self, name: &str, leaves_only: Option<bool>) -> JsResult<Vec<String>> {
        self.dependants_of(name, leaves_only)
    }
}

/// `DepGraphCycleError`: message `"Dependency Cycle Found: " + cyclePath.join(" -> ")`.
#[derive(Clone, Debug, PartialEq)]
pub struct DepGraphCycleError {
    pub cycle_path: Vec<String>,
}

impl DepGraphCycleError {
    pub fn new(cycle_path: Vec<String>) -> Self {
        DepGraphCycleError { cycle_path }
    }

    pub fn message(&self) -> String {
        format!("Dependency Cycle Found: {}", self.cycle_path.join(" -> "))
    }
}

impl From<DepGraphCycleError> for JsError {
    fn from(e: DepGraphCycleError) -> Self {
        JsError::error(e.message())
    }
}
