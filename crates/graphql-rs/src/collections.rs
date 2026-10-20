//! Insertion-ordered maps and sets with a fast deterministic hasher.
//!
//! Use these instead of `indexmap::IndexMap`/`IndexSet` (whose default hasher is SipHash with
//! a random seed) and instead of `std::collections::HashMap`/`HashSet` (whose iteration order
//! is unspecified, while JavaScript `Map`/`Set` iterate in insertion order). The hasher does
//! not affect iteration order: an `IndexMap` iterates in insertion order whatever it hashes
//! with. Construct them with `default()`, `with_capacity_and_hasher(n, Default::default())`,
//! `from_iter` or `collect()`; `new()`, `with_capacity()` and `from([..])` exist only for
//! the default hasher.

/// FxHash: a few multiply/rotate operations per word, no random seed (no `getrandom` on wasm32).
pub type FxBuildHasher = rustc_hash::FxBuildHasher;
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, FxBuildHasher>;
pub type IndexSet<T> = indexmap::IndexSet<T, FxBuildHasher>;
