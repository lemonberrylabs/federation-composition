The test suite now exists in Rust. Every TypeScript test that exercises this library has a Rust counterpart: 3,792 passing and 17 ignored, the same counts as the TypeScript, file by file. The Rust library can now be tested without Node.js or the WebAssembly bridge.

<div data-chart="tests-tiles"></div>

## What was ported

The TypeScript harness runs most tests twice: once against Apollo's `@apollo/composition` for comparison, and once against this library. Only the second half tests this library, so only it was ported. That leaves 3,792 passing tests and 17 skipped or todo out of the 7,376 in the TypeScript run. Each Rust test corresponds to one TypeScript test for one federation version, so the counts can be compared file by file. All 93 spec files match.

The port added two parts:

- **The suite**, 41,414 lines of Rust in `crates/composition/tests/suite`. It builds as one test binary with one module per TypeScript spec file, plus a testkit that mirrors `__tests__/shared`.
- **Schema building in `crates/graphql`**, 6,443 lines. It ports graphql-js's `buildSchema` and `assertValidSchema`, and the 15 SDL validation rules they run.

Expected values were copied as they are: error messages, SDL, and inline snapshots, with the same indentation stripping as `strip-indent` and Vitest. The testkit's snapshot and indentation helpers were checked against the npm packages the TypeScript uses.

## Two checks the TypeScript harness adds

The TypeScript test helpers do more than call the library, and both extra checks matter:

- **`assertCompositionSuccess` builds and validates the supergraph and public schemas with graphql-js.** Rust had no GraphQL type system, because phase 1 only ported graphql-js's language layer. One agent ported `buildSchema` and `assertValidSchema` and compared them with graphql-js on 1,966 documents: 940 supergraph and public schemas from the corpus, 834 documents from the tests, and 192 written to reach every error message. All outcomes match: the same schemas build, the same ones are invalid, and the error messages are identical.
- **Every Guild supergraph is checked by Apollo's gateway code.** Its federation-specific part needs Apollo's library and cannot run in Rust. Its GraphQL-validity part can, so the Rust testkit runs it with the graphql-js port and raises Apollo's message. One test depends on it: it expects composition of a conflicting `@oneOf` definition to throw "The schema is not a valid GraphQL schema", caused by a missing required argument on `@oneOf`. Without this check that test could only have been ignored.

## How it was done

- **Testkit first.** One agent wrote the testkit, created an empty module for each spec file so no two agents edited the same file, and ported one spec file as an example.
- **Nine porters in parallel**, each in its own git worktree with a group of spec files. In parallel with them, one agent ported graphql-js schema building.
- **An independent reviewer for each group.** As soon as a porter finished, a reviewer compared the group test by test against the TypeScript, looking for missing tests and weaker assertions. The reviewers found no weakened assertions. Three of them raised the then-empty schema-validity check, which the merge filled in.
- **Merge and verify.** One agent merged the ten branches without conflicts and connected the schema validation. It also confirmed that the existing verifications still pass: 7,350 tests on WebAssembly, 3567/3567 identical compositions, and the graphql-js comparisons.

No library code changed. Every ported test passed against the existing Rust, which is expected: the same Rust already passed these tests through WebAssembly.

## Does the suite catch mistakes?

A test suite that passes proves little unless it fails when the code is wrong. Three deliberate bugs were introduced, one at a time, and reverted:

| Change to the library                                         | Rust tests that fail |
| ------------------------------------------------------------- | -------------------: |
| One word removed from the `ENUM_VALUE_MISMATCH` error message |                   10 |
| The `override:` argument dropped from printed `@join__field`  |                   85 |
| One condition inverted in the satisfiability rule             |                  173 |

I also compared a sample of ported tests with the TypeScript by hand. The expected text was identical.

## Running the tests

The Rust suite runs in about 0.5 seconds in a release build (`cargo test --release --test suite`), plus the time to compile it. The TypeScript suite takes about 9 seconds with Vitest, but that runs the Apollo half as well and spends most of its time loading modules, so the two numbers are not a like-for-like comparison.

## Time

Phase 3 took 50 minutes of wall-clock time and about 3.8 million subagent tokens:

| Step                                        | Time          |
| ------------------------------------------- | ------------- |
| Testkit and example                         | 16 min        |
| graphql-js schema building, in parallel     | 39 min        |
| Nine porters, in parallel                   | 3–13 min each |
| Nine reviewers, each right after its porter | 2–5 min each  |
| Merge and verification                      | 11 min        |

## Where this leaves the project

All four phases are done. The library is in Rust, the unchanged TypeScript suite passes against its WebAssembly build, native Rust is 9.75× faster than TypeScript on typical compositions, and the tests are in Rust too. The TypeScript suite and the corpus differential remain useful as cross-checks for as long as the TypeScript implementation is kept.
