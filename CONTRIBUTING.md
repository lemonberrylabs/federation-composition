# Contributing

This repository is a Rust port of [`@theguild/federation-composition`](https://github.com/graphql-hive/federation-composition)
("upstream"). The port must behave exactly like upstream at the release recorded in
[`UPSTREAM`](UPSTREAM): same supergraph SDL byte for byte, same errors, same results from every
exported function. Read [crates/PORTING.md](crates/PORTING.md) before changing anything under
`crates/`: it is the porting guide, and its rules (never edit a test to make it pass, mirror
upstream's module structure, preserve JavaScript semantics) apply to every change.

Everyone taking part is expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md). Report
security issues privately, as [SECURITY.md](SECURITY.md) describes, not in a public issue.

## Setup

You need:

- **Rust**, through [rustup](https://rustup.rs), with the `wasm32-unknown-unknown` target. The
  stable toolchain is what CI uses; the minimum supported version is below.

  ```sh
  rustup toolchain install stable --component rustfmt,clippy --target wasm32-unknown-unknown
  ```

  On macOS, Homebrew's `cargo` has no wasm32 standard library: put rustup's toolchain first on
  `PATH` (see "Toolchain" in [crates/PORTING.md](crates/PORTING.md#toolchain), which also covers
  `DYLD_FALLBACK_LIBRARY_PATH` for linking the wasm32 build).

- **wasm-bindgen-cli** at exactly the version of the `wasm-bindgen` crate in `Cargo.lock`
  (the build checks):

  ```sh
  cargo install wasm-bindgen-cli --version "=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/"/, "", $3); print $3 }' Cargo.lock)" --locked
  ```

- Optionally **binaryen** (`wasm-opt`), for a smaller and faster WebAssembly build.
- **Node.js** at the version in [`.node-version`](.node-version), and **pnpm** at the version in
  `package.json`'s `packageManager` (`corepack enable` provides it).
- **git** and network access: the conformance check fetches upstream.

Then:

```sh
pnpm install
```

## Commands

| Command                                                                                               | What it does                                                                                                                                                                           |
| ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo build --workspace`                                                                             | Builds the crates. It must have no warnings.                                                                                                                                           |
| `cargo fmt --all --check`                                                                             | Formatting (`rustfmt.toml`, width 120). rustfmt can reindent raw strings inside macro bodies: check it never changes a string literal's value.                                         |
| `cargo clippy --workspace --all-targets -- -D warnings`                                               | Lints; must be clean.                                                                                                                                                                  |
| `pnpm conformance`                                                                                    | The upstream conformance check ([conformance/README.md](conformance/README.md)). Also checks out upstream into `conformance/upstream/`, which the next commands need.                  |
| `node crates/graphql-rs/tests/gen-corpus.mjs`<br>`node crates/graphql-rs/tests/gen-schema-corpus.mjs` | Generates graphql-rs's differential corpora from graphql-js and from the compositions upstream's tests perform. Run after `pnpm conformance` (or `pnpm conformance --only=capture`).   |
| `cargo test --workspace`                                                                              | All Rust tests: upstream's test suite ported to Rust (`crates/federation-composition-rs/tests/suite`) and graphql-rs's differential tests (which need the corpora).                    |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`                                          | API docs; must have no warnings.                                                                                                                                                       |
| `cargo build --release --target wasm32-unknown-unknown --workspace`                                   | The wasm32 build.                                                                                                                                                                      |
| `pnpm wasm:build`                                                                                     | Builds the npm package (`npm/pkg`, `npm/dist`).                                                                                                                                        |
| `pnpm test:npm`                                                                                       | The npm package's own tests (after `pnpm wasm:build`).                                                                                                                                 |
| `cargo deny check`                                                                                    | Licenses, advisories, banned crates and sources ([deny.toml](deny.toml); `cargo install cargo-deny`).                                                                                  |
| `cargo package --workspace`                                                                           | Packages each crate and builds it from its packaged files.                                                                                                                             |
| `pnpm conformance:watch --dry-run`                                                                    | The upstream watcher: whether upstream has a newer release, and the porting task it would file.                                                                                        |
| `node fuzz/seed-corpus.mjs`<br>`cargo +nightly fuzz run <target> -- -dict=fuzz/graphql.dict`          | Fuzzing ([fuzz/README.md](fuzz/README.md)): seeds the corpora, then runs one target (`parse`, `build_schema`, `compose`, `compose_ast`). Needs nightly and `cargo install cargo-fuzz`. |

On a fresh clone, run them in this order: `pnpm install`, `pnpm conformance`, the two corpus
generators, then `cargo test --workspace`.

### Minimum supported Rust version

The workspace's `rust-version` (in [Cargo.toml](Cargo.toml)) is **1.87**. It was found by
checking every target of the workspace (`cargo check --locked --workspace --all-targets`, and
the wasm32 bindings) with each toolchain's own `cargo` and `rustc`:

- 1.84 and older fail: `hashbrown` 0.17 (through `indexmap`) is an edition 2024 crate, which
  needs Cargo 1.85.
- 1.85 and 1.86 build the libraries, but not graphql-rs's differential tests, which use the
  unsigned integers' `is_multiple_of` (stable in 1.87).
- 1.87 builds everything.

The `msrv` job of [CI](.github/workflows/ci.yml) checks it on every pull request. Raising it is
fine when a change needs a newer Rust; say so in the pull request and update this section.

## Continuous integration

| Workflow                                                                                           | When                                                                | What                                                                                                                                                                                                                                                                                                                                                                          |
| -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [CI](.github/workflows/ci.yml)                                                                     | pull requests, pushes to `main`                                     | rustfmt; clippy (native and wasm32); `cargo test --workspace` on Linux, macOS and Windows (with the graphql-rs corpora generated once from upstream); the MSRV check; docs with `-D warnings`; the wasm32 build, the npm package build, its tests, and the packed tarball installed alone with Node 22 and 24 and graphql 16 and 17; cargo-deny; `cargo package --workspace`. |
| [Conformance](.github/workflows/conformance.yml)                                                   | pull requests, pushes to `main`, weekly, by hand (any upstream ref) | `pnpm conformance`. Results are uploaded as the `conformance` artifact.                                                                                                                                                                                                                                                                                                       |
| [Upstream watch](.github/workflows/upstream-watch.yml)                                             | weekly, by hand                                                     | `node conformance/watch.mjs --issue`: when upstream has a release newer than `UPSTREAM`, runs conformance against it and opens or updates the issue "Port upstream vX.Y.Z".                                                                                                                                                                                                   |
| [Fuzz](.github/workflows/fuzz.yml)                                                                 | daily, by hand                                                      | A two-minute run of every cargo-fuzz target in `fuzz/` from the seed corpus ([fuzz/README.md](fuzz/README.md)).                                                                                                                                                                                                                                                               |
| [Pages](.github/workflows/pages.yml)                                                               | pushes to `main` that change `research/`                            | Builds the research journal and deploys it to GitHub Pages.                                                                                                                                                                                                                                                                                                                   |
| [Release-plz](.github/workflows/release-plz.yml), [npm publish](.github/workflows/npm-publish.yml) | pushes to `main`, release tags                                      | Releasing (below). Off until `RELEASE_ENABLED` is set.                                                                                                                                                                                                                                                                                                                        |

## Porting upstream changes

Upstream releases are ported one at a time, following [docs/upstream-sync.md](docs/upstream-sync.md).
The weekly watcher files each one as an issue labelled `upstream-sync` with the diff, the failing
upstream tests and the changed outputs. Deliberate divergences from upstream are the owner's
decision and go in [`conformance/known-exceptions.json`](conformance/known-exceptions.json), never
in an edit to an upstream test.

## Pull requests

- One topic per pull request; the description says what changed and why, and for a port, which
  upstream release.
- CI and the conformance check must pass. Never weaken, skip or delete a test to make it pass.
- Commit messages are plain sentences in the imperative ("Port upstream v0.28.0's
  `@cost` validation"). release-plz drafts each crate's changelog from them in the release pull
  request; edit the draft there before merging if it needs it.

## Releasing

Nothing is released automatically until the owner turns it on. Releases of the Rust crates are
made by [release-plz](https://release-plz.dev) ([release-plz.toml](release-plz.toml),
[release-plz.yml](.github/workflows/release-plz.yml)); the npm package is published by
[npm-publish.yml](.github/workflows/npm-publish.yml).

| Package                               | Where                                                    | Version source                                                     |
| ------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------ |
| `graphql-rs`                          | crates.io                                                | `crates/graphql-rs/Cargo.toml`                                     |
| `federation-composition-rs`           | crates.io                                                | `crates/federation-composition-rs/Cargo.toml`                      |
| `federation-composition-wasm` (crate) | not published (`publish = false`); tagged by release-plz | `crates/federation-composition-wasm/Cargo.toml`                    |
| `federation-composition-wasm` (npm)   | npm, with provenance                                     | the crate's version: the publish job sets `npm/package.json` to it |

How a release goes:

1. Every push to `main` makes release-plz open or update a release pull request: it bumps the
   versions of the crates that changed (and of the crates that depend on them), updates each
   crate's `CHANGELOG.md`, and runs cargo-semver-checks against the last published version, so an
   API break needs a breaking version bump.
2. Merging that pull request makes release-plz publish `graphql-rs` and
   `federation-composition-rs` to crates.io, push a tag per released crate
   (`<crate>-vX.Y.Z`) and create its GitHub release.
3. The tag `federation-composition-wasm-vX.Y.Z` runs npm-publish.yml: it checks the tag against
   the crate's version, sets `npm/package.json` to that version, builds the package, runs its
   tests and the packed-tarball smoke test, and publishes it with `npm publish --provenance`
   (prerelease versions under the `next` dist-tag). It can also be run by hand for an existing
   tag.

To turn it on, the owner adds, in the repository's settings:

| Setting                                                    | Kind                                     | For                                                                                                                                                                                                                                                                                    |
| ---------------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `RELEASE_ENABLED` = `true`                                 | Actions variable                         | Both release workflows run only when it is set.                                                                                                                                                                                                                                        |
| `CARGO_REGISTRY_TOKEN`                                     | Actions secret                           | crates.io API token with the `publish-new` and `publish-update` scopes. (Once the crates exist, crates.io trusted publishing can replace it: add `id-token: write` to the release job and drop the secret.)                                                                            |
| `NPM_TOKEN`                                                | secret of the `npm` environment          | npm granular access token allowed to publish `federation-composition-wasm`. (npm trusted publishing can replace it after the first publish.)                                                                                                                                           |
| `RELEASE_PLZ_TOKEN` (optional)                             | Actions secret                           | A fine-grained token with `contents: write` and `pull-requests: write`. Tags and pull requests made with the default `GITHUB_TOKEN` do not trigger other workflows, so without it CI does not run on the release pull request and npm-publish.yml must be run by hand after a release. |
| "Allow GitHub Actions to create and approve pull requests" | Actions → General → Workflow permissions | release-plz's release pull request (when `RELEASE_PLZ_TOKEN` is not set).                                                                                                                                                                                                              |
| Pages source: "GitHub Actions"                             | Pages                                    | pages.yml.                                                                                                                                                                                                                                                                             |

The first release of each crate is a new crate on crates.io; release-plz publishes it like any
other version when `CARGO_REGISTRY_TOKEN` is set.
