# Federation Composition in Rust

A Rust port of [`@theguild/federation-composition`](https://github.com/graphql-hive/federation-composition),
the open source Apollo Federation v1/v2 composition library by [The Guild](https://the-guild.dev):
given a set of subgraph schemas, it validates them and composes the supergraph schema. The port
produces the same supergraph SDL byte for byte and the same errors as the TypeScript library, at the
upstream release recorded in [`UPSTREAM`](UPSTREAM) (v0.27.0).

| Path                                                                       | What it is                                                                                                                                                             |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`crates/federation-composition-rs`](crates/federation-composition-rs)     | The library: the port of upstream's `src/`, module for module, with upstream's test suite ported to Rust (`tests/suite`).                                              |
| [`crates/graphql-rs`](crates/graphql-rs)                                   | The port of graphql-js 17.0.2 the library needs: the language layer (AST, parser, printer, visitor) and `buildSchema`/`assertValidSchema`.                             |
| [`crates/federation-composition-wasm`](crates/federation-composition-wasm) | wasm-bindgen bindings of the library.                                                                                                                                  |
| [`npm/`](npm)                                                              | The npm package `federation-composition-wasm`: the library compiled to WebAssembly, a drop-in replacement for `@theguild/federation-composition`.                      |
| [`conformance/`](conformance)                                              | The check that the port matches upstream: upstream's unchanged test suite against the npm package, and native Rust against upstream's TypeScript on every composition. |
| [`docs/upstream-sync.md`](docs/upstream-sync.md)                           | How a new upstream release is ported.                                                                                                                                  |
| [`crates/PORTING.md`](crates/PORTING.md)                                   | The porting guide.                                                                                                                                                     |
| [`bench/`](bench), [`research/`](research)                                 | Benchmarks, and the research journal of the port.                                                                                                                      |

## Using it

From Rust (in a function returning `Result<(), Box<dyn std::error::Error>>`):

```rust
use federation_composition_rs::compose::{compose_services, CompositionResult};
use federation_composition_rs::types::ServiceDefinition;

let services = vec![ServiceDefinition {
    name: "users".into(),
    url: None,
    type_defs: graphql_rs::parse(r#"
        extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])
        type User @key(fields: "id") { id: ID! name: String! }
        type Query { users: [User] }
    "#)?,
}];
match compose_services(&services, None)? {
    CompositionResult::Success(success) => println!("{}", success.supergraph_sdl()),
    CompositionResult::Failure(failure) => eprintln!("{:?}", failure.errors),
}
```

From JavaScript, replace `@theguild/federation-composition` with `federation-composition-wasm`
([npm/README.md](npm/README.md)).

## Development

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"   # see crates/PORTING.md
cargo test --workspace
pnpm install
pnpm conformance     # check out upstream at UPSTREAM; run its tests on wasm; compare native output
pnpm test:npm        # the npm package's own tests
```

Nothing is published yet.

## Credits

This is a port of `@theguild/federation-composition`, created and maintained by The Guild, and of
graphql-js by the GraphQL Contributors, and keeps their MIT license. Thanks to The Guild for the
original library and for encouraging this port. See [LICENSE](LICENSE) and each crate's LICENSE.
