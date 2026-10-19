// Shim of src/specifications/link.ts: the exports the tests use.
import type * as ts from "../../src/specifications/link.js";
import { call, parseJSON } from "../runtime.js";

export type { Link, LinkImport } from "../../src/specifications/link.js";

export function parseLinkUrl(
  urlString: string,
): ReturnType<typeof ts.parseLinkUrl> {
  return parseJSON(call("parse_link_url", urlString));
}

export function parseLinkImport(
  importString: string,
): ReturnType<typeof ts.parseLinkImport> {
  return parseJSON(call("parse_link_import", importString));
}

export const sdl: string = call("link_sdl");
