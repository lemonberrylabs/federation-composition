// Shim of src/graphql/transform-supergraph-to-public-schema.ts.
import type { DocumentNode } from "graphql";
import { call, parseJSON, toJSON } from "../runtime.js";

/**
 * Transform a supergraph document node to the public API schema, as served by a gateway.
 */
export function transformSupergraphToPublicSchema(
  documentNode: DocumentNode,
): DocumentNode {
  return parseJSON<DocumentNode>(
    call("transform_supergraph_to_public_schema", toJSON(documentNode)),
  );
}
