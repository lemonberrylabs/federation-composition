The same corpus from milestone 1, now run through three implementations: the TypeScript library, the Rust port compiled to WebAssembly and called through the JavaScript wrapper, and the Rust port compiled natively. Native Rust is faster than TypeScript on every one of the 3,567 cases. The gain is largest on small compositions and smallest on the huge schema. The WebAssembly build is faster on typical compositions but is no faster on the large schemas.

<div data-chart="speed-tiles"></div>

## Method

Unchanged from milestone 1, with two additions:

- **The fixtures are now part of the corpus.** The three benchmark fixtures (`stars-stuff`, `dgs`, `huge-schema`) are stored in the corpus together with their TypeScript output, so all three implementations read the same file. The corpus differential test now also confirms that the native Rust output for the 191-subgraph huge schema is byte-identical to the TypeScript's.
- **Two more runners use the same protocol:** three warm-up runs, then at least 5 timed runs and at least 50 ms per case, up to 200 runs, reporting the median.
  - `pnpm bench:corpus guild-wasm` times `composeServices()` from the WebAssembly wrapper. This includes everything a JavaScript caller pays: `JSON.stringify` of the input ASTs, deserializing them in Rust, composing, and `JSON.parse` of the supergraph AST that comes back.
  - `pnpm bench:native` (`crates/composition/examples/corpus_bench.rs`) times `compose_services` on ASTs that are already deserialized, as the TypeScript runner times `composeServices()` on ASTs that are already parsed.

All three runs were made one after another on an otherwise idle machine: Apple M4 Pro, Node.js v24.19.0, Rust 1.99 with `opt-level=3`, LTO and one codegen unit. Native Rust uses the system allocator; WebAssembly uses Rust's default allocator for `wasm32`. `wasm-opt` was not available, so the WebAssembly module is not post-optimized.

## Results

<div data-chart="speed-fixtures"></div>

| Fixture      | Subgraphs | TypeScript | WebAssembly | Native Rust | Native speedup |
| ------------ | --------: | ---------: | ----------: | ----------: | -------------: |
| stars-stuff  |         5 |    2.71 ms |     1.59 ms |     0.61 ms |          4.46× |
| dgs (medium) |        11 |   55.56 ms |    54.79 ms |    24.43 ms |          2.27× |
| huge-schema  |       191 |  308.85 ms |   329.80 ms |   178.28 ms |          1.73× |

<div data-chart="speedup-histogram"></div>

| Test-suite compositions (3,564) | TypeScript | WebAssembly | Native Rust |
| ------------------------------- | ---------: | ----------: | ----------: |
| Median time                     |   0.630 ms |    0.334 ms |    0.161 ms |
| p90 time                        |   1.037 ms |    0.571 ms |    0.274 ms |
| Total of medians                |   2,488 ms |    1,379 ms |      657 ms |
| Speedup, geometric mean         |          — |       1.82× |       3.84× |
| Speedup, range                  |          — | 0.70×–3.11× | 2.20×–6.73× |
| Cases slower than TypeScript    |          — |          54 |           0 |

Successful compositions gain slightly more from native Rust (4.09×) than ones that fail validation (3.57×).

## Why the large schemas gain least

I sampled the native build composing the huge schema for 15 seconds with macOS `sample`. Grouping the 12,029 samples by the function at the top of the stack:

| Where the time goes                                                                            | Share |
| ---------------------------------------------------------------------------------------------- | ----: |
| Allocating, freeing and copying memory (`malloc`, `free`, `realloc`, `memmove`, vector growth) |   52% |
| Code in the satisfiability rule's own functions                                                |   11% |
| Comparing memory (`memcmp`, mostly string comparisons)                                         |    7% |
| Hashing with the standard library's SipHash                                                    |    6% |
| Building strings (`format!`, `replace`)                                                        |    5% |

Most of the time is spent on memory management, not on composition logic. The port is line by line, so it keeps the TypeScript's habits: building string keys with template literals, copying AST nodes and strings into each new structure, and looking up many small maps. V8 makes those patterns cheap with string interning, a bump allocator for short-lived objects and fast hashing of strings. In Rust each one is a separate heap allocation, a SipHash computation or a copy. The huge schema exercises those patterns the most. That explains why its speedup (1.73×) is less than half of the typical case's (3.84×).

The WebAssembly build adds two costs on top: converting ASTs to and from JSON at the boundary, and WebAssembly's slower memory allocation. For the huge schema those costs cancel the gain entirely. For typical compositions it still comes out 1.82× faster than TypeScript.

## What this suggests

The first goal was a faithful port, and no change was made for speed. The profile shows clear next steps that don't change behavior. Each would be checked by the same three verifications (the unchanged test suite on WebAssembly, the corpus differential, and the graphql-js differential):

- Use a faster hasher than SipHash for the internal maps.
- Use a faster allocator: mimalloc for native, a better one for WebAssembly.
- Share strings and AST nodes (`Rc<str>`, borrowed references) instead of copying them.
- Avoid building temporary string keys with `format!`.
- At the WebAssembly boundary, pass SDL text when the caller has it, instead of serializing the AST to JSON.
