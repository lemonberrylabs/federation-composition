// Shim of src/validate.ts.
import type { DefinitionNode, DocumentNode, GraphQLError } from "graphql";
import type { Link } from "./specifications/link.js";
import {
  call,
  parseJSON,
  serviceToJSON,
  servicesToJSON,
  toGraphQLErrors,
  type GraphQLErrorJSON,
} from "./runtime.js";

/** The Federation versions `@link(url: "https://specs.apollo.dev/federation/vX.Y")` can name. */
export type FederationVersion =
  | "v1.0"
  | "v2.0"
  | "v2.1"
  | "v2.2"
  | "v2.3"
  | "v2.4"
  | "v2.5"
  | "v2.6"
  | "v2.7"
  | "v2.8"
  | "v2.9";

/** What `validate` returns (the return type of the TS `validate`). */
export type ValidateResult =
  | { readonly success: false; readonly errors: GraphQLError[] }
  | {
      readonly success: true;
      /** The supergraph's definitions. */
      readonly supergraph: DefinitionNode[];
      readonly links: readonly Link[];
      readonly specs: {
        tag: boolean;
        context: boolean;
        cost: {
          used: boolean;
          names: { cost: string | null; listSize: string | null };
        };
        inaccessible: boolean;
        policy: boolean;
        requiresScopes: boolean;
        authenticated: boolean;
      };
      readonly federationVersion: FederationVersion;
    };

export function validateSubgraph(subgraph: {
  name: string;
  url?: string;
  typeDefs: DocumentNode;
}): GraphQLError[] {
  return toGraphQLErrors(
    parseJSON<GraphQLErrorJSON[]>(
      call("validate_subgraph", serviceToJSON(subgraph)),
    ),
  );
}

export function validate(
  subgraphs: ReadonlyArray<{
    name: string;
    url?: string;
    typeDefs: DocumentNode;
  }>,
  __internal?: {
    disableValidationRules?: string[];
  },
): ValidateResult {
  const result = parseJSON<
    | { success: false; errors: GraphQLErrorJSON[] }
    | (Extract<ValidateResult, { success: true }> & { success: true })
  >(
    call(
      "validate",
      servicesToJSON(subgraphs),
      __internal === undefined ? undefined : JSON.stringify(__internal),
    ),
  );
  if (!result.success) {
    return { success: false, errors: toGraphQLErrors(result.errors) } as const;
  }
  return result;
}
