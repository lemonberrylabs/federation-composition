The repository is now a Rust project. The TypeScript implementation is gone. The Rust tracks upstream through a conformance check that runs upstream's unchanged test suite on the WebAssembly build. Both library crates have a deliberate public API. Public entry points return errors instead of panicking, including on deeply nested input. Continuous integration, release and fuzzing workflows are written but have not run on GitHub yet.

## The crates

| Crate                         | What it is                                                         |
| ----------------------------- | ------------------------------------------------------------------ |
| `graphql-rs`                  | the port of graphql-js 17.0.2's language layer and schema building |
| `federation-composition-rs`   | the port of `@theguild/federation-composition`                     |
| `federation-composition-wasm` | the WebAssembly bindings behind the npm package of the same name   |

All three are MIT licensed and credit The Guild and the GraphQL Contributors. Nothing is published yet.

## Removing the TypeScript, keeping upstream as the reference

Deleting `src/` and `__tests__/` removed 80,793 lines. Upstream is still the specification, so it is now checked out on demand rather than kept in the repository:

- **`UPSTREAM`** names the upstream release the Rust matches: v0.27.0.
- **`pnpm conformance`** checks out that release and runs its unchanged test suite against the WebAssembly package: 7,343 passed, 25 skipped, 8 todo. It then captures every composition the suite makes and checks that native Rust produces byte-identical output: 3,567 of 3,567. Deliberate divergences would go in `conformance/known-exceptions.json`, which is empty.
- **A weekly watcher** compares `UPSTREAM` with upstream's latest release. When upstream ships a new release, it opens one issue with the diff, a list of the Rust files matching each changed upstream file, and the conformance result against the new release. `docs/upstream-sync.md` describes how to port a release.

The seven tests that checked the WebAssembly bridge itself moved into the npm package's own tests.

## The public API

Before, every module was public and errors were JavaScript-shaped values. Now the crate root of `federation-composition-rs` re-exports the same list as upstream's `src/index.ts`, with Rust names: `compose_services`, `validate`, `compose_schema_contract`, `sort_sdl`, `transform_supergraph_to_public_schema`, and the result types. Failures return an `Error` type that implements `std::error::Error` and gives access to the GraphQL error and its code. `graphql-rs` exposes parse, print, visit, the AST, `GraphQLError`, `build_schema` and schema validation. The rest is private. The test suite and the bindings reach it through one hidden module documented as unstable. Every public item has rustdoc, and the examples in the READMEs are compiled and run as tests.

## No panics on input

TypeScript throws an exception that a caller can catch. A Rust panic usually ends the program, so a library must not panic on bad input. One agent checked every `unwrap`, `expect`, `panic!` and `unreachable!` reachable from a public function:

- Where upstream throws an error, Rust now returns an error with the same message. One of these sites could be reached from ordinary SDL: a fragment named as an argument type.
- The remaining sites check conditions that the types guarantee, or catch misuse of the visitor API. They do not depend on the input.

Deep nesting was the other risk. graphql-js has no limit and fails with a JavaScript `RangeError` when it runs out of stack. In Rust the same overflow aborts the process. The parser now stops at 128 levels of nesting with a syntax error, and documents built without the parser are checked against the same limit. Some functions recursed once per type in a chain of types, overflowing at about 1,000 types. They now keep an explicit stack and do the same work in the same order, and chains of 20,000 types compose. At the limit, the worst case uses about 260 KiB of stack in a release build. That fits both 2 MiB threads and WebAssembly's 1 MiB stack.

The limit changes behavior in one case. A `@key`, `@requires` or `@provides` field set nested more than 127 levels deep is now a composition error, where upstream would compose it. No upstream test comes near that depth. SECURITY.md documents the limits.

## Fuzzing

Four cargo-fuzz targets run on input seeded from the test fixtures and the ported suite:

| Target         | What it checks                                                                         | Executions |
| -------------- | -------------------------------------------------------------------------------------- | ---------: |
| `parse`        | no panic, and printing then reparsing gives the same document                          |      85.0M |
| `build_schema` | no panic                                                                               |      33.4M |
| `compose`      | up to four subgraphs, then public schema, sorting, transformation and contracts         |      21.8M |
| `compose_ast`  | the same on syntax trees whose names the parser could never produce                    |      23.4M |

One failure came up. graphql-js prints an anonymous query in shorthand, `{ … }`. After a type definition with no fields, reparsing reads that brace as the type's fields. The Rust prints the same thing as graphql-js, so the library stayed as it is and the round-trip check now allows this case. A regression test pins it. The final 15-minute run of each target found nothing.

## Continuous integration and releases

The workflows are in `.github/workflows`:

- **`ci.yml`**: formatting, clippy, tests on Linux, macOS and Windows, the minimum supported Rust version (1.87), docs, cargo-deny, package builds, and the npm package installed alone with Node 22 and 24 and graphql 16 and 17.
- **`conformance.yml`**: runs on pull requests, weekly and on demand.
- **`upstream-watch.yml`**: the weekly upstream watcher.
- **`release-plz.yml`** and **`npm-publish.yml`**: publish the crates and the npm package. They do nothing until the repository sets `RELEASE_ENABLED` and the release tokens.
- **`pages.yml`**: deploys this site.
- **`fuzz.yml`**: daily fuzzing.

Everything a workflow runs was also run locally on macOS, including actionlint. GitHub Actions has never run on this fork, so the Linux and Windows jobs, the issue writes, Pages and releases are still unverified.

## Time

The work took 4 hours 17 minutes of wall-clock time and 1.77 million subagent tokens across seven agents:

1. removing the TypeScript and setting up conformance;
2. three in parallel: the public API, CI and release, and the documentation;
3. merging them;
4. hardening against bad input;
5. a final verification from a clean build.
