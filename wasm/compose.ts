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
  decodeJSON,
  encodeJSON,
  parseJSON,
  servicesToJSON,
  toGraphQLErrors,
  type GraphQLErrorJSON,
} from "./runtime.js";

export type { CompositionFailure, CompositionResult, CompositionSuccess };

/**
 * What the Rust exports returning a composition give back (`composition_out` in
 * crates/federation-composition-wasm/src/exports.rs): the errors' JSON on failure, the
 * supergraph document's JSON as UTF-8 bytes on success.
 */
export type CompositionOut = string | Uint8Array;

export function fromCompositionOut(out: CompositionOut): CompositionResult {
  if (typeof out === "string") {
    return { errors: toGraphQLErrors(parseJSON<GraphQLErrorJSON[]>(out)) };
  }
  return createCompositionSuccessFromJSON(out);
}

export function composeServices(
  services: ServiceDefinition[],
  __internal?: {
    disableValidationRules?: string[];
  },
): CompositionResult {
  return fromCompositionOut(
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
          encodeJSON(supergraphDocumentNode),
        );
      }
      return supergraphSdl;
    },
    get publicDocumentNode() {
      if (publicDocumentNode == null) {
        publicDocumentNode = parseJSON<DocumentNode>(
          call(
            "transform_supergraph_to_public_schema",
            encodeJSON(supergraphDocumentNode),
          ),
        );
      }

      return publicDocumentNode;
    },
    get publicSdl() {
      if (publicSdl == null) {
        publicSdl = call(
          "print_supergraph",
          encodeJSON(this.publicDocumentNode),
        );
      }

      return publicSdl;
    },
  };
}

/**
 * {@link createCompositionSuccessReadCacheContainer} for a supergraph document
 * that is still the JSON a Rust export returned.
 *
 * PERF: the TS has the document as an object already. Here turning the JSON
 * into objects costs about as much as serializing it in Rust, and most callers
 * only read `supergraphSdl`, so the JSON is parsed only when
 * `supergraphDocumentNode` is read. Until then the getters hand the JSON bytes
 * straight back to Rust. Once the document has been read the caller may have
 * changed it, so from then on the getters serialize that object, which is what
 * the TS prints.
 */
export function createCompositionSuccessFromJSON(
  supergraphDocumentJSON: Uint8Array,
): CompositionSuccess {
  let supergraphDocumentNode: DocumentNode | undefined;
  let supergraphSdl: string;
  let publicDocumentNode: DocumentNode;
  let publicSdl: string;

  const documentJSON = () =>
    supergraphDocumentNode === undefined
      ? supergraphDocumentJSON
      : encodeJSON(supergraphDocumentNode);

  return {
    get supergraphDocumentNode(): DocumentNode {
      if (supergraphDocumentNode === undefined) {
        supergraphDocumentNode = decodeJSON<DocumentNode>(
          supergraphDocumentJSON,
        );
      }
      return supergraphDocumentNode;
    },
    // A writable data property in the TS. Assigning it changes what it reads,
    // not what the getters below print (they use the closure variable).
    set supergraphDocumentNode(value: DocumentNode) {
      Object.defineProperty(this, "supergraphDocumentNode", {
        value,
        writable: true,
        enumerable: true,
        configurable: true,
      });
    },
    get supergraphSdl() {
      if (supergraphSdl == null) {
        supergraphSdl = call("print_supergraph", documentJSON());
      }
      return supergraphSdl;
    },
    get publicDocumentNode() {
      if (publicDocumentNode == null) {
        publicDocumentNode = parseJSON<DocumentNode>(
          call("transform_supergraph_to_public_schema", documentJSON()),
        );
      }

      return publicDocumentNode;
    },
    get publicSdl() {
      if (publicSdl == null) {
        publicSdl = call(
          "print_supergraph",
          encodeJSON(this.publicDocumentNode),
        );
      }

      return publicSdl;
    },
  };
}
