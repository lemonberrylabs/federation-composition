## What and why

<!-- What this changes and why. For a port of an upstream release: which release, and "Closes #<upstream-sync issue>". -->

## Checklist

- [ ] No upstream test edited, and no test weakened, skipped or deleted to make it pass (crates/PORTING.md).
- [ ] Behavior matches upstream (`pnpm conformance` passes), or the divergence is the owner's decision and listed in `conformance/known-exceptions.json` with its reason.
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass.
- [ ] Public API changes are documented, and the npm shim (`npm/src`) is updated if the exported surface changed.
- [ ] `rust-version` raised (and CONTRIBUTING.md updated) if the change needs a newer Rust.
