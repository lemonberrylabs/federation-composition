//! Port of the TypeScript test suite (`__tests__/**/*.spec.ts`) to Rust, guild side only.
//!
//! One integration test binary: `cargo test -p federation_composition --test suite`.
//! The module tree mirrors `__tests__/` (`__tests__/supergraph/errors/FIELD_TYPE_MISMATCH.spec.ts`
//! is `supergraph/errors/field_type_mismatch.rs`). `src/specifications/link.spec.ts` is ported
//! by the unit tests in `crates/composition/src/specifications/link.rs`.
//! `shared/` is the testkit (`__tests__/shared/*.ts`, `__tests__/fixtures/*`); see the
//! "Rust test suite" section of `crates/PORTING.md`.

// Declared first, so its macros (`test_versions!`, `graphql!`, ...) are in scope in every module.
#[macro_use]
mod shared;

mod ast;
mod auth;
mod composition;
mod context;
mod contracts;
mod cost;
mod graphql;
mod interface_object_composition;
mod r#override;
mod override_with_label;
mod subgraph;
mod subgraph_validation_and_compositon_root_query;
mod supergraph;
mod supergraph_composition;
mod todo;
mod utils;
