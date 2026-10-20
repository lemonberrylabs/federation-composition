// The upstream watcher (`pnpm conformance:watch`): compares upstream's latest release with
// UPSTREAM and, when it is newer, runs the conformance check against it and writes a porting
// task to conformance/out/issue.md: the src/ and __tests__/ changes between the two releases
// (with the Rust files they map to), the upstream tests that fail on the WebAssembly package
// and the compositions whose output changed. With --issue it opens or updates the GitHub issue
// for that release (one per release, found by its title "Port upstream vX.Y.Z", labelled
// `upstream-sync`) through the GitHub REST API; with --dry-run it prints the issue it would
// write instead.
//
// Options:
//   --to=<tag>      the release to port (default: upstream's latest GitHub release)
//   --from=<ref>    the release the port matches (default: UPSTREAM)
//   --issue         open or update the GitHub issue (needs GITHUB_TOKEN with issues: write)
//   --dry-run       print the issue's title and body instead of writing it
//   --repo=<o/r>    the repository for the issue (default: GITHUB_REPOSITORY, else origin)
//   --skip-build    use the npm package as built
// The token is GITHUB_TOKEN, else GH_TOKEN, else `gh auth token` if the gh CLI is logged in. It
// is optional for --dry-run and for finding the latest release (unauthenticated requests are
// rate limited).
import { spawnSync } from "node:child_process";
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
    "dry-run": { type: "boolean", default: false },
    repo: { type: "string" },
    "skip-build": { type: "boolean", default: false },
  },
});

const token = githubToken();
const current = options.from ?? readUpstreamRef();
fetchUpstream();
const latest = options.to ?? (await latestRelease());
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
const stat = git("diff", "--stat=100", current, latest, "--", ...paths);
const log = git(
  "log",
  "--format=- %s (%h)",
  `${current}..${latest}`,
  "--",
  ...paths,
);

// The title identifies the issue of a release: a later run for the same release updates it.
const title = `Port upstream ${latest}`;
const body = issueBody();
fs.writeFileSync(path.join(outDir, "issue.md"), body);
console.log(
  `porting task -> ${path.relative(repo, path.join(outDir, "issue.md"))}`,
);
if (options["dry-run"]) {
  console.log(
    `\n--- dry run: the issue "${title}" ---\n\n${body}\n--- end ---`,
  );
  if (options.issue) {
    const existing = token ? await findIssue(targetRepo(), title) : null;
    console.log(
      existing
        ? `would update ${existing.html_url}`
        : `would open a new issue in ${targetRepo()}${token ? "" : " (no token: did not look for an existing one)"}`,
    );
  }
} else if (options.issue) {
  await openOrUpdateIssue(targetRepo(), title, body);
}
console.log(`conformance against ${latest} exited with ${status}`);

function issueBody() {
  let target = null;
  try {
    target = targetRepo();
  } catch {
    // Not on GitHub: the guide is named by its path.
  }
  const repoUrl = target
    ? `${process.env.GITHUB_SERVER_URL ?? "https://github.com"}/${target}`
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
  if (stat.trim()) {
    lines.push("", "```", stat.trimEnd(), "```");
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

/** The tag of upstream's latest GitHub release (GitHub excludes drafts and prereleases). */
async function latestRelease() {
  const m = /github\.com[/:]([^/]+\/[^/]+?)(?:\.git)?\/?$/.exec(upstreamRepo);
  if (!m) {
    throw new Error(
      `UPSTREAM_REPO ${upstreamRepo} is not a GitHub repository; pass --to=<tag>`,
    );
  }
  const release = await github("GET", `/repos/${m[1]}/releases/latest`);
  if (!parseVersion(release.tag_name)) {
    throw new Error(
      `upstream's latest release has the tag ${release.tag_name}, not vX.Y.Z; pass --to=<tag>`,
    );
  }
  return release.tag_name;
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

/** The repository the issue goes to: --repo, GITHUB_REPOSITORY, or this checkout's origin. */
function targetRepo() {
  if (options.repo) return options.repo;
  if (process.env.GITHUB_REPOSITORY) return process.env.GITHUB_REPOSITORY;
  const origin = output("git", ["remote", "get-url", "origin"], {
    cwd: repo,
  }).trim();
  const m = /github\.com[/:]([^/]+\/[^/]+?)(?:\.git)?\/?$/.exec(origin);
  if (!m) {
    throw new Error(
      `origin (${origin}) is not on GitHub; pass --repo=<owner/name>`,
    );
  }
  return m[1];
}

function githubToken() {
  const env = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
  if (env) return env;
  const r = spawnSync("gh", ["auth", "token"], { encoding: "utf8" });
  return r.status === 0 ? r.stdout.trim() : null;
}

/** A GitHub REST API call; returns the parsed response, or null for a 404 when `allow404`. */
async function github(method, route, body, { allow404 = false } = {}) {
  const api = process.env.GITHUB_API_URL ?? "https://api.github.com";
  const res = await fetch(`${api}${route}`, {
    method,
    headers: {
      accept: "application/vnd.github+json",
      "x-github-api-version": "2022-11-28",
      "user-agent": "federation-composition-rs-upstream-watch",
      ...(token ? { authorization: `Bearer ${token}` } : {}),
      ...(body ? { "content-type": "application/json" } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (allow404 && res.status === 404) return null;
  if (!res.ok) {
    throw new Error(
      `GitHub API ${method} ${route}: ${res.status} ${await res.text()}`,
    );
  }
  return res.status === 204 ? null : res.json();
}

/** The issue (open or closed) labelled LABEL whose title is exactly `title`, or null. */
async function findIssue(target, title) {
  for (let page = 1; ; page++) {
    const issues = await github(
      "GET",
      `/repos/${target}/issues?labels=${LABEL}&state=all&per_page=100&page=${page}`,
    );
    const found = issues.find((i) => !i.pull_request && i.title === title);
    if (found) return found;
    if (issues.length < 100) return null;
  }
}

/** Updates the issue of this release, or opens it (creating the label if needed). */
async function openOrUpdateIssue(target, title, body) {
  if (!token) {
    throw new Error(
      "--issue needs GITHUB_TOKEN (or GH_TOKEN, or `gh auth login`)",
    );
  }
  const existing = await findIssue(target, title);
  if (existing) {
    await github("PATCH", `/repos/${target}/issues/${existing.number}`, {
      body,
    });
    console.log(`updated ${existing.html_url}`);
    return;
  }
  const label = await github("GET", `/repos/${target}/labels/${LABEL}`, null, {
    allow404: true,
  });
  if (!label) {
    await github("POST", `/repos/${target}/labels`, {
      name: LABEL,
      color: "0E8A16",
      description: "Porting a new upstream release",
    });
  }
  const created = await github("POST", `/repos/${target}/issues`, {
    title,
    body,
    labels: [LABEL],
  });
  console.log(`opened ${created.html_url}`);
}
