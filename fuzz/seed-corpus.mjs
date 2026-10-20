// Writes the seed corpora of the fuzz targets to fuzz/corpus/<target>/ (gitignored):
//
// - every subgraph schema in crates/federation-composition/tests/fixtures, and
// - every group of subgraph schemas that one `compose_services(&[...])` call of the ported test
//   suite (crates/federation-composition/tests/suite) composes, with `${version}` set to the
//   latest federation version.
//
// `parse` and `build_schema` get each schema on its own, `compose` each group joined with NUL
// bytes (the target's subgraph separator), and `compose_ast` each group with a few names that
// are not GraphQL names for its renaming (see fuzz_targets/compose_ast.rs). Seed files are named by a hash of their content, so
// rerunning the script adds nothing twice. Usage: `node fuzz/seed-corpus.mjs`.
import { createHash } from "node:crypto";
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const fuzzDir = dirname(fileURLToPath(import.meta.url));
const crateDir = join(fuzzDir, "../crates/federation-composition");
const VERSION = "v2.9";
// libFuzzer's default -max_len grows from 4096 bytes; larger seeds only slow every run down.
const MAX_SEED_BYTES = 16 * 1024;

function files(dir, extension) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return files(path, extension);
    return path.endsWith(extension) ? [path] : [];
  });
}

/** The raw string literals (`r#"..."#`) passed to each `compose_services(` call in `source`. */
function composeCallGroups(source) {
  const groups = [];
  let at = 0;
  for (;;) {
    const call = source.indexOf("compose_services(", at);
    if (call < 0) return groups;
    let i = call + "compose_services(".length;
    let depth = 1;
    const literals = [];
    while (i < source.length && depth > 0) {
      if (source.startsWith('r#"', i)) {
        const end = source.indexOf('"#', i + 3);
        if (end < 0) break;
        literals.push(source.slice(i + 3, end));
        i = end + 2;
      } else if (source[i] === '"') {
        i++;
        while (i < source.length && source[i] !== '"')
          i += source[i] === "\\" ? 2 : 1;
        i++;
      } else {
        if (source[i] === "(") depth++;
        if (source[i] === ")") depth--;
        i++;
      }
    }
    // A literal that is not a schema (an expected message) is just one more seed for `parse`.
    if (literals.length > 0)
      groups.push(literals.map((s) => s.replaceAll("${version}", VERSION)));
    at = i;
  }
}

const seeds = {
  parse: new Set(),
  build_schema: new Set(),
  compose: new Set(),
  compose_ast: new Set(),
};
// Renames of the 2nd, 5th and 9th name nodes (the `compose_ast` input after `\x01`).
const RENAMES = [
  "\x02Fö o",
  "\x02\x02\x02\x02[]",
  "\x02\x02\x02\x02\x02\x02\x02\x02a b",
];
for (const path of files(join(crateDir, "tests/fixtures"), ".graphql")) {
  const sdl = readFileSync(path, "utf8");
  for (const target of Object.keys(seeds)) seeds[target].add(sdl);
}
for (const path of files(join(crateDir, "tests/suite"), ".rs")) {
  for (const group of composeCallGroups(readFileSync(path, "utf8"))) {
    for (const sdl of group) {
      seeds.parse.add(sdl);
      seeds.build_schema.add(sdl);
    }
    seeds.compose.add(group.slice(0, 4).join("\0"));
    for (const renames of RENAMES)
      seeds.compose_ast.add(group.slice(0, 4).join("\0") + "\x01" + renames);
  }
}

for (const [target, set] of Object.entries(seeds)) {
  const dir = join(fuzzDir, "corpus", target);
  mkdirSync(dir, { recursive: true });
  let written = 0;
  for (const seed of set) {
    const bytes = Buffer.from(seed, "utf8");
    if (bytes.length > MAX_SEED_BYTES) continue;
    writeFileSync(
      join(dir, createHash("sha1").update(bytes).digest("hex")),
      bytes,
    );
    written++;
  }
  console.log(`${target}: ${written} seeds in ${dir}`);
}
