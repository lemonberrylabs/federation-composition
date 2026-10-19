The whole library now exists in Rust. Run against a WebAssembly build of that Rust, the unchanged TypeScript test suite passes: 7,343 tests, the same count as the TypeScript implementation. Separately, the native Rust build produces byte-identical output to the TypeScript on all 3,564 compositions recorded from the test suite.

<div data-chart="port-tiles"></div>

## What was built

| Part                      | What it is                                                                                                   |      Lines of Rust |
| ------------------------- | ------------------------------------------------------------------------------------------------------------ | -----------------: |
| `crates/graphql`          | A port of graphql-js 17.0.2's language layer: AST, lexer, parser, printer, visitor, `stripIgnoredCharacters` |              5,308 |
| `crates/composition`      | A port of `src/`, one Rust module per TypeScript file                                                        |             40,613 |
| `crates/composition-wasm` | wasm-bindgen bindings for JavaScript users                                                                   |              1,610 |
| `wasm/`                   | A TypeScript wrapper exposing the same modules and exports as `src/`, backed by the WebAssembly build        | 1,146 (TypeScript) |

The TypeScript library is 30,262 lines, so the Rust is about 1.34 times longer. The Rust module tree mirrors `src/` exactly: `src/supergraph/composition/object-type.ts` became `crates/composition/src/supergraph/composition/object_type.rs`, with the same functions in the same order. A reader can compare the two side by side.

The long-term product is the Rust library. The WebAssembly build has two jobs: it lets existing JavaScript users migrate, and it lets the existing tests check the port without being rewritten.

## How the port was done

The work was split so that many agents could work at once without editing the same files.

1. **The GraphQL layer first.** The library uses graphql-js for its AST, parsing, printing and visiting, and uses graphql-js's type system in only one file. One agent ported graphql-js's language layer and checked it against graphql-js on 6,518 documents. It compared parse results, error messages, printed output, `stripIgnoredCharacters` output, the AST as JSON, and the order in which the visitor reaches each node. Every comparison matches. In that agent's measurement the Rust parser handled its 1,138-document corpus in 3.9 ms, against 9.35 ms for graphql-js.
2. **Porting rules.** `crates/PORTING.md` lists the JavaScript behaviors a line-by-line port can silently break:
   - `Map` and `Set` keep insertion order, so they became `IndexMap`/`IndexSet`, and deletions must not reorder the rest.
   - `localeCompare` sorts by ICU collation, not by bytes. It was reimplemented and checked against Node on 26,047 string pairs.
   - `String(number)` has its own formatting rules.
   - A `try/catch` also catches runtime `TypeError`s.
   - Truthiness differs for `""` and `0`.
   - Plain-object dictionaries put integer-like keys first.
3. **An outline of the crate.** One agent wrote every type and every function signature in the crate, with empty bodies, and decided how shared mutable state maps to Rust. The satisfiability graph became an arena with typed indexes, and the validation contexts became the shared context passed to visitors. It compiled to a crate of 653 empty function bodies.
4. **Nine porters in parallel.** Each agent took one area of `src/` in its own git worktree and filled in every function body, compiling against the outline. One merge agent then combined the nine branches. There was a single conflict, two porters adding the same field, and no fixes were needed where the areas meet.
5. **Bindings.** Another agent built the WebAssembly bindings and the wrapper, plus a Vitest config that sends every test import of `src/...` to the wrapper. If a module has no wrapper, the run fails rather than quietly using the TypeScript.

## Verification

Three independent checks:

- **The existing test suite, unchanged, against WebAssembly.** `pnpm test:wasm` passes 7,343 tests plus 7 tests of the bindings themselves. To prove the TypeScript is never loaded, the suite was run with every non-test file in `src/` replaced by `throw new Error(...)`, and it still passed. No test file differs from `main`.
- **Native Rust against recorded TypeScript output.** The corpus capture from milestone 1 now also stores the TypeScript result of each composition: supergraph SDL, public SDL, or each error's message and code. A Rust integration test replays all 3,564 compositions and requires an exact match. All 3,564 match, including 2,452 errors with 80 different error codes. As a control, changing one character in one stored output makes the test report exactly that one mismatch.
- **graphql-js against the Rust GraphQL layer**, on 6,518 documents, as described above.

## Known differences not covered by tests

Some behaviors are approximated where the tests have no coverage and the inputs are unlikely in practice. Each is marked `NOTE(port)` in the code:

- WHATWG URL parsing of `@link` URLs (a port of the part of `new URL()` the library uses) only lowercases non-ASCII hostnames before Punycode encoding them, instead of applying full UTS #46, and gives no special treatment to Windows drive letters in `file:` URLs.
- The JSON5 parser used for `@link(import:)` decides which non-ASCII characters can appear in identifiers using Rust's Unicode classes instead of JSON5's own tables.
- The TypeScript looks up subgraph names in plain objects, so a subgraph named after a property of `Object.prototype`, such as `constructor`, finds that property. The Rust does not reproduce this.
- `getReachableTypes` returns the same set of type names but in document order, where graphql-js uses its schema's type-map order.

## Problems hit along the way

- serde_json's default float parsing is not correctly rounded, and `String(1.2345678901234568e20)` came out one digit wrong. Enabling `float_roundtrip` fixed it.
- On this Mac, rustup's `rust-lld` cannot find `libLLVM.dylib` when linking a WebAssembly `cdylib`. The build script sets `DYLD_FALLBACK_LIBRARY_PATH` to the toolchain's `lib` directory.
- The workflow tool created each porter's git worktree from `main` instead of from the working branch. The porters noticed the outline was missing and reset their branches onto it before writing anything.

## Time

Phase 1 took about 2 hours of wall-clock time: 11:30 to 13:36 on 2026-10-19.

| Step                                             | Time           |
| ------------------------------------------------ | -------------- |
| graphql-js port, while the bindings were planned | 45 min         |
| Crate outline: types and signatures              | 38 min         |
| Nine porters in parallel                         | 12–28 min each |
| Merge, wiring the bindings, verification         | 25 min         |

The workflow (outline, nine porters and merge) used about 3.2 million subagent tokens.

## Next

Phase 2 measures speed: the same benchmark as milestone 1, run against the TypeScript, the WebAssembly build including the cost of converting the AST, and native Rust.
