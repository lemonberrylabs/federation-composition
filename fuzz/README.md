# Fuzzing

[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer) targets for the crates' public
entry points. A target fails on a panic, a stack overflow, an abort or a broken property; the
crates promise none of those on any input (see [SECURITY.md](../SECURITY.md)).

| Target         | Input                                                                                                                                                           | Checks                                                                                                                                                                                                                             |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `parse`        | any text                                                                                                                                                        | `graphql_core::parse`, `parse_value`, `parse_const_value`, `parse_type`, `parse_schema_coordinate`, `strip_ignored_characters`; a parsed document is within the nesting limit and prints to text that parses back to the same AST. |
| `build_schema` | SDL                                                                                                                                                             | `graphql_core::build_schema`, `validate_schema`, `assert_valid_schema`.                                                                                                                                                            |
| `compose`      | up to four subgraphs' SDL, separated by NUL bytes                                                                                                               | `extract_link_implementations`, `validate_subgraph` (one subgraph), `compose_services`; for a successful composition, `public_sdl`, `sort_sdl`, `transform_supergraph_to_public_schema` and `compose_schema_contract`.             |
| `compose_ast`  | the `compose` input, then `\x01` and names separated by `\x02`: the `k`-th name node of the parsed documents is renamed to the `k`-th name when it is not empty | `compose_services` on documents the parser cannot produce (names that are not GraphQL names), as documents built in code or deserialized from JavaScript can be.                                                                   |

## Running

The targets need a nightly toolchain and cargo-fuzz:

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
node fuzz/seed-corpus.mjs            # writes fuzz/corpus/<target>/
cargo +nightly fuzz run compose -- -dict=fuzz/graphql.dict -max_total_time=600
```

`seed-corpus.mjs` seeds every target's corpus with the subgraph schemas of
`crates/federation-composition/tests/fixtures` and with each group of subgraphs that a
`compose_services` call of the ported test suite composes. `graphql.dict` lists GraphQL tokens and
federation names for libFuzzer's mutations. `fuzz/corpus` and `fuzz/artifacts` are gitignored.

On macOS, libFuzzer runs with AddressSanitizer (cargo-fuzz's default) can stall in some sandboxed
environments; `--sanitizer none` avoids that. The crates have no `unsafe` code, so the sanitizer
adds little: panics, integer overflow (cargo-fuzz builds with debug assertions) and stack
overflows are caught either way.

The [Fuzz workflow](../.github/workflows/fuzz.yml) runs every target listed by `cargo fuzz list`
for two minutes a day, from the seed corpus.

## When a target finds a crash

libFuzzer writes the input to `fuzz/artifacts/<target>/crash-*`. Reproduce it with
`cargo +nightly fuzz run <target> fuzz/artifacts/<target>/crash-...`, then:

1. Compare with upstream: run the same subgraphs through upstream's TypeScript (`pnpm conformance`
   checks it out in `conformance/upstream/`). Where upstream throws, the port returns that error
   (`Err`, the same message); where upstream returns a result, the port returns the same result.
2. Fix the crate, never the target (unless the target checks a property graphql-js itself does not
   have).
3. Add the input as a regression test in the crate's normal test suite:
   `crates/graphql-core/tests/robustness.rs` or `crates/federation-composition/tests/robustness.rs`.
