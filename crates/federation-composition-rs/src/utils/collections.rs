//! The crate's `IndexMap`/`IndexSet`: insertion-ordered, with a fast deterministic hasher.
//! See `graphql_rs::collections` (the same types, so maps cross the crate boundary as is).
//! Never use `indexmap::IndexMap` directly or a std `HashMap`/`HashSet`.

pub use graphql_rs::collections::{IndexMap, IndexSet};
