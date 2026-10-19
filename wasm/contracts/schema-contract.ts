// Shim of src/contracts/schema-contract.ts.
import type { ServiceDefinition } from "../../src/types.js";
import { fromCompositionJSON } from "../compose.js";
import { call, servicesToJSON } from "../runtime.js";
import { filterToJSON, type SchemaContractFilter } from "./tag-extraction.js";

/** Compose a federation schema contract using a filter. */
export function composeSchemaContract(
  services: ServiceDefinition[],
  tagFilter: SchemaContractFilter,
  removeUnreachableTypes = true,
) {
  return fromCompositionJSON(
    call(
      "compose_schema_contract",
      servicesToJSON(services),
      filterToJSON(tagFilter),
      Boolean(removeUnreachableTypes),
    ),
  );
}
