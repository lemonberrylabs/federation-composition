import baseline from "../journal/01-baseline.md?raw";
import port from "../journal/02-port.md?raw";

export type Phase = {
  title: string;
  status: "done" | "in progress" | "not started";
};

export type Entry = {
  slug: string;
  date: string;
  milestone: number;
  title: string;
  summary: string;
  body: string;
};

export const phases: Phase[] = [
  { title: "Baseline the TypeScript implementation", status: "done" },
  {
    title: "Port to Rust, pass the existing tests via WebAssembly",
    status: "done",
  },
  {
    title: "Benchmark TypeScript vs WebAssembly vs native",
    status: "in progress",
  },
  { title: "Port the tests to Rust", status: "not started" },
];

// Newest first.
export const entries: Entry[] = [
  {
    slug: "port",
    date: "2026-10-19",
    milestone: 2,
    title: "The port: Rust passes the unchanged test suite",
    summary:
      "47.5K lines of Rust in about two hours of wall-clock time. All 7,343 TypeScript tests pass against the WebAssembly build, and native Rust output is byte-identical on 3,564 compositions.",
    body: port,
  },
  {
    slug: "baseline",
    date: "2026-10-19",
    milestone: 1,
    title: "Baseline: how fast is the TypeScript implementation?",
    summary:
      "Huge schema in 320 ms, medium in 58 ms, a typical test composition in 0.66 ms. Existing Rust parsers don't match graphql-js, so the parser gets ported too.",
    body: baseline,
  },
];
