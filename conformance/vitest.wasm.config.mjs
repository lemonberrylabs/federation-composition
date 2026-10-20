// Runs upstream's unchanged test suite (conformance/upstream) against the npm package
// federation-composition-wasm: every import of a module under upstream's src/ from a test
// module is redirected to the package's entry point for that module
// (`federation-composition-wasm/internal/<path>`, the shim in npm/src/<path>.ts).
// Build the package first (`pnpm wasm:build`). Run by conformance/run.mjs with upstream's
// own vitest, so this file imports nothing from vitest.
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, "upstream");
const srcDir = path.join(root, "src");
const testsDir = path.join(root, "__tests__");
const require = createRequire(path.join(here, "package.json"));
const { default: base } = await import(
  pathToFileURL(path.join(root, "vitest.config.js")).href
);

function isTestModule(file) {
  return (
    file.startsWith(testsDir + path.sep) ||
    (file.startsWith(srcDir + path.sep) && file.endsWith(".spec.ts"))
  );
}

function isUpstreamImplementation(file) {
  return file.startsWith(srcDir + path.sep) && !file.endsWith(".spec.ts");
}

/** The module path under src/ of an import target, with or without extension, or a directory. */
function modulePath(target) {
  let rel = path.relative(srcDir, target).replace(/\.(js|ts)$/, "");
  if (
    rel === "" ||
    (fs.existsSync(target) && fs.statSync(target).isDirectory())
  ) {
    rel = path.join(rel, "index");
  }
  return rel.split(path.sep).join("/");
}

const srcToWasm = {
  name: "upstream-src-to-wasm",
  enforce: "pre",
  resolveId(source, importer) {
    if (!importer || !source.startsWith(".")) return null;
    const importerFile = importer.split("?")[0];
    if (!isTestModule(importerFile)) return null;
    const target = path.resolve(path.dirname(importerFile), source);
    if (target !== srcDir && !target.startsWith(srcDir + path.sep)) return null;
    if (target.endsWith(".spec.ts")) return null;
    const rel = modulePath(target);
    try {
      return require.resolve(`federation-composition-wasm/internal/${rel}`);
    } catch {
      // Never fall back to the TypeScript implementation.
      this.error(
        `No WebAssembly shim for src/${rel} (imported by ${path.relative(root, importerFile)}): add npm/src/${rel}.ts, or build the package (pnpm wasm:build)`,
      );
    }
  },
  // The self-check: no module of upstream's implementation may be loaded, by any path.
  load(id) {
    const file = id.split("?")[0];
    if (isUpstreamImplementation(file)) {
      this.error(
        `Upstream's TypeScript implementation was loaded (${path.relative(root, file)}); the conformance run must only use federation-composition-wasm`,
      );
    }
    return null;
  },
};

export default {
  ...base,
  root,
  plugins: [...(base.plugins ?? []), srcToWasm],
  resolve: {
    ...base.resolve,
    // The package's modules must use the same graphql as the tests (instanceof GraphQLError,
    // the patched Location.prototype.toJSON).
    dedupe: [...(base.resolve?.dedupe ?? []), "graphql"],
  },
};
