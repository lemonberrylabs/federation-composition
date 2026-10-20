# Benchmarks

The research site's speed numbers ([research/](../research), `research/data/*.json`) come from
replaying the same corpus of compositions through each implementation. The corpus is the one the
conformance check records (`conformance/out/corpus.jsonl`: every composition upstream's test suite
performs, plus the `stars-stuff`, `dgs` and `huge-schema` fixtures), so run `pnpm conformance`
first: it also checks out upstream into `conformance/upstream` and builds the npm package.

```sh
pnpm conformance                                  # upstream checkout, npm package, corpus
pnpm bench:corpus guild-ts                        # upstream's TypeScript -> bench/results-guild-ts.json
pnpm bench:corpus guild-wasm                      # the npm package       -> bench/results-guild-wasm.json
pnpm bench:native                                 # native Rust           -> bench/results-guild-rust.json
pnpm bench:native:mimalloc                        # native Rust, mimalloc -> bench/results-guild-rust-mimalloc.json
pnpm bench:summarize bench/results-guild-ts.json research/data/<entry>-guild-ts.json
bench/profile.sh [case-id] [seconds]              # where native time goes (macOS `sample`)
```

| File                                                                                                                                                    | What it does                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`run-corpus.ts`](run-corpus.ts)                                                                                                                        | Times `composeServices()` per corpus case: `guild-ts` imports upstream's `src/compose.ts` from the checkout, `guild-wasm` the built npm package (`npm/dist`). Runs with upstream's `tsx`. |
| [`crates/federation-composition-rs/examples/corpus_bench.rs`](../crates/federation-composition-rs/examples/corpus_bench.rs)                             | The same protocol and output format for native Rust.                                                                                                                                      |
| [`summarize.ts`](summarize.ts)                                                                                                                          | Compacts a results file into the data the research site charts.                                                                                                                           |
| [`profile.sh`](profile.sh), [`profile_report.py`](profile_report.py), [`profile_case.rs`](../crates/federation-composition-rs/examples/profile_case.rs) | Profile one corpus case natively.                                                                                                                                                         |

`CONFORMANCE_CORPUS` points all of them at another corpus file.

## What moved when the TypeScript implementation was removed

- `bench/capture-setup.ts`, `bench/vitest.capture.config.js` and `bench/build-corpus.ts` are now
  [`conformance/capture-setup.ts`](../conformance/capture-setup.ts),
  [`conformance/vitest.capture.config.mjs`](../conformance/vitest.capture.config.mjs) and
  [`conformance/build-corpus.ts`](../conformance/build-corpus.ts), and run against the upstream
  checkout. `pnpm bench:capture` is `pnpm conformance --only=capture`, and the corpus moved from
  `bench/corpus.jsonl` to `conformance/out/corpus.jsonl`.
- `run-corpus.ts` reads upstream's TypeScript from `conformance/upstream/src` instead of `src/`,
  and the WebAssembly build from the npm package (`npm/dist`) instead of the `wasm/` shim.
- `benchmark.ts` (the [mitata](https://github.com/evanwashere/mitata) comparison of this library
  with `@apollo/composition` on the fixtures, `pnpm bench`, the source of
  `research/data/01-baseline-mitata.json`) and `compose.ts` are upstream's files. They are no
  longer in this repository; run them in the checkout: `cd conformance/upstream && pnpm bench`.
