// The environment for running cargo and the wasm-bindgen CLI, shared by build.mjs and
// conformance/run.mjs.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

/**
 * `process.env` with rustup's toolchain first on PATH (not Homebrew's, which has no wasm32
 * std) and ~/.cargo/bin (wasm-bindgen) on PATH. Its rust-lld loads libLLVM.dylib from the
 * toolchain's lib/ directory, which on macOS it only finds through DYLD_FALLBACK_LIBRARY_PATH.
 * `RUST_TOOLCHAIN_DIR` overrides the toolchain directory; where it does not exist (Linux CI,
 * which installs the toolchain on PATH), PATH is left as it is.
 */
export function rustEnv() {
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
  return env;
}
