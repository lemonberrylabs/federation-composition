// Shim of src/types.ts.
import type { DocumentNode } from "graphql";

export interface ServiceDefinition {
  typeDefs: DocumentNode;
  name: string;
  url?: string;
}
