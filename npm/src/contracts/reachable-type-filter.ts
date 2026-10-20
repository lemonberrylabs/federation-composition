// Shim of src/contracts/reachable-type-filter.ts.
import type { DocumentNode } from "graphql";
import { call, encodeJSON, parseJSON } from "../runtime.js";

/**
 * Retrieve a named list of all types that are reachable from the root types.
 */
export function getReachableTypes(documentNode: DocumentNode): Set<string> {
  return new Set(
    parseJSON<string[]>(call("get_reachable_types", encodeJSON(documentNode))),
  );
}

/**
 * For a given GraphQL schema document node, add a directive with a given name to all types not in the provided set.
 */
export function addDirectiveOnTypes(args: {
  documentNode: DocumentNode;
  excludedTypeNames: Set<string>;
  directiveName: string;
}): DocumentNode {
  return parseJSON<DocumentNode>(
    call(
      "add_directive_on_types",
      encodeJSON({
        documentNode: args.documentNode,
        excludedTypeNames: Array.from(args.excludedTypeNames),
        directiveName: args.directiveName,
      }),
    ),
  );
}
