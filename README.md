# Federation Composition in Rust

Apollo Federation v1 and v2 schema composition in Rust. Given a set of subgraph schemas, it
validates them and composes the supergraph schema.

It is a port of [`@theguild/federation-composition`](https://github.com/graphql-hive/federation-composition),
the open source composition library by [The Guild](https://the-guild.dev) ("upstream"). The port
produces the same supergraph SDL byte for byte and the same errors (message, `extensions.code` and
order) as the TypeScript library at the upstream release recorded in [`UPSTREAM`](UPSTREAM)
(v0.27.0). This is checked on every pull request and every week against upstream's own test suite
(see [Correctness](#correctness) and [Staying in sync with upstream](#staying-in-sync-with-upstream)).

Nothing is published to crates.io or npm yet. The install instructions below give the names the
packages will be published under, and how to use them from this repository until then.

## Packages

| Package                                                         | Language   | What it is                                                                                                                                                             |
| --------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`federation-composition-rs`](crates/federation-composition-rs) | Rust       | The composition library: a module-for-module port of upstream's `src/`.                                                                                                |
| [`graphql-rs`](crates/graphql-rs)                               | Rust       | The part of [graphql-js](https://github.com/graphql/graphql-js) 17.0.2 the library needs: AST, lexer, parser, printer, visitor, and `buildSchema`/`assertValidSchema`. |
| [`federation-composition-wasm`](npm)                            | JavaScript | The library compiled to WebAssembly, as a drop-in replacement for `@theguild/federation-composition`.                                                                  |

### federation-composition-rs

```toml
[dependencies]
federation-composition-rs = "0.1"
graphql-rs = "0.1"
# Until the crates are published:
# federation-composition-rs = { git = "https://github.com/Enrico2/federation-composition" }
# graphql-rs = { git = "https://github.com/Enrico2/federation-composition" }
```

The subgraph schemas are parsed with `graphql-rs`; `compose_services` returns the supergraph or
the composition errors.

```rust
// README example: compose
use federation_composition_rs::compose::{compose_services, CompositionResult};
use federation_composition_rs::types::ServiceDefinition;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let services = vec![ServiceDefinition {
        name: "users".into(),
        url: None,
        type_defs: graphql_rs::parse(
            r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])
            type User @key(fields: "id") { id: ID! name: String! }
            type Query { users: [User] }
            "#,
        )?,
    }];
    match compose_services(&services, None)? {
        CompositionResult::Success(success) => println!("{}", success.supergraph_sdl()),
        CompositionResult::Failure(failure) => eprintln!("{:?}", failure.errors),
    }
    Ok(())
}
```

The other functions upstream's `src/index.ts` exports (`validate`, `compose_schema_contract`,
`sort_sdl`, `transform_supergraph_to_public_schema`, ...) are re-exported from the crate root
under their snake_case names.

### graphql-rs

```toml
[dependencies]
graphql-rs = "0.1"
# Until the crate is published:
# graphql-rs = { git = "https://github.com/Enrico2/federation-composition" }
```

Parsing, printing and schema validation behave like graphql-js, including its error messages:

```rust
// README example: graphql-rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = graphql_rs::parse("type Query { hello: String }")?;
    println!("{}", graphql_rs::print(&document));

    let schema = graphql_rs::build_schema("type Query { hello: String }")?;
    graphql_rs::assert_valid_schema(&schema)?;
    Ok(())
}
```

### federation-composition-wasm (npm)

```sh
npm install federation-composition-wasm graphql
```

It has the same exports, types and results as `@theguild/federation-composition`, so switching is
a change of import:

```diff
-import { composeServices, compositionHasErrors } from "@theguild/federation-composition";
+import { composeServices, compositionHasErrors } from "federation-composition-wasm";
```

```ts
import { parse } from "graphql";
import {
  composeServices,
  compositionHasErrors,
} from "federation-composition-wasm";

const result = composeServices([
  {
    name: "users",
    typeDefs: parse(/* GraphQL */ `
      extend schema
        @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])
      type User @key(fields: "id") {
        id: ID!
        name: String!
      }
      type Query {
        users: [User]
      }
    `),
  },
]);

if (compositionHasErrors(result)) {
  console.error(result.errors);
} else {
  console.log(result.supergraphSdl);
}
```

`graphql` 16 or 17 is a peer dependency. The package is an ES module; CommonJS code can
`require()` it on Node.js 20.19, 22.12 and later. Until it is published, build it from this
repository with `pnpm wasm:build` (see [npm/README.md](npm/README.md)).

## Performance

Measured on the compositions upstream's test suite performs (3,564 of them) and on two larger
fixtures, against `@theguild/federation-composition` on Node.js, on the same machine
([research journal, milestone 4](research/journal/04-optimize.md)):

| Speedup over TypeScript                 | Native Rust | WebAssembly |
| --------------------------------------- | ----------: | ----------: |
| Test-suite compositions, geometric mean |       9.75× |       5.12× |
| Huge schema (191 subgraphs)             |       4.21× |       2.58× |

On the huge schema, TypeScript takes 307.5 ms, WebAssembly 119.0 ms and native Rust 73.0 ms.
Native Rust is faster than TypeScript on every one of the 3,564 compositions; WebAssembly is
slower on 2 single-subgraph failures that take 13 µs in TypeScript, where the cost of crossing
into WebAssembly is larger than the composition.

The research site in [`research/`](research) has the charts, the method and the full journal of
the port. Build it with `pnpm --filter federation-composition-rust-research build` (output in
`research/dist/`) or serve it with `pnpm --filter federation-composition-rust-research dev`. The
benchmark harness is described in [bench/README.md](bench/README.md).

## Correctness

Four independent checks show that the port behaves like the TypeScript:

1. **Upstream's test suite on WebAssembly.** Upstream's TypeScript test suite runs unchanged
   against the npm package: every import of upstream's `src/` is redirected to the package, and
   loading any of upstream's implementation is an error. At v0.27.0: 7,343 passed, 25 skipped,
   8 todo, as on upstream's own TypeScript.
2. **Corpus differential.** Every composition upstream's test suite performs is recorded with its
   TypeScript output, and native Rust must produce identical output for each one
   ([`tests/corpus.rs`](crates/federation-composition-rs/tests/corpus.rs)). At v0.27.0: 3,567 of
   3,567 identical.
3. **The test suite in Rust.** Upstream's tests are ported to Rust
   ([`tests/suite`](crates/federation-composition-rs/tests/suite)), so the library is tested
   without the WebAssembly bridge: 3,774 tests, 17 ignored (upstream's skipped and todo tests).
4. **graphql-js differentials.** `graphql-rs` is compared with graphql-js 17.0.2 on parsing,
   printing and errors ([`tests/differential.rs`](crates/graphql-rs/tests/differential.rs)), and on
   building and validating the 1,966 schemas in the conformance corpus
   ([`tests/schema_differential.rs`](crates/graphql-rs/tests/schema_differential.rs)).

A deliberate divergence from upstream would be listed, with its reason, in
[`conformance/known-exceptions.json`](conformance/known-exceptions.json) (empty today). Upstream's
tests and recorded outputs are never edited.

## Staying in sync with upstream

- [`UPSTREAM`](UPSTREAM) records the upstream release the Rust matches.
- `pnpm conformance` ([conformance/](conformance)) checks upstream out at that release into
  `conformance/upstream/` (gitignored) and runs checks 1 and 2 above. CI runs it on every pull
  request, on pushes to `main` and weekly ([`conformance.yaml`](.github/workflows/conformance.yaml)).
- A weekly watcher ([`upstream-watch.yaml`](.github/workflows/upstream-watch.yaml),
  `pnpm conformance:watch`) compares upstream's latest release with `UPSTREAM`. When a newer one
  exists, it runs the conformance check against it and opens or updates a GitHub issue labelled
  `upstream-sync` with the diff of upstream's `src/` and `__tests__/` between the two tags, the Rust
  file each changed file maps to, the failing upstream tests and the compositions whose output
  changed.

The procedure for porting a release is [docs/upstream-sync.md](docs/upstream-sync.md).

## Repository layout

| Path                                                                       | Contents                                                                                                                       |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| [`crates/federation-composition-rs`](crates/federation-composition-rs)     | The library; `src/` mirrors upstream's `src/` file for file. Tests: `tests/suite`, `tests/corpus.rs`, `tests/js_semantics.rs`. |
| [`crates/graphql-rs`](crates/graphql-rs)                                   | The graphql-js port; `src/` mirrors graphql-js's files. Differential tests and their generators in `tests/`.                   |
| [`crates/federation-composition-wasm`](crates/federation-composition-wasm) | wasm-bindgen bindings used by the npm package.                                                                                 |
| [`npm/`](npm)                                                              | The npm package `federation-composition-wasm`: the TypeScript shim (`src/`) and its tests (`test/`).                           |
| [`conformance/`](conformance)                                              | The upstream conformance check, the watcher and the known-exceptions list.                                                     |
| [`docs/`](docs)                                                            | [Architecture](docs/architecture.md) and the [upstream sync procedure](docs/upstream-sync.md).                                 |
| [`crates/PORTING.md`](crates/PORTING.md)                                   | The porting guide: module mapping, JavaScript semantics to preserve, design decisions, test-porting rules.                     |
| [`bench/`](bench)                                                          | Benchmark and profiling scripts.                                                                                               |
| [`research/`](research)                                                    | The research site and journal of the port.                                                                                     |
| [`UPSTREAM`](UPSTREAM)                                                     | The upstream release the Rust matches.                                                                                         |

## Development

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"   # rustup's toolchain; see crates/PORTING.md
pnpm install
pnpm conformance     # check out upstream at UPSTREAM, run its tests on wasm, compare native output
pnpm test:npm        # the npm package's own tests (after pnpm wasm:build, which pnpm conformance runs)
node crates/graphql-rs/tests/gen-corpus.mjs          # graphql-js expected values, from the checkout
node crates/graphql-rs/tests/gen-schema-corpus.mjs   # graphql-js schema expected values, from the corpus
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

`cargo test --workspace` needs the upstream checkout and the generated graphql-js corpora, so on a
fresh clone run the commands in this order. Building the WebAssembly package needs the
`wasm32-unknown-unknown` target and `wasm-bindgen-cli` 0.2.129; see
[crates/PORTING.md](crates/PORTING.md#toolchain). How the code is organized is in
[docs/architecture.md](docs/architecture.md).

Minimum supported Rust version: <!-- MSRV: to be filled in by the CI setup --> TBD.

## License and credits

MIT. See [LICENSE](LICENSE).

This is a port of [`@theguild/federation-composition`](https://github.com/graphql-hive/federation-composition),
created and maintained by [The Guild](https://the-guild.dev), and of
[graphql-js](https://github.com/graphql/graphql-js) by the GraphQL Contributors. Both are MIT
licensed, and this project keeps their license and copyright notices (see [LICENSE](LICENSE) and
each crate's LICENSE). Thanks to The Guild for the original library and for encouraging this port.
