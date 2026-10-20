// Deduplicates the compositions recorded by capture-setup.ts into the corpus
// (conformance/out/corpus.jsonl), then appends the three benchmark fixtures of
// upstream's __tests__/fixtures with their TypeScript output.
//
// Usage (run.mjs does this): tsx conformance/build-corpus.ts <raw.jsonl> <corpus.jsonl>
import { createHash } from "node:crypto";
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { getSubgraphs as dgs } from "./upstream/__tests__/fixtures/dgs/index.js";
import { getSubgraphs as huge } from "./upstream/__tests__/fixtures/huge-schema/index.js";
import { createStarsStuff } from "./upstream/__tests__/fixtures/stars-stuff.js";
import {
  composeServices,
  compositionHasErrors,
} from "./upstream/src/compose.js";

const [RAW, CORPUS] = process.argv.slice(2);
if (!RAW || !CORPUS) {
  throw new Error(
    "Usage: tsx conformance/build-corpus.ts <raw.jsonl> <corpus.jsonl>",
  );
}

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

const fixtures: Array<[string, Array<{ name: string; typeDefs: any }>]> = [
  ["fixture:stars-stuff", Object.values(createStarsStuff())],
  ["fixture:dgs", await dgs()],
  ["fixture:huge-schema", await huge()],
];
for (const [id, services] of fixtures) {
  const result = composeServices(services);
  if (compositionHasErrors(result)) {
    throw new Error(`${id} failed to compose`);
  }
  unique.set(id, {
    id,
    source: "fixture",
    services: services.map((s) => ({ name: s.name, typeDefs: s.typeDefs })),
    success: true,
    expected: {
      supergraphSdl: result.supergraphSdl,
      publicSdl: result.publicSdl,
    },
  });
}

writeFileSync(
  CORPUS,
  [...unique.values()]
    .map((c) =>
      JSON.stringify(c, (key, value) => (key === "loc" ? undefined : value)),
    )
    .join("\n") + "\n",
);
rmSync(RAW);
console.log(`${unique.size} compositions (3 fixtures) -> ${CORPUS}`);
