// Records every guild composeServices() call made by the test suite so the
// same inputs can be replayed by bench/run-corpus.ts against each implementation.
import { appendFileSync } from "node:fs";
import { vi } from "vitest";

const out = process.env.CAPTURE_FILE!;

vi.mock("../src/compose.js", async (importOriginal) => {
  const original = await importOriginal<typeof import("../src/compose.js")>();
  return {
    ...original,
    composeServices(...args: Parameters<typeof original.composeServices>) {
      const result = original.composeServices(...args);
      const [services, internal] = args;
      appendFileSync(
        out,
        JSON.stringify(
          {
            services: services.map((s) => ({
              name: s.name,
              url: s.url,
              // AST rather than SDL: some test inputs print to SDL that does not reparse.
              typeDefs: s.typeDefs,
            })),
            internal,
            success: !("errors" in result && result.errors),
          },
          (key, value) => (key === "loc" ? undefined : value),
        ) + "\n",
      );
      return result;
    },
  };
});
