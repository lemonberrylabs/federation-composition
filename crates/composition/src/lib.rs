//! Port of `@theguild/federation-composition` (`src/`). See `crates/PORTING.md`.
//!
//! The `graphql-js` port is the `graphql_js` crate; `crate::graphql` is `src/graphql/`.
//! `src/index.ts` is this file: the re-exports below are its `export` list.

// The port keeps the TS signatures, so visitor callbacks and rule contexts carry parameters
// and fields the TS also leaves unused (about 180 such warnings without this allow).
#![allow(dead_code, unused_variables, unused_imports, unreachable_code)]

pub mod compose;
pub mod contracts;
pub mod graphql;
pub mod specifications;
pub mod subgraph;
pub mod supergraph;
pub mod types;
pub mod utils;
pub mod validate;

// `src/index.ts`
pub use compose::*;
pub use contracts::add_inaccessible_to_unreachable_types::add_inaccessible_to_unreachable_types;
pub use contracts::schema_contract::compose_schema_contract;
pub use contracts::tag_extraction::apply_tag_filter_on_subgraphs;
pub use graphql::contains_supergraph_spec::contains_supergraph_spec;
pub use graphql::sort_sdl::sort_sdl;
pub use graphql::transform_supergraph_to_public_schema::transform_supergraph_to_public_schema;
pub use types::*;
pub use utils::link::*;
pub use validate::*;
