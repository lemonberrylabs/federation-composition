// Shared by the conformance scripts: paths, the UPSTREAM file, the upstream checkout and the
// known-exceptions file.
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const here = path.dirname(fileURLToPath(import.meta.url));
export const repo = path.resolve(here, "..");
/** The upstream checkout (gitignored). */
export const upstreamDir = path.join(here, "upstream");
/** Everything the checks write (gitignored). */
export const outDir = path.join(here, "out");
/** The corpus of compositions with upstream's TypeScript outputs (see corpus.rs). */
export const corpusFile =
  process.env.CONFORMANCE_CORPUS ?? path.join(outDir, "corpus.jsonl");
export const exceptionsFile =
  process.env.CONFORMANCE_EXCEPTIONS ??
  path.join(here, "known-exceptions.json");
export const upstreamRepo =
  process.env.UPSTREAM_REPO ??
  "https://github.com/graphql-hive/federation-composition";

/** The upstream ref recorded in UPSTREAM: its first line that is not blank or a `#` comment. */
export function readUpstreamRef() {
  const text = fs.readFileSync(path.join(repo, "UPSTREAM"), "utf8");
  const ref = text
    .split("\n")
    .map((line) => line.trim())
    .find((line) => line !== "" && !line.startsWith("#"));
  if (!ref) {
    throw new Error("UPSTREAM has no ref (see docs/upstream-sync.md)");
  }
  return ref;
}

/** Runs a command with inherited stdio; returns its exit status. */
export function run(cmd, args, options = {}) {
  console.log(
    `$ ${options.cwd ? `(cd ${path.relative(repo, options.cwd) || "."}) ` : ""}${cmd} ${args.join(" ")}`,
  );
  const r = spawnSync(cmd, args, { stdio: "inherit", ...options });
  if (r.error) throw r.error;
  return r.status ?? 1;
}

/** Runs a command and returns its stdout; throws if it fails. */
export function output(cmd, args, options = {}) {
  const r = spawnSync(cmd, args, {
    encoding: "utf8",
    maxBuffer: 1 << 30,
    ...options,
  });
  if (r.error) throw r.error;
  if (r.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} failed:\n${r.stderr}`);
  }
  return r.stdout;
}

function mustRun(cmd, args, options) {
  const status = run(cmd, args, options);
  if (status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} exited with ${status}`);
  }
}

/**
 * Checks out upstream at `ref` (a tag or a commit) into conformance/upstream, with all tags,
 * and returns the commit. Untracked files are removed; ignored ones (node_modules) are kept.
 */
export function checkoutUpstream(ref) {
  fetchUpstream();
  mustRun(
    "git",
    ["checkout", "--quiet", "--force", "--detach", `${ref}^{commit}`],
    { cwd: upstreamDir },
  );
  mustRun("git", ["clean", "--quiet", "-fd"], { cwd: upstreamDir });
  return output("git", ["rev-parse", "HEAD"], { cwd: upstreamDir }).trim();
}

/** Creates conformance/upstream if needed and fetches upstream's history and tags into it. */
export function fetchUpstream() {
  if (!fs.existsSync(path.join(upstreamDir, ".git"))) {
    fs.mkdirSync(upstreamDir, { recursive: true });
    mustRun("git", ["init", "--quiet"], { cwd: upstreamDir });
    mustRun("git", ["remote", "add", "origin", upstreamRepo], {
      cwd: upstreamDir,
    });
  }
  mustRun("git", ["remote", "set-url", "origin", upstreamRepo], {
    cwd: upstreamDir,
  });
  mustRun("git", ["fetch", "--quiet", "--tags", "--force", "origin"], {
    cwd: upstreamDir,
  });
}

/** Installs upstream's dependencies from its lockfile, outside this workspace. */
export function installUpstream() {
  mustRun(
    "pnpm",
    ["install", "--frozen-lockfile", "--ignore-workspace", "--reporter=silent"],
    { cwd: upstreamDir },
  );
}

/** A binary from upstream's node_modules/.bin (vitest, tsx). */
export function upstreamBin(name) {
  const bin = path.join(upstreamDir, "node_modules/.bin", name);
  if (!fs.existsSync(bin)) {
    throw new Error(
      `${path.relative(repo, bin)} is missing: run \`pnpm conformance\` first (it checks out and installs upstream)`,
    );
  }
  return bin;
}

/**
 * The known exceptions (format in conformance/README.md): deliberate divergences from
 * upstream, as `{ upstreamTests: [{ file, test, reason }], compositions: [{ id, reason }] }`.
 */
export function readExceptions() {
  const json = JSON.parse(fs.readFileSync(exceptionsFile, "utf8"));
  for (const key of ["upstreamTests", "compositions"]) {
    if (!Array.isArray(json[key])) {
      throw new Error(`${exceptionsFile}: "${key}" must be an array`);
    }
    for (const entry of json[key]) {
      if (typeof entry.reason !== "string" || entry.reason === "") {
        throw new Error(
          `${exceptionsFile}: every entry needs a "reason": ${JSON.stringify(entry)}`,
        );
      }
    }
  }
  return json;
}

/** The key of an upstream test in known-exceptions.json and in reports. */
export function testKey(file, ancestorTitles, title) {
  return `${file} > ${[...ancestorTitles, title].join(" > ")}`;
}
