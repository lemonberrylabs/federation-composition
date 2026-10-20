// The upstream conformance check (`pnpm conformance [ref]`): checks out upstream at the ref in
// UPSTREAM (or the given tag or commit) and runs
//   wasm    upstream's unchanged test suite against the npm package federation-composition-wasm;
//   capture upstream's test suite on its own TypeScript, recording every composition it performs
//           and its output, plus the three benchmark fixtures, into conformance/out/corpus.jsonl;
//   native  crates/federation-composition-rs/tests/corpus.rs, which requires native Rust output to
//           be identical to the recorded TypeScript output.
// Both checks honor known-exceptions.json. Exits non-zero if any step fails.
//
// Options: --only=wasm,capture,native (default: all), --skip-build (use the npm package as
// built), --skip-install (use upstream's node_modules as installed).
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { rustEnv } from "../npm/rust-env.mjs";
import {
  checkoutUpstream,
  corpusFile,
  exceptionsFile,
  here,
  installUpstream,
  outDir,
  readExceptions,
  readUpstreamRef,
  repo,
  run,
  testKey,
  upstreamBin,
  upstreamDir,
} from "./lib.mjs";

const STEPS = ["wasm", "capture", "native"];
const { values: options, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    only: { type: "string" },
    "skip-build": { type: "boolean", default: false },
    "skip-install": { type: "boolean", default: false },
  },
});
const steps = options.only ? options.only.split(",") : STEPS;
for (const step of steps) {
  if (!STEPS.includes(step)) {
    throw new Error(
      `Unknown step "${step}"; the steps are ${STEPS.join(", ")}`,
    );
  }
}
const ref = positionals[0] ?? readUpstreamRef();
const exceptions = readExceptions();
fs.mkdirSync(outDir, { recursive: true });

const summary = { ref, commit: null, steps: {} };
const commit = checkoutUpstream(ref);
summary.commit = commit;
console.log(
  `upstream ${ref} (${commit}) in ${path.relative(repo, upstreamDir)}`,
);
if (!options["skip-install"]) {
  installUpstream();
}

if (steps.includes("wasm")) {
  summary.steps.wasm = wasmStep();
}
if (steps.includes("capture")) {
  summary.steps.capture = captureStep();
}
if (steps.includes("native")) {
  summary.steps.native = nativeStep();
}

fs.writeFileSync(
  path.join(outDir, "summary.json"),
  JSON.stringify(summary, null, 2) + "\n",
);
console.log(`\nConformance with upstream ${ref} (${commit.slice(0, 12)}):`);
for (const [step, result] of Object.entries(summary.steps)) {
  console.log(
    `  ${result.ok ? "ok  " : "FAIL"} ${step}: ${result.description}`,
  );
}
process.exit(Object.values(summary.steps).every((s) => s.ok) ? 0 : 1);

/** Upstream's test suite against the npm package. */
function wasmStep() {
  if (!options["skip-build"]) {
    const status = run(process.execPath, [path.join(repo, "npm/build.mjs")], {
      cwd: repo,
    });
    if (status !== 0) {
      return { ok: false, description: "building the npm package failed" };
    }
  }
  const reportFile = path.join(outDir, "upstream-wasm.json");
  fs.rmSync(reportFile, { force: true });
  run(
    upstreamBin("vitest"),
    [
      "run",
      "--config",
      path.join(here, "vitest.wasm.config.mjs"),
      "--reporter=default",
      "--reporter=json",
      `--outputFile.json=${reportFile}`,
    ],
    { cwd: upstreamDir },
  );
  if (!fs.existsSync(reportFile)) {
    return { ok: false, description: "vitest wrote no report" };
  }
  return evaluateReport(JSON.parse(fs.readFileSync(reportFile, "utf8")));
}

/** Pass/fail of a vitest JSON report, with the known exceptions applied. */
function evaluateReport(report) {
  const expected = new Map(
    exceptions.upstreamTests.map((e) => [`${e.file} > ${e.test}`, e]),
  );
  const seen = new Set();
  const counts = { passed: 0, failed: 0, skipped: 0, todo: 0 };
  const failures = [];
  const excepted = [];
  const stale = [];
  for (const file of report.testResults) {
    const rel = path.relative(upstreamDir, file.name).split(path.sep).join("/");
    if (file.status === "failed" && file.message) {
      failures.push({ test: `${rel} (the file)`, message: file.message });
    }
    for (const a of file.assertionResults) {
      const key = testKey(rel, a.ancestorTitles, a.title);
      seen.add(key);
      if (a.status === "passed") {
        counts.passed++;
        if (expected.has(key)) stale.push(key);
      } else if (a.status === "failed") {
        counts.failed++;
        if (expected.has(key)) {
          excepted.push(key);
        } else {
          failures.push({ test: key, message: a.failureMessages?.[0] ?? "" });
        }
      } else if (a.status === "todo") {
        counts.todo++;
      } else {
        counts.skipped++;
      }
    }
  }
  const unknown = [...expected.keys()].filter((key) => !seen.has(key));
  for (const f of failures) console.log(`FAIL ${f.test}`);
  for (const key of stale) {
    console.log(
      `passes, but is listed in ${path.relative(repo, exceptionsFile)}: ${key}`,
    );
  }
  for (const key of unknown) {
    console.log(
      `listed in ${path.relative(repo, exceptionsFile)} but not run: ${key}`,
    );
  }
  const ok =
    failures.length === 0 && stale.length === 0 && unknown.length === 0;
  return {
    ok,
    description:
      `${counts.passed} passed, ${counts.skipped} skipped, ${counts.todo} todo, ` +
      `${failures.length} failed` +
      (excepted.length ? `, ${excepted.length} known exceptions` : "") +
      (stale.length ? `, ${stale.length} stale exceptions` : "") +
      (unknown.length ? `, ${unknown.length} exceptions not run` : ""),
    counts,
    failures,
    excepted,
    stale,
    unknown,
  };
}

/** Upstream's test suite on its TypeScript, recording the corpus. */
function captureStep() {
  const raw = path.join(outDir, "raw.jsonl");
  fs.rmSync(raw, { force: true });
  const status = run(
    upstreamBin("vitest"),
    ["run", "--config", path.join(here, "vitest.capture.config.mjs")],
    { cwd: upstreamDir, env: { ...process.env, CAPTURE_FILE: raw } },
  );
  if (status !== 0) {
    return {
      ok: false,
      description: "upstream's test suite failed on its own TypeScript",
    };
  }
  const built = run(
    upstreamBin("tsx"),
    [path.join(here, "build-corpus.ts"), raw, corpusFile],
    { cwd: repo },
  );
  if (built !== 0) {
    return { ok: false, description: "building the corpus failed" };
  }
  const cases = fs.readFileSync(corpusFile, "utf8").split("\n").filter(Boolean);
  return {
    ok: true,
    description: `${cases.length} compositions -> ${path.relative(repo, corpusFile)}`,
  };
}

/** Native Rust against the recorded TypeScript outputs. */
function nativeStep() {
  const summaryFile = path.join(repo, "target/corpus-summary.json");
  fs.rmSync(summaryFile, { force: true });
  run(
    "cargo",
    [
      "test",
      "-p",
      "federation-composition-rs",
      "--release",
      "--test",
      "corpus",
      "--",
      "--ignored",
    ],
    {
      cwd: repo,
      env: {
        ...rustEnv(),
        CONFORMANCE_CORPUS: corpusFile,
        CONFORMANCE_EXCEPTIONS: exceptionsFile,
      },
    },
  );
  if (!fs.existsSync(summaryFile)) {
    return { ok: false, description: "the corpus test did not run" };
  }
  const result = JSON.parse(fs.readFileSync(summaryFile, "utf8"));
  const ok =
    result.matched + result.excepted === result.total &&
    result.stale.length === 0 &&
    result.unknown.length === 0;
  return {
    ok,
    description:
      `${result.matched}/${result.total} identical` +
      (result.excepted ? `, ${result.excepted} known exceptions` : "") +
      (result.panicked ? `, ${result.panicked} panicked` : "") +
      (result.stale.length ? `, ${result.stale.length} stale exceptions` : "") +
      (result.unknown.length
        ? `, ${result.unknown.length} exceptions not in the corpus`
        : "") +
      (result.matched + result.excepted < result.total
        ? ` (mismatches in target/corpus-mismatches.jsonl)`
        : ""),
    ...result,
  };
}
