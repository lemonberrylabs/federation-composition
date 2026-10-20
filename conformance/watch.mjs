// The upstream watcher (`pnpm conformance:watch`): compares upstream's latest release with
// UPSTREAM and, when it is newer, runs the conformance check against it and writes a porting
// task to conformance/out/issue.md: the src/ and __tests__/ changes between the two releases
// (with the Rust files they map to), the upstream tests that fail on the WebAssembly package
// and the compositions whose output changed. With --issue it opens or updates the GitHub issue
// labelled `upstream-sync` with that text (needs the gh CLI and GH_TOKEN).
//
// Options: --to=<tag> (default: the latest vX.Y.Z tag upstream), --from=<ref> (default:
// UPSTREAM), --issue, --skip-build.
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import {
  fetchUpstream,
  here,
  output,
  outDir,
  readUpstreamRef,
  repo,
  run,
  upstreamDir,
  upstreamRepo,
} from "./lib.mjs";

const LABEL = "upstream-sync";
// GitHub rejects issue bodies longer than 65,536 characters.
const MAX_BODY = 60_000;

const { values: options } = parseArgs({
  options: {
    to: { type: "string" },
    from: { type: "string" },
    issue: { type: "boolean", default: false },
    "skip-build": { type: "boolean", default: false },
  },
});

const current = options.from ?? readUpstreamRef();
fetchUpstream();
const latest = options.to ?? latestReleaseTag();
const newer = isNewer(latest, current);
setOutput("current", current);
setOutput("latest", latest);
setOutput("newer", String(newer));
if (!newer) {
  console.log(
    `UPSTREAM is ${current}; upstream's latest release is ${latest}: nothing to port`,
  );
  process.exit(0);
}
console.log(
  `upstream ${latest} is newer than UPSTREAM (${current}): running conformance against it`,
);

const status = run(
  process.execPath,
  [
    path.join(here, "run.mjs"),
    latest,
    ...(options["skip-build"] ? ["--skip-build"] : []),
  ],
  { cwd: repo },
);
const summaryFile = path.join(outDir, "summary.json");
const summary = fs.existsSync(summaryFile)
  ? JSON.parse(fs.readFileSync(summaryFile, "utf8"))
  : null;

const git = (...args) => output("git", args, { cwd: upstreamDir });
const paths = ["src", "__tests__"];
const diff = git("diff", current, latest, "--", ...paths);
fs.writeFileSync(path.join(outDir, "upstream.diff"), diff);
const changes = git("diff", "--name-status", current, latest, "--", ...paths)
  .split("\n")
  .filter(Boolean)
  .map((line) => line.split("\t"));
const log = git(
  "log",
  "--format=- %s (%h)",
  `${current}..${latest}`,
  "--",
  ...paths,
);

const title = `Port upstream ${latest} (UPSTREAM is ${current})`;
const body = issueBody();
fs.writeFileSync(path.join(outDir, "issue.md"), body);
console.log(
  `porting task -> ${path.relative(repo, path.join(outDir, "issue.md"))}`,
);
if (options.issue) {
  openOrUpdateIssue(title, path.join(outDir, "issue.md"));
}
console.log(`conformance against ${latest} exited with ${status}`);

function issueBody() {
  const repoUrl = process.env.GITHUB_REPOSITORY
    ? `${process.env.GITHUB_SERVER_URL ?? "https://github.com"}/${process.env.GITHUB_REPOSITORY}`
    : null;
  const guide = repoUrl
    ? `[docs/upstream-sync.md](${repoUrl}/blob/HEAD/docs/upstream-sync.md)`
    : "`docs/upstream-sync.md`";
  const lines = [];
  lines.push(
    `Upstream released [${latest}](${upstreamRepo}/releases/tag/${latest}); this repository matches ${current} (the \`UPSTREAM\` file).`,
    `Porting it is a self-contained task: follow ${guide}.`,
    "",
    `Compare: ${upstreamRepo}/compare/${current}...${latest}`,
    "",
    "## Conformance against " + latest,
    "",
  );
  if (summary) {
    for (const [step, result] of Object.entries(summary.steps)) {
      lines.push(
        `- ${result.ok ? "ok" : "**FAIL**"} ${step}: ${result.description}`,
      );
    }
  } else {
    lines.push(
      `The conformance run did not finish (exit status ${status}); see the workflow log.`,
    );
  }

  lines.push("", "## Changed files", "");
  if (changes.length === 0) {
    lines.push("No changes under src/ or __tests__/.");
  }
  for (const [kind, ...files] of changes) {
    const file = files.at(-1);
    const was = files.length > 1 ? ` (from \`${files[0]}\`)` : "";
    lines.push(`- ${kind} \`${file}\`${was} -> ${rustCounterpart(file)}`);
  }
  if (log.trim()) {
    lines.push("", "Commits:", "", log.trim());
  }

  const failures = summary?.steps.wasm?.failures ?? [];
  lines.push(
    "",
    `## Upstream tests failing on the WebAssembly package (${failures.length})`,
    "",
  );
  for (const f of failures.slice(0, 200)) {
    lines.push(`- \`${f.test}\``);
  }
  if (failures.length > 200) {
    lines.push(
      `- ... and ${failures.length - 200} more (conformance/out/summary.json)`,
    );
  }

  const mismatchFile = path.join(repo, "target/corpus-mismatches.jsonl");
  const mismatches = fs.existsSync(mismatchFile)
    ? fs
        .readFileSync(mismatchFile, "utf8")
        .split("\n")
        .filter(Boolean)
        .map((l) => JSON.parse(l))
    : [];
  lines.push(
    "",
    `## Compositions whose native Rust output differs from upstream's (${mismatches.length})`,
    "",
    "Each is a composition upstream's test suite performs at " +
      latest +
      " (new tests included) whose TypeScript output the Rust no longer reproduces.",
    "",
  );
  for (const m of mismatches.slice(0, 20)) {
    lines.push(
      `<details><summary><code>${m.id}</code></summary>`,
      "",
      "```json",
      JSON.stringify({ expected: m.expected, actual: m.actual }, null, 2).slice(
        0,
        2_000,
      ),
      "```",
      "</details>",
    );
  }
  if (mismatches.length > 20) {
    lines.push(
      `... and ${mismatches.length - 20} more (target/corpus-mismatches.jsonl)`,
    );
  }

  const head = lines.join("\n");
  const room = MAX_BODY - head.length - 200;
  const shown = diff.length > room ? diff.slice(0, Math.max(room, 0)) : diff;
  return [
    head,
    "",
    "## The diff of src/ and \\_\\_tests\\_\\_/",
    "",
    diff.length > room
      ? `Truncated to ${shown.length} of ${diff.length} characters; the whole diff is \`conformance/out/upstream.diff\` (a workflow artifact) and the compare link above.`
      : "",
    "",
    "<details><summary>diff</summary>",
    "",
    "```diff",
    shown,
    "```",
    "</details>",
    "",
  ].join("\n");
}

/** Where an upstream file is ported (see crates/PORTING.md). */
function rustCounterpart(file) {
  const crate = "crates/federation-composition-rs";
  const snake = (p) => p.replace(/-/g, "_").toLowerCase();
  if (file === "src/index.ts") return `\`${crate}/src/lib.rs\``;
  if (file.startsWith("src/") && file.endsWith(".spec.ts")) {
    return `unit tests in \`${crate}/${snake(file.replace(/\.spec\.ts$/, ".rs"))}\``;
  }
  if (file.startsWith("src/") && file.endsWith(".ts")) {
    const rel = file.endsWith("/index.ts")
      ? file.replace(/\/index\.ts$/, "/mod.rs")
      : file.replace(/\.ts$/, ".rs");
    return `\`${crate}/${snake(rel)}\``;
  }
  if (
    file.startsWith("__tests__/shared/") ||
    file.startsWith("__tests__/fixtures/")
  ) {
    return `the testkit, \`${crate}/tests/suite/shared/\` (fixtures: \`${crate}/tests/fixtures/\`)`;
  }
  if (file.startsWith("__tests__/") && file.endsWith(".spec.ts")) {
    const rel = file.replace(/^__tests__\//, "").replace(/\.spec\.ts$/, ".rs");
    return `\`${crate}/tests/suite/${snake(rel)}\``;
  }
  return "no Rust counterpart";
}

/** The highest vX.Y.Z tag upstream (prereleases excluded). */
function latestReleaseTag() {
  const tags = output("git", ["ls-remote", "--tags", "--refs", upstreamRepo])
    .split("\n")
    .map((line) => line.split("\t")[1]?.replace("refs/tags/", ""))
    .filter((tag) => tag && parseVersion(tag));
  if (tags.length === 0) {
    throw new Error(`${upstreamRepo} has no vX.Y.Z tags`);
  }
  return tags
    .sort((a, b) => compareVersions(parseVersion(a), parseVersion(b)))
    .at(-1);
}

function parseVersion(tag) {
  const m = /^v(\d+)\.(\d+)\.(\d+)$/.exec(tag);
  return m ? m.slice(1).map(Number) : null;
}

function compareVersions(a, b) {
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return 0;
}

/** Whether `latest` is newer than `current`: by version for two tags, by history otherwise. */
function isNewer(latest, current) {
  const [a, b] = [parseVersion(latest), parseVersion(current)];
  if (a && b) {
    return compareVersions(a, b) > 0;
  }
  // A commit: newer unless `latest` is already in its history.
  const r = run("git", ["merge-base", "--is-ancestor", latest, current], {
    cwd: upstreamDir,
  });
  return r !== 0;
}

function setOutput(name, value) {
  if (process.env.GITHUB_OUTPUT) {
    fs.appendFileSync(process.env.GITHUB_OUTPUT, `${name}=${value}\n`);
  }
}

/** Updates the open issue labelled LABEL, or opens one. */
function openOrUpdateIssue(title, bodyFile) {
  const open = JSON.parse(
    output("gh", [
      "issue",
      "list",
      "--label",
      LABEL,
      "--state",
      "open",
      "--json",
      "number",
    ]),
  );
  if (open.length > 0) {
    const number = String(open[0].number);
    output("gh", [
      "issue",
      "edit",
      number,
      "--title",
      title,
      "--body-file",
      bodyFile,
    ]);
    console.log(`updated issue #${number}`);
  } else {
    const url = output("gh", [
      "issue",
      "create",
      "--label",
      LABEL,
      "--title",
      title,
      "--body-file",
      bodyFile,
    ]).trim();
    console.log(`opened ${url}`);
  }
}
