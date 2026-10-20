# Security policy

## Supported versions

Only the latest release of each package receives fixes: the crates `graphql-rs` and
`federation-composition-rs`, and the npm package `federation-composition-wasm`. Before the first
release, that is the `main` branch.

## Reporting a vulnerability

Report it privately through GitHub's
[private vulnerability reporting](https://github.com/Enrico2/federation-composition/security/advisories/new)
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
- **Input limits.** The policy on input limits (the size of documents, the number of tokens,
  the nesting depth the parser and composition accept, and how exceeding a limit is reported) is
  set by the crates' robustness work and documented in each crate's API documentation; this
  section will link to it. Today, `graphql_rs::parse` accepts graphql-js's `max_tokens` parse
  option, which bounds the work of parsing one document; upstream's TypeScript library has no
  limits of its own, and composition behaves like it.
- **Panics.** A panic on any input is a bug: report it.

Reports of excessive time or memory on crafted input are welcome under the process above.
