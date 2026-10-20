//! Port of `src/utils/dependency-graph.ts` (copied in TS from github:kamilkisiela/dependency-graph).
//!
//! NOTE(port): nothing in `src/` uses `DepGraph`; it is ported for completeness.
//! NOTE(port): the TS keeps nodes and edges in plain objects iterated with `Object.keys`, so
//! integer-like node names come first in numeric order. `object_keys` reproduces that order
//! where the TS iterates the keys (`overallOrder`, `entryNodes`).

use crate::utils::collections::IndexMap;

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

fn create_dfs<'g>(edges: &'g IndexMap<String, Vec<String>>, leaves_only: Option<bool>, circular: bool) -> Dfs<'g> {
    Dfs {
        edges,
        leaves_only: leaves_only.unwrap_or(false),
        circular,
        visited: IndexMap::default(),
    }
}

impl Dfs<'_> {
    /// The function returned by `createDFS`; pushes into `result`. Throws `DepGraphCycleError`.
    fn run(&mut self, start: &str, result: &mut Vec<String>) -> Result<(), DepGraphCycleError> {
        if self.visited.get(start).copied().unwrap_or(false) {
            return Ok(());
        }
        let mut in_current_path: IndexMap<String, bool> = IndexMap::default();
        let mut current_path: Vec<String> = Vec::new();
        // used as a stack: (node, processed)
        let mut todo: Vec<(String, bool)> = Vec::new();
        todo.push((start.to_string(), false));
        while !todo.is_empty() {
            // peek at the todo stack
            let last = todo.len() - 1;
            let processed = todo[last].1;
            let node = todo[last].0.clone();
            if !processed {
                // Haven't visited edges yet (visiting phase)
                if self.visited.get(&node).copied().unwrap_or(false) {
                    todo.pop();
                    continue;
                } else if in_current_path.get(&node).copied().unwrap_or(false) {
                    // It's not a DAG
                    if self.circular {
                        todo.pop();
                        // If we're tolerating cycles, don't revisit the node
                        continue;
                    }
                    current_path.push(node);
                    return Err(DepGraphCycleError::new(current_path));
                }

                in_current_path.insert(node.clone(), true);
                current_path.push(node.clone());
                // NOTE(port): every node reachable through an edge has an edge list (removeNode
                // removes the node from all lists); the TS would throw a TypeError otherwise.
                let node_edges = self.edges.get(&node).map(Vec::as_slice).unwrap_or(&[]);
                // (push edges onto the todo stack in reverse order to be order-compatible with the old DFS implementation)
                for edge in node_edges.iter().rev() {
                    todo.push((edge.clone(), false));
                }
                todo[last].1 = true;
            } else {
                // Have visited edges (stack unrolling phase)
                todo.pop();
                current_path.pop();
                in_current_path.insert(node.clone(), false);
                self.visited.insert(node.clone(), true);
                if !self.leaves_only || self.edges.get(&node).is_none_or(|e| e.is_empty()) {
                    result.push(node);
                }
            }
        }
        Ok(())
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

fn node_does_not_exist(name: &str) -> JsError {
    JsError::error(format!("Node does not exist: {name}"))
}

impl<T: Clone> DepGraph<T> {
    pub fn new(opts: Option<Options>) -> Self {
        DepGraph {
            nodes: IndexMap::default(),
            outgoing_edges: IndexMap::default(),
            incoming_edges: IndexMap::default(),
            circular: opts.and_then(|o| o.circular).unwrap_or(false),
        }
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// `addNode(name, data?)`. `data` is `Some` when the TS call passes a second argument
    /// (even `undefined`).
    pub fn add_node(&mut self, name: &str, data: Option<Option<T>>) {
        if !self.has_node(name) {
            // Checking the arguments length allows the user to add a node with undefined data
            match data {
                Some(data) => {
                    self.nodes.insert(name.to_string(), NodeData::Data(data));
                }
                None => {
                    self.nodes.insert(name.to_string(), NodeData::Name(name.to_string()));
                }
            }
            self.outgoing_edges.insert(name.to_string(), Vec::new());
            self.incoming_edges.insert(name.to_string(), Vec::new());
        }
    }

    pub fn remove_node(&mut self, name: &str) {
        if self.has_node(name) {
            self.nodes.shift_remove(name);
            self.outgoing_edges.shift_remove(name);
            self.incoming_edges.shift_remove(name);
            for edge_list in [&mut self.incoming_edges, &mut self.outgoing_edges] {
                for edges in edge_list.values_mut() {
                    if let Some(idx) = edges.iter().position(|e| e == name) {
                        edges.remove(idx);
                    }
                }
            }
        }
    }

    pub fn has_node(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    pub fn get_node_data(&self, name: &str) -> JsResult<&NodeData<T>> {
        match self.nodes.get(name) {
            Some(data) => Ok(data),
            None => Err(node_does_not_exist(name)),
        }
    }

    pub fn set_node_data(&mut self, name: &str, data: Option<T>) -> JsResult<()> {
        match self.nodes.get_mut(name) {
            Some(slot) => {
                *slot = NodeData::Data(data);
                Ok(())
            }
            None => Err(node_does_not_exist(name)),
        }
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) -> JsResult<bool> {
        if !self.has_node(from) {
            return Err(node_does_not_exist(from));
        }
        if !self.has_node(to) {
            return Err(node_does_not_exist(to));
        }
        if let Some(outgoing) = self.outgoing_edges.get_mut(from) {
            if !outgoing.iter().any(|e| e == to) {
                outgoing.push(to.to_string());
            }
        }
        if let Some(incoming) = self.incoming_edges.get_mut(to) {
            if !incoming.iter().any(|e| e == from) {
                incoming.push(from.to_string());
            }
        }
        Ok(true)
    }

    pub fn remove_dependency(&mut self, from: &str, to: &str) {
        if self.has_node(from) {
            if let Some(outgoing) = self.outgoing_edges.get_mut(from) {
                if let Some(idx) = outgoing.iter().position(|e| e == to) {
                    outgoing.remove(idx);
                }
            }
        }

        if self.has_node(to) {
            if let Some(incoming) = self.incoming_edges.get_mut(to) {
                if let Some(idx) = incoming.iter().position(|e| e == from) {
                    incoming.remove(idx);
                }
            }
        }
    }

    pub fn direct_dependencies_of(&self, name: &str) -> JsResult<Vec<String>> {
        if self.has_node(name) {
            Ok(self.outgoing_edges.get(name).cloned().unwrap_or_default())
        } else {
            Err(node_does_not_exist(name))
        }
    }

    pub fn direct_dependants_of(&self, name: &str) -> JsResult<Vec<String>> {
        if self.has_node(name) {
            Ok(self.incoming_edges.get(name).cloned().unwrap_or_default())
        } else {
            Err(node_does_not_exist(name))
        }
    }

    pub fn dependencies_of(&self, name: &str, leaves_only: Option<bool>) -> JsResult<Vec<String>> {
        if self.has_node(name) {
            let mut result: Vec<String> = Vec::new();
            let mut dfs = create_dfs(&self.outgoing_edges, leaves_only, self.circular);
            dfs.run(name, &mut result)?;
            if let Some(idx) = result.iter().position(|n| n == name) {
                result.remove(idx);
            }
            Ok(result)
        } else {
            Err(node_does_not_exist(name))
        }
    }

    pub fn dependants_of(&self, name: &str, leaves_only: Option<bool>) -> JsResult<Vec<String>> {
        if self.has_node(name) {
            let mut result: Vec<String> = Vec::new();
            let mut dfs = create_dfs(&self.incoming_edges, leaves_only, self.circular);
            dfs.run(name, &mut result)?;
            if let Some(idx) = result.iter().position(|n| n == name) {
                result.remove(idx);
            }
            Ok(result)
        } else {
            Err(node_does_not_exist(name))
        }
    }

    pub fn overall_order(&self, leaves_only: Option<bool>) -> JsResult<Vec<String>> {
        let mut result: Vec<String> = Vec::new();
        let keys = object_keys(&self.nodes);
        if keys.is_empty() {
            // Empty graph
            return Ok(result);
        }
        if !self.circular {
            // Look for cycles - we run the DFS starting at all the nodes in case there
            // are several disconnected subgraphs inside this dependency graph.
            let mut cycle_dfs = create_dfs(&self.outgoing_edges, Some(false), self.circular);
            let mut ignored: Vec<String> = Vec::new();
            for n in &keys {
                cycle_dfs.run(n, &mut ignored)?;
            }
        }

        let mut dfs = create_dfs(&self.outgoing_edges, leaves_only, self.circular);
        // Find all potential starting points (nodes with nothing depending on them) an
        // run a DFS starting at these points to get the order
        let starts: Vec<&String> = keys.iter().filter(|node| self.has_no_incoming_edges(node)).collect();
        for n in starts {
            dfs.run(n, &mut result)?;
        }

        // If we're allowing cycles - we need to run the DFS against any remaining
        // nodes that did not end up in the initial result (as they are part of a
        // subgraph that does not have a clear starting point)
        if self.circular {
            // `keys.filter(...)` runs to completion before the `forEach`.
            let remaining: Vec<&String> = keys.iter().filter(|node| !result.contains(node)).collect();
            for n in remaining {
                dfs.run(n, &mut result)?;
            }
        }

        Ok(result)
    }

    pub fn entry_nodes(&self) -> Vec<String> {
        object_keys(&self.nodes)
            .into_iter()
            .filter(|node| self.has_no_incoming_edges(node))
            .collect()
    }

    /// `this.incomingEdges[node].length === 0`.
    fn has_no_incoming_edges(&self, node: &str) -> bool {
        self.incoming_edges.get(node).is_none_or(|e| e.is_empty())
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

/// `Object.keys(object)`: integer-like keys (array indexes) first in ascending numeric order,
/// then the other keys in insertion order.
fn object_keys<V>(object: &IndexMap<String, V>) -> Vec<String> {
    fn array_index(key: &str) -> Option<u32> {
        if key.is_empty() || (key.len() > 1 && key.starts_with('0')) || !key.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // Array indexes are below 2^32 - 1.
        key.parse::<u32>().ok().filter(|n| *n != u32::MAX)
    }
    let mut indexes: Vec<(u32, &String)> = object.keys().filter_map(|k| array_index(k).map(|n| (n, k))).collect();
    indexes.sort_by_key(|(n, _)| *n);
    indexes
        .into_iter()
        .map(|(_, k)| k.clone())
        .chain(object.keys().filter(|k| array_index(k).is_none()).cloned())
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(circular: bool) -> DepGraph<()> {
        DepGraph::new(Some(Options {
            circular: Some(circular),
        }))
    }

    fn chain(g: &mut DepGraph<()>) {
        for n in ["a", "b", "c"] {
            g.add_node(n, None);
        }
        g.add_dependency("a", "b").unwrap();
        g.add_dependency("b", "c").unwrap();
        g.add_dependency("c", "a").unwrap();
    }

    #[test]
    fn overall_order_and_dependencies() {
        let mut g = graph(false);
        for n in ["a", "b", "c", "d"] {
            g.add_node(n, None);
        }
        g.add_dependency("a", "b").unwrap();
        g.add_dependency("b", "c").unwrap();
        g.add_dependency("a", "d").unwrap();
        assert_eq!(g.overall_order(None).unwrap(), ["c", "b", "d", "a"]);
        assert_eq!(g.overall_order(Some(true)).unwrap(), ["c", "d"]);
        assert_eq!(g.dependencies_of("a", None).unwrap(), ["c", "b", "d"]);
        assert_eq!(g.dependants_of("c", None).unwrap(), ["a", "b"]);
        assert_eq!(g.entry_nodes(), ["a"]);
        assert_eq!(g.get_node_data("a").unwrap(), &NodeData::Name("a".to_string()));
        assert_eq!(g.get_node_data("x").unwrap_err().message(), "Node does not exist: x");
        g.remove_node("b");
        assert_eq!(g.direct_dependencies_of("a").unwrap(), ["d"]);
    }

    #[test]
    fn cycle_error() {
        let mut g = graph(false);
        chain(&mut g);
        assert_eq!(
            g.overall_order(None).unwrap_err().message(),
            "Dependency Cycle Found: a -> b -> c -> a"
        );
        let mut g = graph(true);
        chain(&mut g);
        assert_eq!(g.overall_order(None).unwrap(), ["c", "b", "a"]);
    }

    #[test]
    fn integer_like_keys_come_first() {
        let mut g = graph(false);
        for n in ["b", "10", "2", "a", "01"] {
            g.add_node(n, None);
        }
        assert_eq!(g.entry_nodes(), ["2", "10", "b", "a", "01"]);
    }
}
