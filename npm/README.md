# @lemonberrylabs/federation-composition-wasm

Apollo Federation v1/v2 schema composition for JavaScript, compiled to WebAssembly from
[federation-composition](../crates/federation-composition), a Rust port of
[`@theguild/federation-composition`](https://github.com/graphql-hive/federation-composition) by
[The Guild](https://the-guild.dev).

It is a drop-in replacement for `@theguild/federation-composition`: the same exports
(`composeServices`, `validate`, `composeSchemaContract`, ...), the same types, the same supergraph
SDL byte for byte and the same errors. Every release is checked against the unchanged test suite of
the `@theguild/federation-composition` release it matches (recorded in
[`UPSTREAM`](../UPSTREAM); see [conformance/](../conformance)).

## Installation

```sh
npm install @lemonberrylabs/federation-composition-wasm graphql
```

`graphql` (16 or 17) is a peer dependency, as it is for `@theguild/federation-composition`.

## Usage

Replace the import; nothing else changes:

```diff
-import { composeServices, compositionHasErrors } from "@theguild/federation-composition";
+import { composeServices, compositionHasErrors } from "@lemonberrylabs/federation-composition-wasm";
```

```ts
import { parse } from "graphql";
import {
  composeServices,
  compositionHasErrors,
} from "@lemonberrylabs/federation-composition-wasm";

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

The package is an ES module for Node.js (it loads the WebAssembly module with Node's `require`).
It needs Node.js 22.12 or later, where CommonJS code can also `require()` it.

`@lemonberrylabs/federation-composition-wasm/internal/<path>` exposes the shim of each upstream module
(`internal/subgraph/state`, `internal/supergraph/composition/ast`, ...). Those entry points exist
so upstream's tests can run against this package; they are not a stable API.

## Building

From the repository root:

```sh
pnpm install
pnpm wasm:build     # cargo (wasm32) + wasm-bindgen into pkg/, tsc into dist/
pnpm test:npm       # the bridge checks and a smoke test of the package
```

Building needs the Rust toolchain with the `wasm32-unknown-unknown` target and the wasm-bindgen CLI
at the version in `Cargo.lock` (`cargo install wasm-bindgen-cli --version '=0.2.129' --locked`);
`wasm-opt` (binaryen) is used when installed. See [crates/PORTING.md](../crates/PORTING.md).

## Credits

This package is built from a port of `@theguild/federation-composition`, created and maintained by
The Guild, and of graphql-js by the GraphQL Contributors. It keeps their MIT license. See
[LICENSE](LICENSE).
