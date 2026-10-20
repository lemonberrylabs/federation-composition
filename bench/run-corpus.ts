// Replays the conformance corpus (conformance/out/corpus.jsonl: the compositions captured from
// upstream's test suite plus the benchmark fixtures) and records per-case timings.
//
// Usage: pnpm bench:corpus <implementation> [out.json]
// (node conformance/upstream-bin.mjs tsx --expose-gc bench/run-corpus.ts ...), where
// <implementation> is "guild-ts" (upstream's TypeScript in conformance/upstream/src) or
// "guild-wasm" (the npm package federation-composition-wasm as built in npm/dist, so AST
// conversion across the boundary is included). Run `pnpm conformance` first; it checks out
// upstream, builds the package and records the corpus.
// Native Rust uses the same protocol: crates/federation-composition-rs/examples/corpus_bench.rs.
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import type { DocumentNode } from "graphql";

// graphql-js from the upstream checkout (this workspace has no copy of its own).
const { print } = createRequire(
  new URL("../conformance/upstream/package.json", import.meta.url),
)("graphql") as typeof import("graphql");
const corpusFile =
  process.env.CONFORMANCE_CORPUS ??
  new URL("../conformance/out/corpus.jsonl", import.meta.url).pathname;

type Case = {
  id: string;
  source: "tests" | "fixture";
  services: Array<{ name: string; url?: string; typeDefs: DocumentNode }>;
  internal?: { disableValidationRules?: string[] };
  success: boolean;
};

const [label = "guild-ts", outFile = `bench/results-${label}.json`] =
  process.argv.slice(2);

const implementations = {
  "guild-ts": () => import("../conformance/upstream/src/compose.js"),
  "guild-wasm": () => import("../npm/dist/compose.js"),
};
if (!(label in implementations)) {
  throw new Error(`Unknown implementation "${label}"`);
}
const { composeServices } =
  await implementations[label as keyof typeof implementations]();

const corpus: Case[] = readFileSync(corpusFile, "utf8")
  .split("\n")
  .filter(Boolean)
  .map((line) => JSON.parse(line));

// The timed region is composeServices() only, on already-parsed ASTs.
const prepared = corpus.map((c) => ({
  ...c,
  sdlBytes: c.services.reduce((n, s) => n + print(s.typeDefs).length, 0),
}));

const gc = (globalThis as any).gc as undefined | (() => void);
const MIN_RUNS = 5;
const MIN_TIME_MS = 50;
const MAX_RUNS = 200;

const results = [];
const started = performance.now();
for (const c of prepared) {
  const run = () => composeServices(c.services, c.internal);
  const first = run();
  const success = !("errors" in first && first.errors);
  if (success !== c.success) {
    throw new Error(`${c.id}: expected success=${c.success}, got ${success}`);
  }
  for (let i = 0; i < 3; i++) run();
  gc?.();
  const samples: number[] = [];
  const caseStart = performance.now();
  while (
    samples.length < MIN_RUNS ||
    (performance.now() - caseStart < MIN_TIME_MS && samples.length < MAX_RUNS)
  ) {
    const t = performance.now();
    run();
    samples.push(performance.now() - t);
  }
  samples.sort((a, b) => a - b);
  results.push({
    id: c.id,
    source: c.source,
    subgraphs: c.services.length,
    sdlBytes: c.sdlBytes,
    success,
    runs: samples.length,
    medianMs: samples[Math.floor(samples.length / 2)],
    minMs: samples[0],
  });
}

writeFileSync(
  outFile,
  JSON.stringify(
    {
      label,
      node: process.version,
      date: new Date().toISOString(),
      wallMs: performance.now() - started,
      results,
    },
    null,
    2,
  ),
);
console.log(
  `${label}: ${results.length} cases in ${((performance.now() - started) / 1000).toFixed(1)}s -> ${outFile}`,
);
