// Shim of src/contracts/add-inaccessible-to-unreachable-types.ts.
import type { DocumentNode } from "graphql";
import {
  createCompositionSuccessReadCacheContainer,
  type CompositionSuccess,
} from "../compose.js";
import { call, parseJSON, toJSON } from "../runtime.js";

type AddInaccessibleToUnreachableTypesInput = Pick<
  CompositionSuccess,
  "supergraphDocumentNode" | "publicDocumentNode"
>;

/**
 * Adds inaccessible directive to unreachable types within the supergraph and removes them from the
 * public GraphQL schema.
 */
export const addInaccessibleToUnreachableTypes = (
  resolveName: (identity: string, name: string) => string,
  compositionResult: AddInaccessibleToUnreachableTypesInput,
): CompositionSuccess => {
  // The Rust side derives the public document from the supergraph, as the
  // getter of a `CompositionSuccess` does.
  const supergraphNode = parseJSON<DocumentNode>(
    call(
      "add_inaccessible_to_unreachable_types",
      resolveName,
      toJSON(compositionResult.supergraphDocumentNode),
    ),
  );
  return createCompositionSuccessReadCacheContainer(supergraphNode);
};
