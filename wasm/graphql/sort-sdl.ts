// Shim of src/graphql/sort-sdl.ts.
import type { DocumentNode } from "graphql";
import { call, encodeJSON, parseJSON } from "../runtime.js";

export function sortSDL(doc: DocumentNode): DocumentNode {
  return parseJSON<DocumentNode>(call("sort_sdl", encodeJSON(doc)));
}
