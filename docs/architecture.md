# Architecture

This document describes how the code is organized and the parts of the design a contributor needs
to know before changing it. The porting rules and every fixed representation are in
[crates/PORTING.md](../crates/PORTING.md); the procedure for porting a new upstream release is in
[upstream-sync.md](upstream-sync.md).

"Upstream" is [graphql-hive/federation-composition](https://github.com/graphql-hive/federation-composition),
the TypeScript library this repository ports. Paths such as `src/compose.ts` and `__tests__/...`
refer to upstream's files, which `pnpm conformance` checks out into `conformance/upstream/` at the
release recorded in [`UPSTREAM`](../UPSTREAM).

## The crates and how they depend on each other

```
graphql-rs  <--  federation-composition-rs  <--  federation-composition-wasm  <--  npm/ (TypeScript shim)
```

| Crate or package                                                              | Ports                                              | Notes                                                                                                   |
| ----------------------------------------------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| [`crates/graphql-rs`](../crates/graphql-rs)                                   | graphql-js 17.0.2, the parts upstream uses         | No dependency on the other crates.                                                                      |
| [`crates/federation-composition-rs`](../crates/federation-composition-rs)     | upstream's `src/`                                  | The product. Nothing in it exists only to serve the bindings.                                           |
| [`crates/federation-composition-wasm`](../crates/federation-composition-wasm) | nothing; it is glue                                | wasm-bindgen exports, JSON conversion in the shapes of the TypeScript objects, the talc allocator.      |
| [`npm/`](../npm)                                                              | the public modules of upstream's `src/`, as a shim | `npm/src/<path>.ts` has the same exports as upstream's `src/<path>.ts` and calls the WebAssembly build. |

Everything specific to JavaScript (converting to and from JSON, rebuilding `GraphQLError` objects,
lazy getters, class wrappers) lives in `crates/federation-composition-wasm` and `npm/src`.

## The module tree mirrors upstream

`crates/federation-composition-rs/src` mirrors upstream's `src/` one file to one file:
`src/supergraph/composition/object-type.ts` is
`crates/federation-composition-rs/src/supergraph/composition/object_type.rs`, a directory's
`index.ts` is its `mod.rs`, and `src/index.ts` is `lib.rs`, whose re-exports are `index.ts`'s
export list. Each Rust file keeps the TypeScript file's function names (in snake_case), the order
of its functions and its control flow, so a reviewer can read the two side by side. Upstream's
spelling is kept too: the satisfiability rule lives in `satisfiablity/`, as it does upstream.

This mapping is what makes upstream releases portable: the watcher's issue lists, for each changed
upstream file, the Rust file it maps to, and the change is ported there. Do not move, merge or
split modules, and do not refactor logic, even where the TypeScript looks redundant or wrong. A
behavior that tests depend on is behavior; mark a place where the TypeScript looks wrong with a
`// NOTE(port):` comment instead.

Two comment markers explain where the Rust differs in form from the TypeScript:

- `// NOTE(port):` explains a JavaScript semantic that needed thought, or a TypeScript quirk that
  was kept.
- `// PERF:` marks a representation chosen for speed (interned names, cached indexes, borrowed
  strings) that does not change the output. The optimization pass that added these is described
  in [research/journal/04-optimize.md](../research/journal/04-optimize.md).

The top-level modules:

| Module           | Upstream              | Contents                                                                                                     |
| ---------------- | --------------------- | ------------------------------------------------------------------------------------------------------------ |
| `compose`        | `src/compose.ts`      | `compose_services`: validates, then prints the supergraph with the `@link`, `@join__*` and spec definitions. |
| `validate`       | `src/validate.ts`     | `validate`: subgraph validation, then supergraph validation, then the supergraph state is built into AST.    |
| `subgraph`       | `src/subgraph/`       | Subgraph state (`state.rs`) and subgraph validation rules, which are graphql-js-style AST visitors.          |
| `supergraph`     | `src/supergraph/`     | Supergraph state, the per-kind composition builders (`composition/`) and supergraph validation rules.        |
| `contracts`      | `src/contracts/`      | Schema contracts: tag filtering and reachability.                                                            |
| `specifications` | `src/specifications/` | The federation, link, join, tag, inaccessible, cost, auth and context spec definitions.                      |
| `graphql`        | `src/graphql/`        | The library's own supergraph printer, `sort_sdl`, the public-schema transform. Not graphql-js.               |
| `utils`          | `src/utils/`          | Helpers, plus `utils::js` and `utils::collections` (below), which have no upstream file.                     |
| `types`          | `src/types.ts`        | `ServiceDefinition` and the other public input types.                                                        |

## graphql-rs mirrors graphql-js

`crates/graphql-rs/src` follows graphql-js's file layout in the same way: `language/` files are at
the crate root (`lexer.rs`, `parser.rs`, `printer.rs`, `visitor.rs`, `ast.rs`), and `type/`,
`utilities/`, `validation/`, `execution/` and `jsutils/` are subdirectories with graphql-js's file
names. Only what upstream calls is ported: the language layer, `buildSchema`/`buildASTSchema` with
the SDL validation rules, and `validateSchema`/`assertValidSchema`. There is no executable-document
validation and no `extendSchema` of an existing schema.

The AST is a set of plain owned structs named after the graphql-js node kinds without the `Node`
suffix (`FieldDefinition`, `ObjectTypeExtension`), with union enums (`Definition`, `Selection`,
`Value`, `Type`). `Node` is the owned union of every kind and `NodeRef<'a>` the borrowed one.
Optional lists are plain `Vec`s, where an empty list stands for `undefined`. The serde form of a
document is graphql-js's JSON, which is how documents cross into WebAssembly. Parse errors, printed
output and schema validation messages match graphql-js byte for byte, and
[`tests/differential.rs`](../crates/graphql-rs/tests/differential.rs) and
[`tests/schema_differential.rs`](../crates/graphql-rs/tests/schema_differential.rs) check this
against values generated from graphql-js itself.

`crates/federation-composition-rs` imports this crate as `graphql_rs`, because `crate::graphql` is
already the port of upstream's `src/graphql/`.

## The visitor

[`graphql_rs::visitor`](../crates/graphql-rs/src/visitor.rs) ports graphql-js `visit` and
`visitInParallel`, with graphql-js's traversal order and its `key`, `parent`, `path` and
`ancestors` bookkeeping (`VisitInfo`). There are two traversals:

- **`visit`** is read-only. A visitor implements `trait Visitor<'a, C>` with `enter` and `leave`
  callbacks that receive a `NodeRef<'a>` borrowed for the document's lifetime, so a visitor may keep
  references into the document, and a mutable context `C` shared by every visitor. A callback
  returns `VisitAction::{Continue, Skip, Break}`. A TypeScript visitor object
  `{ FieldDefinition(node) { ... } }` becomes a `match node { NodeRef::FieldDefinition(node) => ..., _ => {} }`.
- **`visit_edit`** is graphql-js `visit` with replacements: callbacks return
  `EditAction::{Replace(Node), Delete, Skip, Break, Continue}` and the edited tree is returned; the
  input is not modified.

`visit_in_parallel` runs several visitors in one traversal, as graphql-js does. Each visitor's
`kinds()` lists the node kinds its callbacks act on, and the parallel visitor calls it only for
those, the way graphql-js calls only the handlers a visitor object defines. **When you change the
node kinds a callback matches, update `kinds()` to match.** A kind missing from `kinds()` means the
callback is never called for it, and nothing fails at compile time.

Subgraph validation is built on this. Each rule is a struct implementing `SubgraphRule<'a>` or
`SimpleRule<'a>` (its `NAME` is the TypeScript function name, which `disableValidationRules`
matches) and `Visitor<'a, Ctx<'a>>`. The rules run in parallel with the subgraph state builder's
visitor, sharing `SubgraphValidationContext<'a>` as the context `C`. A TypeScript `throw` inside a
callback becomes `return cx.throw(err)`, which records the error and stops the traversal for every
visitor.

Supergraph validation uses a different visitor,
[`supergraph::composition::visitor`](../crates/federation-composition-rs/src/supergraph/composition/visitor.rs)
(`SupergraphVisitor<'s>`, upstream's `SupergraphVisitorMap`), which walks the composed supergraph
state once for all rules: object types, fields, arguments, interfaces, enums and so on. Its
callbacks return `JsResult`, so a rule that throws aborts the visit as in TypeScript.

## Composition, step by step

`compose_services` calls `validate` and then prints the result:

1. **Subgraph validation.** For each subgraph, `@link`s are extracted, the federation version is
   detected, the federation and link spec definitions are added, and the subgraph rules and the
   subgraph state builder run in one parallel visit. If any subgraph has errors, composition stops
   with those errors.
2. **Supergraph state.** Each subgraph state is cleaned of the federation and link spec types and
   shared as `Rc<SubgraphState>`; it is read-only from then on. The supergraph state builder visits
   every subgraph state and composes the merged type states (`supergraph/composition/*.rs`, one
   builder per type kind).
3. **Supergraph validation.** The pre-supergraph rules run, then the post-supergraph rules run in
   one `SupergraphVisitor` pass. Where a TypeScript rule mutates the supergraph state during the
   visit, the Rust rule records the write in `SupergraphValidationContext::pending_state_updates`,
   and `validate_supergraph` applies the writes after the visit. If there are no errors, the
   satisfiability rule and the auth rules run in a second pass.
4. **Output.** The supergraph state is built into AST definitions and printed by the library's own
   printer (`graphql/printer.rs`, not graphql-js `print`). `CompositionSuccess` computes
   `supergraph_sdl`, `public_document_node` and `public_sdl` on first use and caches them.

## The satisfiability graph arena

The satisfiability rule
([`satisfiablity_rule.rs`](../crates/federation-composition-rs/src/supergraph/validation/rules/satisfiablity_rule.rs)
and [`satisfiablity/`](../crates/federation-composition-rs/src/supergraph/validation/rules/satisfiablity))
checks that every field of the supergraph can be reached by some query plan. It is the most
expensive part of composing large schemas.

`Supergraph::new` builds three kinds of graph from the supergraph state: one graph per subgraph,
the **merged graph** (every subgraph graph joined through entity keys and overrides) and the
**supergraph graph**. The `Walker` walks the supergraph graph depth-first, and for every step looks
for a matching path in the merged graph with `PathFinder` and `MoveValidator` (which checks
`@key`, `@requires` and `@provides` requirements). Steps that cannot be resolved become
satisfiability errors.

In TypeScript these graphs are object graphs with identity. In Rust they are arenas:

- `Graph<'s>` owns `nodes: Vec<Node>` and `edges: Vec<Edge>`. A `NodeId` is the TypeScript
  `node.index` (one node per `nodesByTypeIndex` slot) and an `EdgeId` is an edge's position.
  Identity comparisons in TypeScript (`edge === other`, `previousEdges.includes(edge)`) compare ids.
  The head and tail indexes hold `EdgeId`s.
- Nodes refer to the type states they describe by reference into the `SupergraphState`
  (`NodeTypeState<'s>`), so `'s` is the lifetime of the supergraph state.
- `addSubgraph` moves a subgraph graph's nodes and edges into the merged graph and remaps their
  ids, where TypeScript moves the same objects and reassigns `node.index`.
- A graph is built through `&mut self` and then shared as `Rc<Graph>`. The walk only needs `&self`:
  what it fills in as it goes (the edge resolvability cache, visited graph combinations, the type
  children cache) is in `RefCell` or `Cell`.
- An `OperationPath` holds ids, not references, and takes the graph it belongs to when it needs an
  edge: a `WalkTracker`'s `super_path` belongs to the supergraph graph and its `paths` to the merged
  graph.
- `MoveValidator` and `PathFinder` are cheap `Clone` structs over `Rc<Graph>`. The TypeScript cycle
  MoveValidator -> PathFinder -> MoveValidator is broken by creating the finder on demand.
- Upstream's `bigint` context masks are `ContextMask`, a bit set that can grow past 128 bits, and
  `lazy(() => ...)` values are `helpers::Lazy`. Subgraph names are interned (`GraphNames`, shared
  by the graphs of one `Supergraph`), so visited-graph checks and queue entries compare integers
  instead of strings.

The full list of these representations, and the rest of the fixed design decisions (subgraph
state, supergraph builders, thrown values, results), is in the "Design decisions" section of
[crates/PORTING.md](../crates/PORTING.md#design-decisions).

## JavaScript semantics to preserve

The output must match the TypeScript byte for byte, and several JavaScript behaviors decide it.
The complete table is in [crates/PORTING.md](../crates/PORTING.md#javascript-semantics-that-must-be-preserved);
these are the rules contributors most often need:

- **Iteration order is observable.** It decides the order of errors and of the printed SDL. Use
  `IndexMap`/`IndexSet` from `crate::utils::collections` (in graphql-rs, `crate::collections`):
  insertion-ordered maps with the FxHash hasher. Never use `indexmap::IndexMap` directly or std
  `HashMap`/`HashSet`. Remove entries with `shift_remove`, never `swap_remove`, which reorders.
- **Plain objects order integer-like keys first.** A JavaScript object used as a dictionary
  iterates keys such as `"0"` and `"12"` first, in numeric order. Where keys can be integer-like,
  the port reproduces that and says so in a `NOTE(port)`.
- **Sorting and comparison.** `array.sort(cmp)` is a stable `sort_by` with the comparator ported
  exactly. `localeCompare` is `utils::js::locale_compare` (ICU root collation emulation), never
  `cmp`. Number formatting (`String(n)`, `${n}`) is `utils::js::number_to_string`. Both are checked
  against values recorded from Node in
  [`tests/js_semantics.rs`](../crates/federation-composition-rs/tests/js_semantics.rs).
- **Truthiness.** `if (x)` is false for `""`, `0` and `NaN` and true for every array; `??` and
  `||` differ on `""`, `0` and `false`. Port each according to the type it actually operates on.
- **Exceptions.** A JavaScript `throw` is `Err(JsError)` (`crate::utils::js::JsError`), and every
  function whose TypeScript can throw returns `JsResult<T>`. A `catch` becomes a match on the
  `Err`, and it must catch what JavaScript would, including runtime `TypeError`s from reading a
  property of `undefined`. A throw that nothing in upstream's `src/` catches reaches the caller:
  the public functions return `JsResult`. Never `unwrap()` a value derived from user input unless
  the TypeScript also crashes there.
- **Error messages are compared verbatim** by upstream's tests. Copy them exactly.
- **Debug logging** is a no-op logger with the same call sites; the closures that build log
  messages must not run.

## How the bindings reach JavaScript

The WebAssembly build exposes one function per upstream export the shim needs
([`exports.rs`](../crates/federation-composition-wasm/src/exports.rs)). Arguments and results are
JSON: AST nodes use graphql-js's JSON form, and documents cross as UTF-8 bytes rather than strings,
because wasm-bindgen copies strings one character at a time. A composition's supergraph document is
returned as bytes and decoded by the shim only when `supergraphDocumentNode` is read. A Rust
`JsError` is rethrown by the shim as a JavaScript `Error` or `GraphQLError` with the same message.

The shim in `npm/src` mirrors upstream's `src/` paths so that the conformance check can redirect
each upstream import to `federation-composition-wasm/internal/<path>`. The package's `.` export
matches upstream's `src/index.ts`; the `internal/*` entry points are not a stable API.

## Tests

| Where                                                                                                                 | What                                                                                                           |
| --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| [`crates/federation-composition-rs/tests/suite`](../crates/federation-composition-rs/tests/suite)                     | Upstream's tests ported to Rust. Its module tree mirrors upstream's `__tests__/`; porting rules in PORTING.md. |
| [`crates/federation-composition-rs/tests/corpus.rs`](../crates/federation-composition-rs/tests/corpus.rs)             | Native output against upstream's recorded TypeScript output, on every composition upstream's tests perform.    |
| [`crates/federation-composition-rs/tests/js_semantics.rs`](../crates/federation-composition-rs/tests/js_semantics.rs) | `locale_compare` and `number_to_string` against Node.                                                          |
| [`crates/graphql-rs/tests`](../crates/graphql-rs/tests)                                                               | Differential tests against graphql-js, the visitor tests, and the generators of the expected values.           |
| [`npm/test`](../npm/test)                                                                                             | The JavaScript-to-Rust bridge checks and a smoke test of the package.                                          |
| [`conformance/`](../conformance)                                                                                      | Upstream's unchanged test suite against the npm package, and the capture of the corpus.                        |

Never edit a test or a recorded output to make it pass. A deliberate divergence from upstream is
the owner's decision and goes in
[`conformance/known-exceptions.json`](../conformance/known-exceptions.json) with its reason.
