// Shim of src/specifications/tag.ts: the exports the tests use.
import type { DirectiveDefinitionNode } from "graphql";
import { call, parseJSON } from "../runtime.js";

export const directive: DirectiveDefinitionNode = parseJSON(
  call("tag_directive"),
);
