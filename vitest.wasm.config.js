// Runs the test suite against the WebAssembly build: every import of a module
// under src/ from a test file is redirected to its shim under wasm/.
// Build the bindings first with `pnpm wasm:build`.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, mergeConfig } from "vitest/config";
import base from "./vitest.config.js";

const root = path.dirname(fileURLToPath(import.meta.url));
const srcDir = path.join(root, "src");
const wasmDir = path.join(root, "wasm");
const testsDir = path.join(root, "__tests__");

function isTestModule(file) {
  return (
    file.startsWith(testsDir + path.sep) ||
    (file.startsWith(srcDir + path.sep) && file.endsWith(".spec.ts"))
  );
}

/** The shim for a module path under src/, with or without extension, or a directory. */
function shimFor(target) {
  let rel = path.relative(srcDir, target).replace(/\.(js|ts)$/, "");
  if (
    rel === "" ||
    (fs.existsSync(target) && fs.statSync(target).isDirectory())
  ) {
    rel = path.join(rel, "index");
  }
  return { rel, shim: path.join(wasmDir, rel + ".ts") };
}

const srcToWasm = {
  name: "src-to-wasm",
  enforce: "pre",
  resolveId(source, importer) {
    if (!importer || !source.startsWith(".")) return null;
    const importerFile = importer.split("?")[0];
    if (!isTestModule(importerFile)) return null;
    const target = path.resolve(path.dirname(importerFile), source);
    if (target !== srcDir && !target.startsWith(srcDir + path.sep)) return null;
    if (target.endsWith(".spec.ts")) return null;
    const { rel, shim } = shimFor(target);
    if (!fs.existsSync(shim)) {
      // Never fall back to the TypeScript implementation.
      this.error(
        `No WebAssembly shim for src/${rel} (imported by ${path.relative(root, importerFile)}): add wasm/${rel}.ts`,
      );
    }
    return shim;
  },
};

export default mergeConfig(
  base,
  defineConfig({
    plugins: [srcToWasm],
    test: {
      include: [
        "src/**/*.spec.ts",
        "__tests__/**/*.spec.ts",
        "wasm/**/*.spec.ts",
      ],
    },
  }),
  false,
);
