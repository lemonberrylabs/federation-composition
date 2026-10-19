# Porting `src/` to Rust

This is the working guide for the Rust port of `@theguild/federation-composition`.
The TypeScript in `src/` is the specification. The port must behave identically:
same supergraph SDL byte for byte, same errors (message, `extensions.code`, order),
same results from every exported function. The existing TypeScript test suite runs
against the WebAssembly build and must pass unchanged.

## Layout

| Crate | Purpose |
|---|---|
| `crates/graphql` | Port of graphql-js 17.0.2's language layer: AST, lexer, parser, printer, visitor, `stripIgnoredCharacters`, `GraphQLError`, specified scalars/directives. |
| `crates/composition` (`federation_composition`) | Port of `src/`. Depends on `crates/graphql` under the name `graphql_js`, because `crate::graphql` is the port of `src/graphql/`. |
| `crates/composition-wasm` | wasm-bindgen bindings used by the JS shim. |
| `wasm/` | TypeScript shim exposing the same modules and exports as `src/`, backed by the WebAssembly build. Tests are pointed at it by a Vitest alias. |

**Never edit a test to make it pass.** The tests in `__tests__/` and `src/**/*.spec.ts`
are the oracle: a failing test means the Rust (or the bindings) is wrong, and the fix goes
there. Only the routing (`vitest.wasm.config.js`, the `wasm/` shim) may change. If a test
cannot run against the WebAssembly build at all, stop and ask the owner.

The long-term product is the pure Rust library. The WebAssembly build exists so current
JS users can migrate and so the existing TypeScript tests can verify the port unchanged.
Keep everything JS-specific (JSON/JS object conversion, rebuilding JS `GraphQLError`s,
lazy getters, class wrappers) in `crates/composition-wasm` and `wasm/`; nothing in
`crates/composition` should exist only to serve the bindings.

Module structure mirrors `src/` one to one: `src/supergraph/composition/object-type.ts`
becomes `crates/composition/src/supergraph/composition/object_type.rs`. Keep the same
function names in snake_case, the same order of functions within a file, and the same
control flow. A reviewer should be able to read a TS file and its Rust file side by side.
Do not refactor, simplify, or "improve" logic while porting, even where the TS looks
redundant or buggy: a bug that tests depend on is behavior. Leave a `// NOTE(port):`
comment where the TS looks wrong instead.

## Toolchain

Use rustup's toolchain (Homebrew's cargo is first on PATH and has no wasm32 std):

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
```

`cargo build -p federation_composition`, and before calling a module done,
`cargo build --release --target wasm32-unknown-unknown -p federation_composition`.
Linking a wasm32 `cdylib` (the bindings) runs rustup's `rust-lld`, which on macOS
finds `libLLVM.dylib` only with
`DYLD_FALLBACK_LIBRARY_PATH=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/lib`
(`pnpm wasm:build` sets it). Building an `rlib` does not link and does not need it.

`pnpm wasm:build` builds `crates/composition-wasm` into `wasm/pkg` (needs
`cargo install wasm-bindgen-cli --version '=0.2.129' --locked`); `pnpm test:wasm`
runs the test suite against it.
No `unsafe`. No dependencies that need `getrandom` (wasm32).

## JavaScript semantics that must be preserved

**Iteration order is observable** (it decides error order and SDL order). Port:

| TypeScript | Rust |
|---|---|
| `Map<K, V>` | `IndexMap<K, V>` |
| `Set<T>` | `IndexSet<T>` |
| `map.delete(k)` / `set.delete(v)` | `shift_remove` (never `swap_remove`: it reorders) |
| `map.set(k, v)` on an existing key | `insert` (keeps the original position, same as JS) |
| plain object used as a dictionary, iterated with `Object.keys/values/entries` or `for..in` | `IndexMap`, but JS orders integer-like keys (`"0"`, `"12"`) first in ascending numeric order, then the rest in insertion order. Only matters if the keys can be integer-like; say so in a `NOTE(port)` if they can. |
| `array.sort(cmp)` | `sort_by` (both stable). Port the comparator exactly; a comparator returning a number maps to `Ordering` by sign. |
| `array.sort()` with no comparator | sorts by UTF-16 code units of `String(x)`. For ASCII strings that is byte order. |
| `a.localeCompare(b)` | `crate::utils::js::locale_compare` (ICU root collation emulation, tested against Node). Never `cmp`. |
| `lodash.sortby` | stable sort by the key(s), in order. |
| `JSON.stringify` of objects | serde_json with `preserve_order`, same key order as the TS object literal. |
| `String(number)`, template literal `${n}` | `crate::utils::js::number_to_string` (JS number formatting: `1`, `1.5`, `1e+21`). |
| `parseInt`, `parseFloat`, `Number(x)` | port to a helper with JS semantics if inputs can be non-trivial. |
| `str.length`, `slice`, `substring`, indexes | UTF-16 code units in JS. Inputs here are GraphQL names and SDL, which are ASCII except in descriptions and string values. If a string operation can see non-ASCII text, use char-based logic and note it. |
| regex | the `regex` crate is fine for the patterns used; check each pattern's semantics (JS `\s`, `\w`, flags). |

**Truthiness and optional values.** `if (x)` on a string is false for `""`; on a
number false for `0` and `NaN`; on an array always true. `x ?? y` vs `x || y` differ
for `""`, `0`, `false`. Port each one according to the type it actually operates on.

**Mutation and sharing.** TS code mutates objects through references held in several
places (a field object reachable from a map and also from a local). In Rust:
- Prefer owned data in `IndexMap`s, mutated through `get_mut` by key or by index.
- Factories that return objects of closures over a shared `state`
  (`objectTypeFactory(state)`, `createSubgraphStateBuilder(...)`) become structs or
  functions that take `&mut State` explicitly.
- Graph-shaped data with identity (satisfiability `Node`/`Edge`) becomes arena
  storage (`Vec` + typed index) or `Rc` for immutable shared values. Use
  `Rc<RefCell<_>>` only where the TS genuinely mutates one object through several
  owners and an index would not work; say why in a comment.
- `lazy(() => ...)` memoization becomes `std::cell::OnceCell`.
- `BigInt` bit sets (satisfiability contexts) become a bit set type that can grow
  beyond 128 bits.
- `Symbol` identities become enum variants.

**Exceptions.** A `throw` that some `catch` in `src/` can observe must become an
`Err` that the corresponding catch site handles. Before porting a `try/catch`, list
what can throw inside it in JS, including runtime errors (a `TypeError` from reading
a property of `undefined` inside the try block is caught too), and make the Rust
produce the same caught outcome. A `throw` that nothing in `src/` catches escapes to
the library's caller; those become `Err` values propagated to the public API (the
shim rethrows them as JS `Error`s with the same message). Never `unwrap()`/`expect()`
on anything derived from user input unless the TS would also crash there, and then
use `expect` with the TS error message.

**Errors.** `new GraphQLError(message, { extensions: { code } })` becomes
`graphql::GraphQLError`. Composition errors carry only `message` and `extensions`.
Error messages are compared by tests verbatim; copy them exactly, including
punctuation and quotes.

**Debug logging** (`debug` package, `Logger`): port as a no-op logger with the same
call sites; the closures that build log messages must not run (they are lazy in TS
too, and some are expensive).

**Module-level constants** (`export const typeDefs = parse(sdl)` at import time)
become `std::sync::LazyLock` statics or functions. They are pure.

## graphql-js usage

Use `crates/graphql` for everything the TS imports from `"graphql"`: AST types,
`parse`, `parseValue`, `print`, `visit`, `visitInParallel`, `stripIgnoredCharacters`,
`GraphQLError`, `Kind`, predicates, specified scalars and directives. Note that
`src/graphql/printer.ts` is the library's own printer (used for supergraph output);
port it in `crates/composition`, do not substitute graphql-js `print` for it.

### `graphql_js` crate API in brief

- AST structs are named like graphql-js node kinds without the `Node` suffix
  (`FieldDefinition`, `StringValue`, `ObjectTypeExtension`). Union enums: `Definition`,
  `Selection`, `Value`, `Type`, `TypeDefinition`, `TypeExtension`. `Node` is the owned
  union of every kind, `NodeRef<'a>` the borrowed one. `Kind` and `OperationType` enums
  have `as_str()`. `ConstValue`/`ConstDirective`/`ConstArgument` alias the plain types.
- Optional lists (`directives?`, `fields?`, `arguments?`) are plain `Vec`s; empty means
  `undefined`. Optional nodes are `Option`. The `type` field is spelled `ty`;
  `ListType`/`NonNullType` hold `Box<Type>`.
- Every node implements `Default`: `FieldDefinition { name: Name::new("id"), ty: Type::named("ID"), ..Default::default() }`.
  Helpers: `Value::string`, `Directive::new`, `Type::non_null`, `NamedType::new`;
  `node.name()` on `NodeRef` returns the name of any node that has one.
- `parse`, `parse_value`, `parse_const_value`, `parse_type` return `Result<_, GraphQLError>`
  with graphql-js's exact messages. `print(impl Into<NodeRef>)` is graphql-js `print`.
  `strip_ignored_characters`, `concat_ast`, predicates (`is_type_definition_node`...).
- `GraphQLError { message, extensions: IndexMap<String, serde_json::Value> }` with
  `new`, `with_code`, `code()`.
- Read-only visit: `visit(root, &mut v, &mut cx) -> bool`, where
  `trait Visitor<'a, C> { fn enter/leave(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction }`,
  `VisitAction::{Continue, Skip, Break}`, `VisitInfo` has graphql-js `key`, `parent`,
  `path`, `ancestors`. `visit_in_parallel(Vec<Box<dyn Visitor<'a, C>>>)`. A TS visitor
  object becomes a `match node { NodeRef::FieldDefinition(f) => ..., _ => {} }`.
  Visitors may keep `&'a` references into the document (see `TypeNodeInfo` in
  `crates/graphql/tests/visitor.rs`).
- Editing visit (graphql-js `visit` with returned replacements):
  `visit_edit(root, &mut impl EditVisitor) -> EditResult` and `visit_in_parallel_edit`,
  `EditAction::{Replace(Node), Delete, Skip, Break, Continue}`; `.into_document()`.
- `specified_directives()`, `specified_scalar_types()`, `INTROSPECTION_TYPE_NAMES`,
  `is_specified_directive`, `is_specified_scalar_type`, `is_introspection_type`.
- JSON (serde) matches graphql-js's JSON for parsed documents; deserialization accepts
  missing/`null`/`[]` optional fields and ignores `loc`.

`buildASTSchema`/`getNamedType`/`isObjectType` from graphql-js are used only in
`src/contracts/reachable-type-filter.ts`; port the needed subset of schema building
inside that module (including any error `buildASTSchema` would throw for the inputs
the tests use).

## Done means

- The module compiles for native and wasm32 with no warnings.
- Every exported TS function has its Rust counterpart with the same behavior.
- `// NOTE(port):` comments for every place where a JS semantic needed thought.
- No `todo!()`/`unimplemented!()` left behind.
- No crate-level `allow`s. A parameter a trait signature requires but the body does not use
  is prefixed with `_`. Unused code the TS also has is removed unless removing it would make
  the Rust structure diverge from the TS file; then it keeps a targeted `#[allow(...)]` with a
  one-line `// TS: ...` reason.

## Design decisions

These representations are fixed. If a signature has to change, change it and fix the call
sites (a mechanical edit in another file is fine), and say so in the commit message.

**Thrown values.** `crate::utils::js::JsError` is a JS throw: `JsError::Error { name, message }`
(`new Error`, or a runtime `TypeError`) or `JsError::GraphQL(GraphQLError)`. Every function
whose TS can throw (directly, via `ensureValue`, `parseFields`, the `getOrCreate*` kind checks,
`graphIdToName`, ...) returns `JsResult<T>`; a `catch` maps to matching on the `Err`.
`to_js_string()` is `String(error)` (`"Error: msg"`), `into_graphql_error()` is
`error instanceof GraphQLError ? error : new GraphQLError(String(error))`. The public functions
(`compose_services`, `validate`, ...) return `JsResult` too. Also in `utils::js`: `parse_int`,
`parse_float` (JS semantics), next to `locale_compare` and `number_to_string`.

**Numbers.** JS `number` values (cost weights, list sizes) are `f64`, printed with
`number_to_string`; `number | null` is `Option<f64>`.

**Subgraph state.** `SubgraphState` owns everything (`types: IndexMap<String, SubgraphType>`,
`SubgraphType` an enum of the seven kinds). AST nodes kept in state (`ast.directives`) are
owned clones (`subgraph::state::Ast`, reused by the supergraph states). `schema` keeps its key
insertion order (the TS iterates it with `for..in`). `createSubgraphStateBuilder` returns the
struct `SubgraphStateBuilder`, which owns `state` and the closure locals. Each factory of
closures (`objectTypeFactory(state, ...)`) is a short-lived borrowing struct:
`builder.object_type().field().arg().set_type(..)` is `builder.objectType.field.arg.setType(..)`.
Every setter returns `JsResult<()>` (the `getOrCreate*` helpers throw on a kind clash). After
validation, `validate.rs` moves each state out (`into_state`), cleans it, and shares it as
`Rc<SubgraphState>` (`supergraph::state::SubgraphStates`): states are read-only from then on and
are held by the supergraph validation context, the supergraph state and its builder.

**Subgraph validation (graphql_js visitors).** `SubgraphValidationContext<'a>` and
`SimpleValidationContext<'a>` are structs (fields = the closure locals, methods = the
closures) and are the `cx` of `graphql_js::Visitor<'a, C>`; `'a` is the lifetime of the visited
document, and the subgraph context holds `state_builder: &'a mut SubgraphStateBuilder` and owns
`type_node_info: TypeNodeInfo<'a>` (references into the document). A rule
`Rule(context): ASTVisitor` is a struct implementing `SubgraphRule<'a>` / `SimpleRule<'a>`
(`NAME` = the TS function name for `disableValidationRules`, `new(&mut cx)` = the factory body
before `return { ... }`) and `Visitor<'a, Ctx<'a>>` (`enter`/`leave` = the returned object, as a
`match node`). Struct fields are the factory's locals; add a lifetime parameter if a rule keeps
`&'a` nodes. The state builder's visitor is `SubgraphStateVisitor` (from
`builder.visitor()`); it reads the builder and `TypeNodeInfo` through `cx`.
`visit_with_type_node_info(visitor)` wraps the parallel visitor and maintains
`cx.type_node_info` (trait `HasTypeNodeInfo`). A `throw` inside a visitor callback becomes
`return cx.throw(err)`: the error is recorded, the wrapper stops the traversal for all visitors,
and `validate_subgraph` returns it. `isAvailableFederationDirective(name, node)` takes the
directive's name (`&node.name.value`). The merged object/interface definitions of
`getSubgraphObjectOrInterfaceTypes()` are owned `ObjectOrInterface` structs behind `Rc`, so
`visit_fields` can hold one while the interceptors mutate the context. `visitFields`'
interceptor closures are a `VisitFieldsInterceptor` trait object (a rule-local struct holding
the shared locals such as `isValid`), plus an `Intercepts` set saying which ones were passed
(the TS tests their presence and forwards only some of them when recursing).
`getAvailableFederationTypeAndDirectiveDefinitions` takes `&mut self` because the TS renames the
spec nodes in place.

**Supergraph state and composition.** `SupergraphStateBuilder` owns `SupergraphState`, the type
builders, `subgraph_states` and `graph_name_to_id_map`. Builders implement
`composition::common::TypeBuilder<T, S>` (`visit_subgraph_state`, `compose_supergraph_node`);
only `ObjectTypeBuilder` has state (the lazy `@requires` usage index). `composeSupergraphState`
mutates one type state while reading other maps of the same `SupergraphState`, so the two
builders that have it take exactly the map they read (`interface_types` for objects,
`object_types` for interfaces). `composeSupergraphNode` gets `ComposeHelpers { supergraph_state,
graph_name_to_id_map }` by shared reference. `build()` must keep the TS interleaving: each object
type is transformed (`createFieldsTransformer`) right before it is composed, and the object
builder's index is built on the first compose. The parameter objects of `ast.ts`
(`ObjectTypeAST`, ...) are the `*Ast` structs (optional arrays as `Vec`, optional flags as
`bool`).

**Supergraph validation.** `SupergraphValidationContext` owns the `Rc` subgraph states and the
error list. A pre-supergraph rule is `fn(&mut SupergraphValidationContext)` (see
`PRE_SUPERGRAPH_RULES`). A post rule `(context, supergraph) => SupergraphVisitorMap` is a struct
implementing `SupergraphRule<'s>` (`NAME`, `new(&mut cx, &'s SupergraphState) -> JsResult`) and
`composition::visitor::SupergraphVisitor<'s>` (callbacks get `cx` and `&'s` states and return
`JsResult`; `has_directive_field_arg` mirrors the TS presence check). Skipped rules are an empty
visitor. Rule constructors run in list order, as the TS `rules.map(...)` does.

**Satisfiability.** Arena graphs: `Graph<'s>` owns `nodes: Vec<Node>` (`NodeId` = the TS
`node.index`, one node per `nodesByTypeIndex` slot) and `edges: Vec<Edge>` (`EdgeId`; identity
comparisons compare ids); the head/tail indexes hold `EdgeId`s. Nodes refer to type states with
`NodeTypeState<'s>` (references into the `SupergraphState`). `addSubgraph` moves a subgraph
graph's nodes and edges into the merged graph, remapping ids. A graph is built with `&mut self`,
then shared as `Rc<Graph>`; what the walk mutates (edge resolvability cache, visited graph
combos, `typeChildrenCache`, the warn flag) is `RefCell`/`Cell`. `OperationPath` holds ids and
takes the graph it belongs to when it needs edges (`WalkTracker::super_path` belongs to the
supergraph graph, `paths` to the merged graph). `MoveValidator` and `PathFinder` are cheap
`Clone` structs over `Rc<Graph>` (the TS cycle MoveValidator -> PathFinder -> MoveValidator is
broken by creating the finder on demand). `lazy(() => ..)` is `helpers::Lazy<'s, T>`, shared as
`Rc<Lazy>`; lazy errors may capture `Rc` clones of graphs. `Contexts` bigint masks are
`ContextMask` (growable bit set; `Eq`/`Hash` match bigint equality); `Contexts` assigns bits
lazily through a `RefCell` and is shared as `Rc<Contexts>`. The TS `Symbol` graph ids are
`GraphId::{Supergraph, MergedGraph}`. `toString` caches (nodes, edges, moves, paths) feed only
the no-op logger; compute them on demand.

**Results.** `CompositionResult` is an enum; `CompositionSuccess` caches `supergraphSdl`,
`publicDocumentNode`, `publicSdl` in `OnceCell`s (the getters that can throw return `JsResult`).
`apply_tag_filter_on_subgraphs` works on `ServiceDefinition`s; the extra properties the TS
spreads onto the returned objects come from `apply_tag_filter_on_subgraph_type_defs`, which the
bindings use.

## Status

Everything is ported: `src/` in `crates/composition` (no `todo!()` left), the graphql-js
language layer in `crates/graphql`, and the WebAssembly bindings in `crates/composition-wasm`
with the shim in `wasm/`.

## Verification

| Check | Command | Expected |
|---|---|---|
| Build, no warnings | `cargo build --workspace` and `cargo build --release --target wasm32-unknown-unknown --workspace` | 0 warnings |
| Rust unit and differential tests | `cargo test --workspace` | all pass; includes `crates/graphql`'s differential tests against graphql-js (regenerate `crates/graphql/tests/corpus.json` with `node crates/graphql/tests/gen-corpus.mjs`) |
| Rust vs TS on every composition the test suite runs | `pnpm bench:capture`, then `cargo test -p federation_composition --release --test corpus -- --ignored` | every captured composition matches (3567/3567 on 2026-10-19) |
| The TS test suite against the WebAssembly build | `pnpm wasm:build && pnpm test:wasm --run` | 7,350 passed (the 7,343 TS tests plus 7 bridge checks in `wasm/runtime.spec.ts`), 25 skipped, 8 todo |
| The TS test suite itself | `pnpm test --run` | 7,343 passed, 25 skipped, 8 todo |
