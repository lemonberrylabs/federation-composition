Milestone 3 showed that the faithful port spent half its time allocating and copying memory. An optimization pass that changes no behavior made native Rust 9.75× faster than TypeScript on typical compositions (was 3.84×) and 4.21× faster on the huge schema (was 1.73×). The WebAssembly build is now 5.12× faster on typical compositions and 2.58× faster on the huge schema, where it used to be no faster at all.

<div data-chart="optimized-tiles"></div>

## The rule for this pass

Every change had to keep the output identical, so it was checked by the same three verifications as the port: the unchanged TypeScript test suite on WebAssembly, the corpus differential (all 3,567 compositions byte-identical to TypeScript, including the huge schema), and the graphql-js differential. Each agent ran the corpus differential after every change, not only at the end. No test or corpus file was edited.

## How it was done

One agent made two global changes first, then four agents worked in parallel on separate areas, each in its own git worktree, and a final agent merged and measured.

1. **Hasher and allocator.** All ordered maps and sets now use the FxHash hasher instead of SipHash. Fx has no random seed, so a deliberately crafted schema could in principle force hash collisions; for a build-time tool that composes the operator's own schemas this is an acceptable trade. The library never sets a global allocator, because that is the application's choice. The benchmark can opt into mimalloc, and the WebAssembly build uses the talc allocator, 2–3% faster than Rust's default one there.
2. **Satisfiability**, the rule that dominated the huge schema's profile:
   - Graph names are interned, so visited-graph checks and queue entries compare and copy integers instead of strings.
   - The node children index is split into four maps by kind, so lookups no longer build a prefixed key such as `field__name`.
   - Debug-log strings are built only when logging is on.
   - Lazy errors take one allocation instead of two.
   - Paths and requirements are shared or moved instead of deep-copied.
   - A per-graph type list replaced a loop over every supergraph type for each of the 191 subgraphs.

   Huge schema: 167 → 97 ms in that agent's measurement.

3. **Subgraph validation**, which dominates small compositions:
   - The federation and link spec SDL is parsed once and cached. Before, it was parsed twice per subgraph.
   - The visitor calls each rule only for the node kinds it declares, as graphql-js does.
   - Rules borrow names from the document instead of copying them.

   Typical compositions became 2.27× faster from this alone.

4. **Supergraph composition and printer.** The type-state builders look up a key before allocating it. Several validation rules return early when every subgraph agrees. The printer writes into one buffer. Together these gave about 4% on the huge schema.
5. **WebAssembly boundary.**
   - The supergraph document comes back as UTF-8 bytes, and the wrapper parses them only when `supergraphDocumentNode` is read. Before, it was always converted to a JavaScript object.
   - Inputs cross as bytes instead of strings, because wasm-bindgen copies strings one character at a time.
   - `JSON.stringify` of the input no longer uses a replacer function.

   Huge schema through the wrapper: 300 → 250 ms on top of the other changes.

The four parallel branches touched disjoint files and merged without a conflict. The code change is 3,237 lines added and 1,646 removed across 121 files.

## Results

All four implementations were run one after another on the same corpus with the milestone 1 protocol. The machine was otherwise idle: the load average was under 3 at the start and about 5 at the end.

<div data-chart="optimized-fixtures"></div>

| Fixture      | Subgraphs | TypeScript | WebAssembly | Native Rust | Native + mimalloc |
| ------------ | --------: | ---------: | ----------: | ----------: | ----------------: |
| stars-stuff  |         5 |    2.68 ms |     0.57 ms |     0.30 ms |           0.22 ms |
| dgs (medium) |        11 |   60.09 ms |    22.07 ms |    12.18 ms |           9.10 ms |
| huge-schema  |       191 |  307.51 ms |   118.97 ms |    73.02 ms |          57.57 ms |

Compared with milestone 3, the native huge schema went from 178.28 to 73.02 ms (2.44× faster), and the WebAssembly huge schema went from 329.80 to 118.97 ms (2.77× faster).

<div data-chart="optimized-histogram"></div>

| Test-suite compositions (3,564) | TypeScript | WebAssembly | Native Rust | Native + mimalloc |
| ------------------------------- | ---------: | ----------: | ----------: | ----------------: |
| Median time                     |   0.633 ms |    0.116 ms |    0.065 ms |          0.048 ms |
| p90 time                        |   1.055 ms |    0.196 ms |    0.113 ms |          0.082 ms |
| Total of medians                |   2,509 ms |      487 ms |      268 ms |            197 ms |
| Speedup, geometric mean         |          — |       5.12× |       9.75× |            13.25× |
| Speedup, range                  |          — | 0.73×–6.88× | 4.68×–25.6× |       5.97×–29.1× |
| Cases slower than TypeScript    |          — |           2 |           0 |                 0 |

Milestone 3 had 54 compositions where WebAssembly was slower than TypeScript; 2 remain. Both are single-subgraph failures that take 13 µs in TypeScript and 17 µs in WebAssembly, where the fixed cost of crossing the boundary is larger than the composition.

## Where the huge schema's time goes now

The merge agent sampled the native build composing the huge schema for 15 seconds, as in milestone 3.

| Inclusive share of samples | Milestone 3 (178 ms) | Now (73 ms) |
| -------------------------- | -------------------: | ----------: |
| Supergraph validation      |                  72% |         62% |
| Satisfiability rule        |                  62% |         50% |
| Subgraph validation        |                  21% |         25% |

| Top of stack                    | Milestone 3 | Now |
| ------------------------------- | ----------: | --: |
| Allocating, freeing and copying |         52% | 45% |
| Hashing                         |          6% | 11% |
| Comparing memory                |          7% |  5% |
| Building strings                |          5% |  1% |

The satisfiability rule went from about 110 ms to about 37 ms. Allocation is still the largest category: the state types still own a copy of every type and field name for each subgraph, and removing that would mean changing the shared types to reference-counted strings. Hashing grew as a share because the other work shrank; it is mostly map lookups by string key.

## Time

The pass took 2 hours 5 minutes of wall-clock time and about 1.5 million subagent tokens: 35 minutes for the hasher and allocator, 54–79 minutes for the four parallel areas, and 11 minutes to merge and benchmark.

## Next

Phase 3: port the TypeScript test suite to Rust, so the Rust library can be tested without the WebAssembly bridge.
