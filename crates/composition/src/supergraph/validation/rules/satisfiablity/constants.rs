//! Port of `src/supergraph/validation/rules/satisfiablity/constants.ts`.
//!
//! The TS symbols `SUPERGRAPH_ID` and `MERGEDGRAPH_ID` identify the two non-subgraph graphs.
//! They are variants of [`GraphId`].

/// The `id: string | Symbol` of a satisfiability `Graph`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GraphId {
    /// A subgraph's id (`Symbol.for(id)` in TS).
    Subgraph(String),
    /// `SUPERGRAPH_ID = Symbol("__supergraph__")`.
    Supergraph,
    /// `MERGEDGRAPH_ID = Symbol("__mergedgraph__")`.
    MergedGraph,
}

impl GraphId {
    /// `this.id`: the subgraph id, or `Symbol(...).toString()` for the two symbols.
    pub fn to_id_string(&self) -> String {
        match self {
            GraphId::Subgraph(id) => id.clone(),
            GraphId::Supergraph => "Symbol(__supergraph__)".to_string(),
            GraphId::MergedGraph => "Symbol(__mergedgraph__)".to_string(),
        }
    }
}
