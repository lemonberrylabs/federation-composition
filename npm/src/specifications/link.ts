// Shim of src/specifications/link.ts: the exports the tests use.
import { call, parseJSON } from "../runtime.js";

/** An element imported by `@link(import:)` (`parseLinkImport`). */
export type LinkImport =
  | { readonly kind: "directive" | "type"; readonly name: string }
  | {
      readonly kind: "directive" | "type";
      readonly name: string;
      readonly alias: string;
    };

/** A parsed `@link` (`parseLink`). */
export type Link = {
  name: string | null;
  version: string | null;
  identity: string;
  imports: LinkImport[];
};

export function parseLinkUrl(urlString: string): {
  name: string | null;
  version: string | null;
  identity: string;
} {
  return parseJSON(call("parse_link_url", urlString));
}

export function parseLinkImport(importString: string): LinkImport[] {
  return parseJSON(call("parse_link_import", importString));
}

export const sdl: string = call("link_sdl");
