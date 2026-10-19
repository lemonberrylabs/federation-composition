// Replays the compositions captured from the test suite (bench/corpus.jsonl)
// plus the benchmark fixtures, and records per-case timings.
//
// Usage: tsx --expose-gc bench/run-corpus.ts <implementation-label> <out.json>
import { readFileSync, writeFileSync } from "node:fs";
import { DocumentNode, print } from "graphql";
import { getSubgraphs as dgs } from "../__tests__/fixtures/dgs/index.js";
import { getSubgraphs as huge } from "../__tests__/fixtures/huge-schema/index.js";
import { createStarsStuff } from "../__tests__/fixtures/stars-stuff.js";
import { composeServices } from "../src/compose.js";

type Case = {
  id: string;
  source: "tests" | "fixture";
  services: Array<{ name: string; url?: string; typeDefs: DocumentNode }>;
  internal?: { disableValidationRules?: string[] };
  success: boolean;
};

const [label = "guild-ts", outFile = "bench/results-guild-ts.json"] =
  process.argv.slice(2);

const corpus: Case[] = readFileSync("bench/corpus.jsonl", "utf8")
  .split("\n")
  .filter(Boolean)
  .map((line) => JSON.parse(line));

const fixtures: Array<[string, Array<{ name: string; typeDefs: any }>]> = [
  ["fixture:stars-stuff", Object.values(createStarsStuff())],
  ["fixture:dgs", await dgs()],
  ["fixture:huge-schema", await huge()],
];
for (const [id, services] of fixtures) {
  corpus.push({
    id,
    source: "fixture",
    services,
    success: true,
  });
}

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
