// Shim of src/validate.ts.
import type { DocumentNode, GraphQLError } from "graphql";
import type { validate as tsValidate } from "../src/validate.js";
import {
  call,
  parseJSON,
  serviceToJSON,
  servicesToJSON,
  toGraphQLErrors,
  type GraphQLErrorJSON,
} from "./runtime.js";

type ValidateResult = ReturnType<typeof tsValidate>;

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
