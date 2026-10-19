// Shim of src/subgraph/context.ts.
import { call, parseJSON } from "../runtime.js";

/**
 * This function divides the value of `@fromContext(field:)` into the context
 * name and the selection.
 */
export function parseContextReference(input: string): {
  context: string | undefined;
  selection: string | undefined;
} {
  const result = parseJSON<{ context?: string; selection?: string }>(
    call("parse_context_reference", input),
  );
  return { context: result.context, selection: result.selection };
}
