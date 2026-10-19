// Records every guild composeServices() call made by the test suite so the
// same inputs can be replayed by bench/run-corpus.ts against each implementation,
// together with the TypeScript output, which crates/composition/tests/corpus.rs compares against.
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
            // The TypeScript output, so other implementations can be checked against it.
            expected: original.compositionHasErrors(result)
              ? {
                  errors: result.errors.map((e) => ({
                    message: e.message,
                    code: e.extensions?.code ?? null,
                  })),
                }
              : {
                  supergraphSdl: result.supergraphSdl,
                  publicSdl: result.publicSdl,
                },
          },
          (key, value) => (key === "loc" ? undefined : value),
        ) + "\n",
      );
      return result;
    },
  };
});
