import baseline from "../journal/01-baseline.md?raw";

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
    status: "in progress",
  },
  {
    title: "Benchmark TypeScript vs WebAssembly vs native",
    status: "not started",
  },
  { title: "Port the tests to Rust", status: "not started" },
];

// Newest first.
export const entries: Entry[] = [
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
