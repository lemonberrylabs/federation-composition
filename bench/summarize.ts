// Compacts a run-corpus.ts results file into the data the research site charts.
//
// Usage: pnpm bench:summarize <results.json> <out.json>
// (node conformance/upstream-bin.mjs tsx bench/summarize.ts ...)
import { readFileSync, writeFileSync } from "node:fs";

const [input, output] = process.argv.slice(2);
if (!input || !output) {
  throw new Error("Usage: pnpm bench:summarize <results.json> <out.json>");
}

type Result = {
  id: string;
  source: "tests" | "fixture";
  subgraphs: number;
  sdlBytes: number;
  success: boolean;
  runs: number;
  medianMs: number;
  minMs: number;
};

const run = JSON.parse(readFileSync(input, "utf8"));
const results: Result[] = run.results;
// 0.1 µs: coarser rounding flips near-ties between implementations on sub-millisecond cases.
const round = (ms: number) => Math.round(ms * 10_000) / 10_000;

writeFileSync(
  output,
  JSON.stringify({
    label: run.label,
    node: run.node,
    date: run.date,
    fixtures: results
      .filter((r) => r.source === "fixture")
      .map((r) => ({
        id: r.id.replace("fixture:", ""),
        subgraphs: r.subgraphs,
        sdlBytes: r.sdlBytes,
        medianMs: round(r.medianMs),
        minMs: round(r.minMs),
        runs: r.runs,
      })),
    // [medianMs, sdlBytes, success] per test-suite composition
    cases: results
      .filter((r) => r.source === "tests")
      .map((r) => [round(r.medianMs), r.sdlBytes, r.success ? 1 : 0]),
  }) + "\n",
);
console.log(`${results.length} results -> ${output}`);
