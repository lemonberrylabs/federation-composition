# Upstream conformance

The Rust port must behave exactly like the upstream TypeScript library,
[graphql-hive/federation-composition](https://github.com/graphql-hive/federation-composition), at the
release recorded in [`UPSTREAM`](../UPSTREAM). This directory holds the check that proves it, and
the watcher that turns a new upstream release into a porting task
([docs/upstream-sync.md](../docs/upstream-sync.md)).

```sh
pnpm install
pnpm conformance            # at the ref in UPSTREAM
pnpm conformance v0.28.0    # at another upstream tag or commit
```

It needs the Rust toolchain with the `wasm32-unknown-unknown` target and the wasm-bindgen CLI (see
[crates/PORTING.md](../crates/PORTING.md#toolchain)), and network access to fetch upstream. It
exits non-zero if any step fails, and prints one line per step:

```
Conformance with upstream v0.27.0 (6272ad80299b):
  ok   wasm: 7343 passed, 25 skipped, 8 todo, 0 failed
  ok   capture: 3567 compositions -> conformance/out/corpus.jsonl
  ok   native: 3567/3567 identical
```

## What it does

1. **Checkout.** Fetches upstream (all tags) into `conformance/upstream/` (gitignored), checks out
   the ref, and installs upstream's dependencies from its own lockfile
   (`pnpm install --frozen-lockfile --ignore-workspace`).
2. **wasm.** Builds the npm package ([`npm/`](../npm), `pnpm wasm:build`), then runs upstream's
   unchanged test suite with upstream's own vitest and
   [`vitest.wasm.config.mjs`](vitest.wasm.config.mjs). That config redirects every import of a
   module under upstream's `src/` made by a test module (`__tests__/**`, `src/**/*.spec.ts`) to
   the package's entry point for the same module, `@lemonberrylabs/federation-composition-wasm/internal/<path>`
   (the shim in `npm/src/<path>.ts`). Two self-checks keep the run honest: an import with no
   shim is an error (it never falls back to the TypeScript), and loading any module of
   upstream's implementation, by any path, is an error. The result is
   `conformance/out/upstream-wasm.json` (vitest's JSON report).
3. **capture.** Runs upstream's test suite again, this time on upstream's own TypeScript, with
   [`vitest.capture.config.mjs`](vitest.capture.config.mjs) adding
   [`capture-setup.ts`](capture-setup.ts), which wraps `composeServices` and records each call's
   input and TypeScript output. [`build-corpus.ts`](build-corpus.ts) deduplicates the calls and
   adds the three benchmark fixtures of upstream's `__tests__/fixtures` (`stars-stuff`, `dgs`,
   `huge-schema`) into the corpus, `conformance/out/corpus.jsonl`. Each line is
   `{ id, source: "tests" | "fixture", services, internal?, success, expected }`, where `id` is a
   hash of the input and `expected` is the TypeScript output (`{ supergraphSdl, publicSdl }` or
   `{ errors: [{ message, code }] }`).
4. **native.** Runs `cargo test -p federation-composition-rs --release --test corpus -- --ignored`
   ([`tests/corpus.rs`](../crates/federation-composition-rs/tests/corpus.rs)), which composes every
   corpus case with the native Rust and requires the output to equal `expected` exactly. Mismatches
   go to `target/corpus-mismatches.jsonl`, the counts to `target/corpus-summary.json`.

The summary of the run is `conformance/out/summary.json`.

Options: `--only=wasm,capture,native` runs some steps (the checkout always runs; `native` reads
the corpus the last `capture` wrote), `--skip-build` uses the npm package as built,
`--skip-install` uses upstream's `node_modules` as installed.

| Environment variable     | Default                             | Used by                                                               |
| ------------------------ | ----------------------------------- | --------------------------------------------------------------------- |
| `CONFORMANCE_CORPUS`     | `conformance/out/corpus.jsonl`      | the capture step, `tests/corpus.rs`, the benchmarks                   |
| `CONFORMANCE_EXCEPTIONS` | `conformance/known-exceptions.json` | both checks                                                           |
| `UPSTREAM_REPO`          | the upstream GitHub URL             | the checkout and the watcher                                          |
| `UPSTREAM_DIR`           | `conformance/upstream`              | `crates/graphql-rs/tests/gen-*.mjs` (graphql-js and upstream's tests) |

## Known exceptions

A deliberate divergence from upstream is listed in
[`known-exceptions.json`](known-exceptions.json), never made by editing an upstream test or a
recorded output. The file is empty now:

```json
{
  "upstreamTests": [
    {
      "file": "__tests__/supergraph/errors/EXAMPLE.spec.ts",
      "test": "'guild' > v2.3 > some test title",
      "reason": "why the port deliberately differs"
    }
  ],
  "compositions": [
    { "id": "0123456789ab", "reason": "why the port deliberately differs" }
  ]
}
```

- `upstreamTests`: a test of upstream's suite that may fail on the npm package. `file` is the
  path in upstream's repository, `test` is the test's describe titles and title joined with
  `" > "`, exactly as vitest's JSON report gives them (`ancestorTitles` and `title`; run.mjs
  prints failures in this form: `FAIL <file> > <test>`).
- `compositions`: a corpus case whose native output may differ. `id` is the case's `id`
  (in `target/corpus-mismatches.jsonl`).
- Every entry needs a non-empty `reason`.

Both checks fail on an entry that is out of date: a listed test that passes or did not run, or a
listed composition that matches or is not in the corpus. Remove such entries.

## The watcher

`pnpm conformance:watch` ([`watch.mjs`](watch.mjs)) compares upstream's latest GitHub release
(the GitHub API's latest release of graphql-hive/federation-composition) with `UPSTREAM`. When it
is newer, it runs the conformance check against it (its failures are expected: they are the work)
and writes the porting task to `conformance/out/issue.md` (the full diff of `src/` and
`__tests__/` is `conformance/out/upstream.diff`): the conformance results, every changed upstream
file with the Rust file it is ported to, the `git diff --stat` summary, the commits, the upstream
tests that fail on the npm package, the compositions whose output the Rust no longer reproduces,
and the diff.

| Option         | Effect                                                                                                                                                                      |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--issue`      | Opens or updates the issue "Port upstream vX.Y.Z" (one per release, found by that exact title among the issues labelled `upstream-sync`, open or closed) with the REST API. |
| `--dry-run`    | Prints the issue's title and body instead of writing it; with `--issue`, also says which issue it would update.                                                             |
| `--from=<ref>` | The release the port matches (default: `UPSTREAM`).                                                                                                                         |
| `--to=<tag>`   | The release to port (default: upstream's latest release).                                                                                                                   |
| `--repo=<o/r>` | The repository for the issue (default: `GITHUB_REPOSITORY`, else this checkout's `origin`).                                                                                 |
| `--skip-build` | Uses the npm package as built.                                                                                                                                              |

The token is `GITHUB_TOKEN`, else `GH_TOKEN`, else the gh CLI's (`gh auth token`). Writing the
issue needs it (with `issues: write`); a dry run and the release lookup work without it. To
preview a task: `pnpm conformance:watch --from=v0.26.3 --to=v0.27.0 --dry-run`.

## CI

- [`.github/workflows/conformance.yml`](../.github/workflows/conformance.yml): on pull requests,
  pushes to `main`, weekly and on demand (with any upstream ref): `pnpm conformance`, with the
  results uploaded as the `conformance` artifact. The Rust checks (rustfmt, clippy, the tests on
  three platforms, the npm package) are in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml).
- [`.github/workflows/upstream-watch.yml`](../.github/workflows/upstream-watch.yml): weekly and
  on demand, `node conformance/watch.mjs --issue`; the task and the diff are also uploaded as the
  `upstream-sync` artifact.
