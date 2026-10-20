// Runs a binary from the upstream checkout's node_modules/.bin with the remaining arguments,
// from the repository root: `node conformance/upstream-bin.mjs tsx bench/run-corpus.ts`.
import { repo, run, upstreamBin } from "./lib.mjs";

const [name, ...args] = process.argv.slice(2);
if (!name) {
  throw new Error("Usage: node conformance/upstream-bin.mjs <bin> [args...]");
}
process.exit(run(upstreamBin(name), args, { cwd: repo }));
