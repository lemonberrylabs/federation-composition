import { defineConfig, mergeConfig } from "vitest/config";
import base from "../vitest.config.js";

export default mergeConfig(
  base,
  defineConfig({
    root: new URL("..", import.meta.url).pathname,
    test: {
      setupFiles: ["./bench/capture-setup.ts"],
    },
  }),
);
