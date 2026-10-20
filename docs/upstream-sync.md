# Porting a new upstream release

This repository is a Rust port of
[graphql-hive/federation-composition](https://github.com/graphql-hive/federation-composition)
("upstream"). The file [`UPSTREAM`](../UPSTREAM) records the upstream release the Rust matches, and
the conformance check ([conformance/README.md](../conformance/README.md)) proves it matches: upstream's
unchanged test suite passes against the npm package, and native Rust output is byte-identical to
upstream's TypeScript output on every composition that suite performs.

## The `UPSTREAM` file

The first line that is neither blank nor a `#` comment is the upstream ref: a release tag
(`v0.27.0`), or a full commit hash if the port matches no release exactly. Anything else is
comments, conventionally one line saying what the file is:

```
v0.27.0
# graphql-hive/federation-composition release the Rust port matches (format: docs/upstream-sync.md)
```

v0.27.0 was established on 2026-10-19: this repository was forked at upstream commit `6272ad8`,
which is the `v0.27.0` tag (and upstream's `main` at the time), and the TypeScript `src/` and
`__tests__/` it carried were identical to the tag (`git diff v0.27.0 -- src __tests__` was empty)
when they were removed.

## When a release comes out

The weekly [upstream watcher](../.github/workflows/upstream-watch.yml) notices a release newer than
`UPSTREAM`, runs the conformance check against it and opens (or updates) an issue labelled
`upstream-sync`, "Port upstream vX.Y.Z". The same task can be produced locally:

```sh
pnpm conformance:watch          # writes conformance/out/issue.md and conformance/out/upstream.diff
```

The task lists the conformance results at the new release, every changed file under upstream's
`src/` and `__tests__/` with the Rust file it is ported to, the upstream tests that fail on the npm
package, the compositions whose output the Rust no longer reproduces, and the diff.

## Procedure

Work on a branch. Read [crates/PORTING.md](../crates/PORTING.md) first: it is the porting guide
(module mapping, JavaScript semantics to preserve, design decisions, the test-porting rules), and
everything below follows it.

1. **Read the task.** Start from the watcher issue (or `conformance/out/issue.md`). Check the
   upstream changelog (`CHANGELOG.md` in the checkout, and the release notes) for intent, and
   `conformance/out/upstream.diff` for the full change. Check out the new release locally:

   ```sh
   pnpm conformance vX.Y.Z      # leaves upstream at vX.Y.Z in conformance/upstream
   ```

   Its failures are the work list. Read the old and new upstream files side by side with
   `git -C conformance/upstream diff <old> vX.Y.Z -- src __tests__`.

2. **Port the changed `src/` files to their mirrored Rust modules.** `src/a/b-c.ts` is
   `crates/federation-composition/src/a/b_c.rs` (`index.ts` is `mod.rs`, `src/index.ts` is
   `lib.rs`). Port the change with the same structure and control flow as the TypeScript, without
   refactoring or "fixing" upstream behavior (leave a `// NOTE(port):` where the TS looks wrong).
   A new file gets a new module in the same place; a deleted file's module is deleted. If upstream
   now uses more of graphql-js, port that part into `crates/graphql-core` (mirroring graphql-js's
   files) and extend its differential tests.

3. **Update the bindings if the public surface changed.** A new or changed export of a module that
   tests import needs the export in `crates/federation-composition-wasm/src/exports.rs` and the
   shim in `npm/src/<same path>.ts`. A test importing a module with no shim fails the conformance
   run with "No WebAssembly shim for src/<path>": add `npm/src/<path>.ts`. If the public entry
   (`src/index.ts`) changed, update `npm/src/index.ts` and the export list in
   `npm/test/smoke.test.mjs`.

4. **Port new and changed upstream tests into `crates/federation-composition/tests/suite`.**
   `__tests__/x/Y-Z.spec.ts` is `tests/suite/x/y_z.rs` (and a new file is added to the module tree
   in `tests/suite/main.rs`); tests in `src/**/*.spec.ts` become unit tests in the module. Follow
   "Rust test suite" in PORTING.md exactly: guild side only, one `#[test]` per TS test per
   federation version, `test.skip`/`test.todo` as `#[ignore]`, names from the TS titles, expected
   strings copied verbatim, no assertion weaker than the TS. Then check the counting rule: for each
   spec file, the Rust module's passing count equals upstream's guild-side passed count and its
   ignored count equals skipped + todo; update the totals in PORTING.md. Changes to
   `__tests__/shared` go into the testkit (`tests/suite/shared`); changed `.graphql` fixtures of
   `__tests__/fixtures/{dgs,huge-schema}` are copied into `tests/fixtures/`.

5. **Run everything until it is clean.**

   ```sh
   pnpm conformance vX.Y.Z                       # upstream's tests on wasm; corpus differential
   pnpm test:npm                                 # the npm package's own tests
   node crates/graphql-core/tests/gen-corpus.mjs   # graphql-js differential corpora, from the checkout
   node crates/graphql-core/tests/gen-schema-corpus.mjs
   cargo test --workspace
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo build --release --target wasm32-unknown-unknown --workspace
   ```

   Every upstream test passes on the npm package with the same pass/skip/todo counts as upstream's
   suite on its own TypeScript (the capture step runs that suite and fails if it does not pass),
   and every composition is identical. Never edit an upstream test or a recorded output. If a
   divergence from upstream is deliberate (the owner decides), list it in
   `conformance/known-exceptions.json` with the reason; the check fails on entries that are no
   longer needed, so remove them as soon as they are.

6. **Bump `UPSTREAM`** to `vX.Y.Z`, and update the dates and counts in PORTING.md's "Verification"
   table. Run `pnpm conformance` once more with no argument: it now checks the new release.

7. **Commit and open the pull request.** Say which upstream release it ports and link the
   `upstream-sync` issue with `Closes #<number>` so it closes when the pull request merges. The conformance workflow runs
   the same checks on the pull request.

## When upstream has not released but `main` moved

Only releases are ported. To look ahead, run `pnpm conformance <commit>` or
`pnpm conformance:watch --to=<tag>`; neither changes `UPSTREAM`.
