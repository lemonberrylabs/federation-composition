//! Port of `@theguild/federation-composition` (`src/`). See `crates/PORTING.md`.
//!
//! The `graphql-js` port is the `graphql_js` crate; `crate::graphql` is `src/graphql/`.

pub mod compose;
pub mod contracts;
pub mod graphql;
pub mod specifications;
pub mod subgraph;
pub mod supergraph;
pub mod types;
pub mod utils;
pub mod validate;
