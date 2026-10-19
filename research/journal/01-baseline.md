The goal of this project is to rewrite `@theguild/federation-composition` in Rust without changing its behavior, and to measure whether it gets faster. Before writing any Rust, this milestone records how fast the current TypeScript implementation is, how that was measured, and which GraphQL parser the Rust version will use.

<div data-chart="baseline-tiles"></div>

## Plan

1. **Baseline (this entry).** Measure the TypeScript implementation on the existing benchmark fixtures and on every composition the test suite performs.
2. **Port.** Rewrite the library in Rust. Compile it to WebAssembly and run the existing TypeScript test suite against the WebAssembly build, unchanged, until it passes.
3. **Measure.** Run the same benchmark against the WebAssembly build and a native Rust build.
4. **Port the tests.** Rewrite the test suite in Rust.

## What is being measured

The library is 30,262 lines of TypeScript in `src/`, tested by 45,188 lines of tests in 93 spec files. Its entry point, `composeServices()`, takes a list of subgraphs (each a name and a parsed GraphQL `DocumentNode`) and returns either a supergraph SDL or a list of composition errors.

The test suite is the correctness baseline: **7,343 tests pass, 25 are skipped and 8 are marked todo.** Most test cases run several times, once per Federation version from v2.0 to v2.9, and many also run against Apollo's `@apollo/composition` to check the two libraries agree. The full suite takes 11.1 s.

All numbers below come from one machine: Apple M4 Pro, Node.js v24.19.0, `NODE_ENV=production`.

## Method

### 1. The existing benchmark

The repository already has `benchmark.ts`, which uses [mitata](https://github.com/evanwashere/mitata) to compose four fixtures with both this library and Apollo's. Each iteration also runs `assertCompositionSuccess`, and mitata runs garbage collection between iterations.

<div data-chart="baseline-mitata"></div>

| Fixture      | Subgraphs |    Apollo | This library | Ratio |
| ------------ | --------: | --------: | -----------: | ----: |
| basic        |         2 |   6.33 ms |      2.12 ms |  3.0× |
| stars-stuff  |         5 |  16.67 ms |      5.08 ms |  3.3× |
| dgs (medium) |        11 | 194.39 ms |     84.27 ms |  2.3× |
| huge-schema  |       191 |    1.62 s |    391.02 ms |  4.1× |

The TypeScript implementation is already 2.3–4.1× faster than Apollo's. The Rust port has to beat this library, not Apollo.

### 2. A corpus recorded from the test suite

Four fixtures are a small sample, so I also recorded every call to `composeServices()` that the test suite makes:

- **Capture.** A Vitest setup file (`bench/capture-setup.ts`) mocks the library's `compose.js` module with a wrapper that calls the real `composeServices()` and appends its input and whether it succeeded to a JSON Lines file. The suite runs unchanged and all 7,343 tests still pass with the wrapper in place.
- **Deduplicate.** The suite made 4,260 calls. Many tests compose the same subgraphs, so calls are keyed by a SHA-1 of their input, leaving **3,564 unique compositions**: 1,888 that succeed and 1,676 that fail validation on purpose.
- **Store ASTs, not SDL.** Inputs are stored as `DocumentNode` JSON with source locations removed. Storing printed SDL did not work: two tests build an AST containing `extend type Query` with no fields, which graphql-js prints but cannot parse back.

The corpus is 13.8 MB, so it is not committed. `pnpm bench:capture` regenerates it in about 15 seconds.

### 3. Timing protocol

`bench/run-corpus.ts` replays the corpus plus three of the fixtures (`stars-stuff`, `dgs`, `huge-schema`) and times each one:

- Only `composeServices()` is timed. Inputs are already-parsed ASTs, so parsing is excluded.
- Each case first runs once to check that it still succeeds or fails as it did in the tests, then three more times as warm-up, followed by a forced garbage collection.
- Each case then runs at least 5 times and keeps running until 50 ms have passed, up to 200 runs. The **median** run is reported.

The whole replay takes 219 s. `pnpm bench:corpus` runs it.

## Results

### Fixtures

| Fixture      | Subgraphs |    SDL |    Median |       Min | Runs |
| ------------ | --------: | -----: | --------: | --------: | ---: |
| stars-stuff  |         5 |   3 KB |   2.81 ms |   2.43 ms |   17 |
| dgs (medium) |        11 | 127 KB |  58.44 ms |  54.86 ms |    5 |
| huge-schema  |       191 | 532 KB | 319.62 ms | 311.58 ms |    5 |

These are lower than the mitata numbers above because they leave out `assertCompositionSuccess` and the garbage collection between runs.

### The test-suite corpus

<div data-chart="baseline-histogram"></div>

| Group              | Compositions |   Median |    Total |
| ------------------ | -----------: | -------: | -------: |
| All                |        3,564 |  0.66 ms | 2,582 ms |
| Successful         |        1,888 |  0.75 ms | 1,634 ms |
| Failing validation |        1,676 |  0.55 ms |   948 ms |
| SDL under 1 KB     |        3,339 |  0.64 ms | 2,094 ms |
| SDL 1–3 KB         |          164 |  1.58 ms |   300 ms |
| SDL 3–10 KB        |           60 |  2.54 ms |   129 ms |
| SDL over 10 KB     |            1 | 58.33 ms |    58 ms |

Across the corpus, p90 is 1.08 ms and p99 is 2.78 ms.

### What this means for the port

- **Most test compositions are tiny.** 94% have under 1 KB of SDL and take well under a millisecond. For these, the fixed cost of calling into WebAssembly and converting the input AST may cancel out any gain. These cases measure overhead more than composition speed.
- **The large fixtures are where a speedup can show.** `dgs` and `huge-schema` spend tens to hundreds of milliseconds inside composition itself.
- **Failing compositions are cheaper.** A failing composition stops at validation, so it never reaches supergraph construction or the satisfiability check.

## Decision: port the GraphQL parser instead of using a crate

The library uses graphql-js for its AST types, `parse`, `print`, `visit` and `stripIgnoredCharacters`. Before planning the port, I checked whether an existing Rust crate could replace these. I built a corpus of 1,087 inputs, each checked against graphql-js 17.0.2:

- 202 fixture `.graphql` files
- every `graphql` template string in the tests
- every `fields:` selection set found in them
- 14 deliberately invalid selection sets

| Crate                  | Parses the valid inputs | Prints like graphql-js | Same error text | Time for all inputs |
| ---------------------- | ----------------------- | ---------------------- | --------------- | ------------------: |
| graphql-parser 0.4     | 542 of 1,068            | 352 of 542             | no              |             14.6 ms |
| apollo-compiler 1.33   | 1,068 of 1,068          | 1,025 of 1,068         | no              |             30.2 ms |
| apollo-parser 0.8      | 1,068 of 1,068          | no printer             | no              |             17.8 ms |
| async-graphql-parser 7 | 1,065 of 1,068          | no printer             | no              |             57.7 ms |
| graphql-js (reference) | —                       | —                      | —               |              6.4 ms |

None of them can be used as they are:

- **graphql-parser** cannot parse `extend schema`, which almost every Federation subgraph uses.
- **apollo-compiler** prints differently from graphql-js in four ways:
  - graphql-js wraps lists longer than 80 characters onto several lines.
  - It prints input objects as `{ name: "x" }`, where apollo-compiler prints `{name: "x"}`.
  - It keeps arguments that carry directives on one line.
  - It keeps `"""` block strings, and apollo-compiler's AST does not record which strings were block strings.
- **Error messages differ for every crate.** graphql-js reports `Syntax Error: Expected Name, found "}".` where apollo reports `expected at least one Selection in Selection Set`, and the tests assert the graphql-js wording.
- **Every crate parses slower than graphql-js.** They take 2–9× as long on this corpus, so the speedup has to come from composition itself, not from parsing.

The decision is to port graphql-js's lexer, parser, printer, visitor and `stripIgnoredCharacters` to Rust directly, at an estimated 3,000–4,000 lines. That gives the same AST shape, the same printed output and the same error messages, with no layer converting between two ASTs.

## Reproducing

```sh
pnpm install
pnpm test --run          # correctness baseline
pnpm bench               # mitata fixtures, Apollo vs this library
pnpm bench:capture       # record the corpus from the test suite
pnpm bench:corpus        # time it -> bench/results-guild-ts.json
pnpm tsx bench/summarize.ts bench/results-guild-ts.json research/data/01-baseline-guild-ts.json
```
