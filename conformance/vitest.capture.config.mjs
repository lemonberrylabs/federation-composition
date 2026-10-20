// Runs upstream's unchanged test suite (conformance/upstream) on upstream's TypeScript with
// capture-setup.ts, which records every composition it performs. Run by conformance/run.mjs
// with upstream's own vitest and CAPTURE_FILE set, so this file imports nothing from vitest.
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, "upstream");
const { default: base } = await import(
  pathToFileURL(path.join(root, "vitest.config.js")).href
);

export default {
  ...base,
  root,
  resolve: {
    ...base.resolve,
    // capture-setup.ts lives outside upstream; its `vitest` import must be upstream's.
    dedupe: [...(base.resolve?.dedupe ?? []), "vitest"],
  },
  test: {
    ...base.test,
    setupFiles: [
      ...[base.test?.setupFiles ?? []].flat(),
      path.join(here, "capture-setup.ts"),
    ],
  },
};
