import baseline from "../journal/01-baseline.md?raw";
import port from "../journal/02-port.md?raw";
import speed from "../journal/03-speed.md?raw";
import optimize from "../journal/04-optimize.md?raw";
import tests from "../journal/05-tests.md?raw";

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
    status: "done",
  },
  { title: "Optimize the port without changing its output", status: "done" },
  { title: "Port the tests to Rust", status: "done" },
];

// Newest first.
export const entries: Entry[] = [
  {
    slug: "tests",
    date: "2026-10-19",
    milestone: 5,
    title: "The tests in Rust",
    summary:
      "3,792 Rust tests, matching the TypeScript count in all 93 spec files, plus a port of graphql-js schema validation the tests rely on. Three deliberate bugs each failed 10 to 173 tests.",
    body: tests,
  },
  {
    slug: "optimize",
    date: "2026-10-19",
    milestone: 4,
    title: "Optimization: same output, 2.4× less time",
    summary:
      "A behavior-preserving pass took native Rust to 9.75× faster than TypeScript on typical compositions and 4.21× on the huge schema. WebAssembly is now 5.12× faster on typical compositions and 2.58× on the huge schema. All 3,567 outputs are still byte-identical.",
    body: optimize,
  },
  {
    slug: "speed",
    date: "2026-10-19",
    milestone: 3,
    title: "Speed: TypeScript vs WebAssembly vs native Rust",
    summary:
      "Native Rust is faster on all 3,567 cases: 3.84× on typical compositions, 1.73× on the huge schema. WebAssembly is 1.82× faster on typical compositions and no faster on large ones. Half the native time goes to memory allocation and copying carried over from the line-by-line port.",
    body: speed,
  },
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
