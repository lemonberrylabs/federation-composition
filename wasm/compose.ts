// Shim of src/compose.ts.
import type { DocumentNode } from "graphql";
import type {
  CompositionFailure,
  CompositionResult,
  CompositionSuccess,
} from "../src/compose.js";
import type { ServiceDefinition } from "../src/types.js";
import {
  call,
  parseJSON,
  servicesToJSON,
  toGraphQLErrors,
  toJSON,
  type GraphQLErrorJSON,
} from "./runtime.js";

export type { CompositionFailure, CompositionResult, CompositionSuccess };

/** What the Rust exports returning a composition give back. */
export type CompositionJSON =
  | { errors: GraphQLErrorJSON[] }
  | { supergraphDocumentNode: DocumentNode };

export function fromCompositionJSON(json: string): CompositionResult {
  const result = parseJSON<CompositionJSON>(json);
  if ("errors" in result) {
    return { errors: toGraphQLErrors(result.errors) };
  }
  return createCompositionSuccessReadCacheContainer(
    result.supergraphDocumentNode,
  );
}

export function composeServices(
  services: ServiceDefinition[],
  __internal?: {
    disableValidationRules?: string[];
  },
): CompositionResult {
  return fromCompositionJSON(
    call(
      "compose_services",
      servicesToJSON(services),
      __internal === undefined ? undefined : JSON.stringify(__internal),
    ),
  );
}

export function assertCompositionSuccess(
  compositionResult: CompositionResult,
  message?: string,
): asserts compositionResult is CompositionSuccess {
  if (compositionHasErrors(compositionResult)) {
    throw new Error(message || "Unexpected test failure");
  }
}

export function assertCompositionFailure(
  compositionResult: CompositionResult,
  message?: string,
): asserts compositionResult is CompositionFailure {
  if (!compositionHasErrors(compositionResult)) {
    throw new Error(message || "Unexpected test failure");
  }
}

export function compositionHasErrors(
  compositionResult: CompositionResult,
): compositionResult is CompositionFailure {
  return "errors" in compositionResult && !!compositionResult.errors;
}

/** Only print sdl and construct public document node when requested! */
export function createCompositionSuccessReadCacheContainer(
  supergraphDocumentNode: DocumentNode,
): CompositionSuccess {
  let supergraphSdl: string;
  let publicDocumentNode: DocumentNode;
  let publicSdl: string;

  return {
    supergraphDocumentNode,
    get supergraphSdl() {
      if (supergraphSdl == null) {
        supergraphSdl = call(
          "print_supergraph",
          toJSON(supergraphDocumentNode),
        );
      }
      return supergraphSdl;
    },
    get publicDocumentNode() {
      if (publicDocumentNode == null) {
        publicDocumentNode = parseJSON<DocumentNode>(
          call(
            "transform_supergraph_to_public_schema",
            toJSON(supergraphDocumentNode),
          ),
        );
      }

      return publicDocumentNode;
    },
    get publicSdl() {
      if (publicSdl == null) {
        publicSdl = call("print_supergraph", toJSON(this.publicDocumentNode));
      }

      return publicSdl;
    },
  };
}
