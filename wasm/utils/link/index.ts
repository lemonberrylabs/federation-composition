// Shim of src/utils/link/index.ts.
import type { DocumentNode } from "graphql";
import { call, parseJSON, toJSON } from "../../runtime.js";
import { FederatedLink, type LinkJSON } from "./link.js";

export const FEDERATION_V1 = Symbol("Federation_V1");

export type LinkVersion =
  | string
  | { major: number; minor: number }
  | null
  | typeof FEDERATION_V1;

function versionToJSON(version: LinkVersion): string {
  if (version === FEDERATION_V1) {
    return JSON.stringify({ kind: "federationV1" });
  }
  if (typeof version === "string") {
    return JSON.stringify({ kind: "string", version });
  }
  if (version === null) {
    return JSON.stringify({ kind: "null" });
  }
  return JSON.stringify({
    kind: "numbers",
    major: version.major ?? null,
    minor: version.minor ?? null,
  });
}

export function extractLinkImplementations(typeDefs: DocumentNode): {
  links: FederatedLink[];
  resolveImportName: (identity: string, name: string) => string;
  matchesImplementation: (identity: string, version: LinkVersion) => boolean;
} {
  const linksJSON = call("extract_link_implementations", toJSON(typeDefs));
  const links = parseJSON<LinkJSON[]>(linksJSON).map(
    FederatedLink.fromJSONValue,
  );
  return {
    links,
    resolveImportName: (identity, name) =>
      call("extract_link_resolve_import_name", linksJSON, identity, name),
    matchesImplementation: (identity, version) =>
      call(
        "extract_link_matches_implementation",
        linksJSON,
        identity,
        versionToJSON(version),
      ),
  };
}
