# Security policy

## Supported versions

Only the latest release of each package receives fixes: the crates `graphql-rs` and
`federation-composition-rs`, and the npm package `@lemonberrylabs/federation-composition-wasm`. Before the first
release, that is the `main` branch.

## Reporting a vulnerability

Report it privately through GitHub's
[private vulnerability reporting](https://github.com/lemonberrylabs/federation-composition/security/advisories/new)
("Report a vulnerability" on the repository's Security tab). Do not open a public issue or pull
request for it. Include what you can of: the affected package and version, the input (subgraph
schemas) or a minimal reproduction, and what happens (crash, hang, excessive memory or time,
wrong output).

You should get an answer within a week. Once the issue is confirmed, the fix is released and a
GitHub security advisory is published, crediting you unless you ask otherwise.

A vulnerability that is also in upstream, [`@theguild/federation-composition`](https://github.com/graphql-hive/federation-composition),
because the port reproduces upstream's behavior exactly, is reported to upstream as well
(through its own security policy); we coordinate the fix with them.

## Untrusted input

Composition reads GraphQL SDL and runs validation and composition over it. If you compose
schemas that come from untrusted parties, know these properties:

- **Hash maps are not randomized.** The internal maps and sets use FxHash (`rustc-hash`), a fast
  hash function with no random seed, instead of Rust's default randomly seeded SipHash. FxHash is
  deterministic and not resistant to collisions chosen by an attacker: schemas crafted so that
  many type, field or directive names collide make each map lookup linear instead of constant
  time, so composition can take time quadratic in the number of colliding names. This is not a memory-safety problem (there is no `unsafe` code),
  but it is a way to spend CPU time. If you compose untrusted schemas, bound the size of the
  input and the time you allow a composition (for example, run it in a worker you can stop).
- **Nesting depth.** The parser rejects a document that nests more than 128 levels, counting
  list values, object values, selection sets and list types together (`{ a: [1] }` is two
  levels), with the syntax error
  `Syntax Error: Document nesting depth exceeds 128. Parsing aborted.` The limit is
  `graphql_rs::MAX_NESTING_DEPTH`; `ParseOptions::max_nesting_depth` changes it for one parse
  (raise it only with a larger stack). graphql-js has no limit of its
  own: it throws a `RangeError` when it runs out of JavaScript stack. The limit also applies to
  the text composition parses: a `@key`, `@requires` or `@provides` field set that nests more
  than 127 selection sets is a composition error with that message, where upstream composes it.
  No upstream test comes near these depths.
- **Documents built in code or deserialized** can nest deeper than the parser allows. Every
  public function that takes a document and returns a `Result` checks it first and returns the
  `GraphQLError` `AST nesting depth exceeds 128.`: in `graphql-rs`, `build_ast_schema` and
  `assert_valid_sdl`; in `federation-composition-rs`, `compose_services`, `validate`,
  `validate_subgraph`, `extract_link_implementations`, `transform_supergraph_to_public_schema`,
  `sort_sdl`, `compose_schema_contract`, `add_inaccessible_to_unreachable_types`,
  `apply_tag_filter_on_subgraphs` and `CompositionSuccess::public_document_node`/`public_sdl`.
  The functions that cannot fail (`print`, `visit`, `visit_edit`, `validate_sdl`, and
  `CompositionSuccess::supergraph_sdl` of a success made with
  `create_composition_success_read_cache_container`) recurse once per level: check such a
  document with `graphql_rs::check_nesting_depth` before calling them. Dropping a deeply nested
  AST recurses too, as for any tree of `Box`es in Rust.
- **Stack.** Within the limit, the deepest case (list types nested 128 levels in a subgraph)
  composes in about 260 KiB of stack in a release build and 850 KiB in a debug build, so it fits
  the 2 MiB stack of a spawned thread and WebAssembly's 1 MiB. The searches over a schema's
  types (the satisfiability walk and its graph, cycles of input objects and of default values,
  referenced types) use explicit stacks instead of recursion, so a long chain of types (tested
  with 5,000 to 20,000) does not overflow the stack.
- **The npm package** (`@lemonberrylabs/federation-composition-wasm`) receives its arguments as JSON. An argument
  that nests more than 448 arrays and objects (3 × 128 + 64, enough for any document within
  the limit) throws
  `composition-wasm: invalid <argument> argument: nested deeper than 448 levels` before it is
  parsed, and a document in it is then checked as above (a `GraphQLError`
  `AST nesting depth exceeds 128.`). Within the limit, the package behaves like upstream.
- **Size.** The number of subgraphs, types, fields and tokens is not limited (except by
  `graphql_rs::parse_with_options`'s `max_tokens`, graphql-js's option, which bounds the work of
  parsing one document); composition time and memory grow with the schemas. If you compose
  untrusted schemas, bound the size of the input and the time you allow a composition (for
  example, run it in a worker you can stop).
- **Panics.** No input makes the crates panic, other than the two cases above that are the
  caller's to avoid: an infallible function given a document deeper than the limit, and an
  edit that `visit_edit` cannot represent (a visitor that deletes a required node or replaces
  one with a node of another kind, documented on the function). Where upstream throws, the port returns its error with the same
  message. Any other panic is a bug: report it. In the npm package a panic traps the
  WebAssembly instance; the package throws an `Error` with the panic message and loads a fresh
  instance.

Reports of excessive time or memory on crafted input are welcome under the process above.

## Fuzzing

`fuzz/` has [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets for the public entry
points ([fuzz/README.md](fuzz/README.md)): `parse` (every parsing function, and the property that
printing a parsed document and parsing the result gives the same AST), `build_schema` (schema
building and validation), `compose` (composition of up to four fuzzed subgraphs, then the public
schema, `sortSDL` and a schema contract) and `compose_ast` (composition of documents with names the
parser cannot produce, as built in code or sent from JavaScript). They start from a corpus of the
subgraphs in the test fixtures and in the ported test suite. The
[Fuzz workflow](.github/workflows/fuzz.yml) runs each target daily. Every crash found becomes a
regression test in `crates/*/tests/robustness.rs`.
