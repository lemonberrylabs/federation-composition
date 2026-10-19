// Builds crates/composition-wasm into wasm/pkg (gitignored): cargo (release,
// wasm32-unknown-unknown) with the rustup toolchain, then wasm-bindgen
// --target nodejs, then wasm-opt if it is installed.
//
//   pnpm wasm:build            release build
//   pnpm wasm:build --dev      debug build (faster to compile, slower to run)
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "..");
const out = path.join(here, "pkg");
const dev = process.argv.includes("--dev");
const profile = dev ? "debug" : "release";

// rustup's toolchain, not Homebrew's (which has no wasm32 std). Its rust-lld
// loads libLLVM.dylib from the toolchain's lib/ directory, which on macOS it
// only finds through DYLD_FALLBACK_LIBRARY_PATH.
const toolchain =
  process.env.RUST_TOOLCHAIN_DIR ??
  path.join(
    os.homedir(),
    ".rustup/toolchains",
    `stable-${process.arch === "arm64" ? "aarch64" : "x86_64"}-apple-darwin`,
  );
const env = { ...process.env };
if (fs.existsSync(path.join(toolchain, "bin/cargo"))) {
  env.PATH = `${path.join(toolchain, "bin")}${path.delimiter}${env.PATH}`;
  env.DYLD_FALLBACK_LIBRARY_PATH = [
    path.join(toolchain, "lib"),
    env.DYLD_FALLBACK_LIBRARY_PATH,
  ]
    .filter(Boolean)
    .join(path.delimiter);
}
env.PATH = `${path.join(os.homedir(), ".cargo/bin")}${path.delimiter}${env.PATH}`;

function run(cmd, args) {
  console.log(`$ ${cmd} ${args.join(" ")}`);
  const r = spawnSync(cmd, args, { cwd: repo, env, stdio: "inherit" });
  if (r.error) throw r.error;
  if (r.status !== 0) process.exit(r.status ?? 1);
}

run("cargo", [
  "build",
  "-p",
  "composition_wasm",
  "--target",
  "wasm32-unknown-unknown",
  ...(dev ? [] : ["--release"]),
]);

// wasm-bindgen-cli must match the wasm-bindgen crate version exactly.
const lock = fs.readFileSync(path.join(repo, "Cargo.lock"), "utf8");
const crateVersion = /name = "wasm-bindgen"\nversion = "([^"]+)"/.exec(
  lock,
)?.[1];
let cliVersion = null;
try {
  cliVersion = execFileSync("wasm-bindgen", ["--version"], {
    env,
    encoding: "utf8",
  })
    .trim()
    .split(" ")[1];
} catch {}
if (cliVersion !== crateVersion) {
  console.error(
    `wasm-bindgen-cli ${cliVersion ?? "is not installed"}; the crate uses ${crateVersion}. Install it with:\n` +
      `  cargo install wasm-bindgen-cli --version '=${crateVersion}' --locked`,
  );
  process.exit(1);
}

fs.rmSync(out, { recursive: true, force: true });
const wasmFile = path.join(
  repo,
  "target/wasm32-unknown-unknown",
  profile,
  "composition_wasm.wasm",
);
run("wasm-bindgen", ["--target", "nodejs", "--out-dir", out, wasmFile]);

// The root package is "type": "module"; the nodejs glue is CommonJS.
fs.writeFileSync(
  path.join(out, "package.json"),
  JSON.stringify({ type: "commonjs" }, null, 2) + "\n",
);

const wasmOpt = spawnSync("wasm-opt", ["--version"], { env });
const bindgenWasm = path.join(out, "composition_wasm_bg.wasm");
if (!dev && wasmOpt.status === 0) {
  run("wasm-opt", [
    "-O3",
    "--enable-bulk-memory",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "--enable-mutable-globals",
    bindgenWasm,
    "-o",
    bindgenWasm,
  ]);
} else if (!dev) {
  console.log(
    "wasm-opt not found; skipping (install binaryen for a smaller, faster build)",
  );
}
console.log(
  `built ${path.relative(repo, out)} (${(fs.statSync(bindgenWasm).size / 1e6).toFixed(2)} MB)`,
);
