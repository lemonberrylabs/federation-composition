// Deduplicates the compositions recorded by capture-setup.ts into bench/corpus.jsonl.
import { createHash } from "node:crypto";
import { readFileSync, rmSync, writeFileSync } from "node:fs";

const RAW = "bench/raw.jsonl";
const CORPUS = "bench/corpus.jsonl";

const unique = new Map<string, unknown>();
for (const line of readFileSync(RAW, "utf8").split("\n")) {
  if (!line) {
    continue;
  }
  const entry = JSON.parse(line);
  const id = createHash("sha1")
    .update(JSON.stringify([entry.services, entry.internal]))
    .digest("hex")
    .slice(0, 12);
  if (!unique.has(id)) {
    unique.set(id, { id, source: "tests", ...entry });
  }
}

writeFileSync(
  CORPUS,
  [...unique.values()].map((c) => JSON.stringify(c)).join("\n") + "\n",
);
rmSync(RAW);
console.log(`${unique.size} unique compositions -> ${CORPUS}`);
