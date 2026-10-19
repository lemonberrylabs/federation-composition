// Shim of src/utils/link/link.ts.
import type { DocumentNode } from "graphql";
import { call, parseJSON, toJSON } from "../../runtime.js";
import { FederatedLinkImport, type LinkImportJSON } from "./link-import.js";
import {
  FederatedLinkUrl,
  supportsArgsToJSON,
  type LinkUrlJSON,
} from "./link-url.js";

export interface LinkJSON {
  url: LinkUrlJSON;
  as: string | null;
  imports: LinkImportJSON[];
}

export class FederatedLink {
  constructor(
    private readonly _url: FederatedLinkUrl,
    private readonly _as: string | null,
    private readonly _imports: FederatedLinkImport[],
  ) {}

  /** @internal The JSON the Rust exports take. */
  toJSONValue(): LinkJSON {
    return {
      url: this._url.toJSONValue(),
      as: this._as,
      imports: this._imports.map((i) => i.toJSONValue()),
    };
  }

  /** @internal */
  static fromJSONValue(json: LinkJSON): FederatedLink {
    return new FederatedLink(
      FederatedLinkUrl.fromJSONValue(json.url),
      json.as,
      json.imports.map((i) => new FederatedLinkImport(i.name, i.as)),
    );
  }

  /** Collects all `@link`s defined in graphql typedefs */
  static fromTypedefs(typeDefs: DocumentNode): FederatedLink[] {
    return parseJSON<LinkJSON[]>(
      call("link_from_typedefs", toJSON(typeDefs)),
    ).map(FederatedLink.fromJSONValue);
  }

  toString(): string {
    return call("link_to_string", JSON.stringify(this.toJSONValue()));
  }

  /** The Link's identity. This is the unique identifier of a `@link`. */
  get identity(): string {
    return this._url.identity;
  }

  supports(version: string): boolean;
  supports(major: number, minor: number): boolean;
  supports(version: FederatedLinkUrl): boolean;
  supports(version: null): boolean;
  supports(
    ...args: [string] | [number, number] | [FederatedLinkUrl] | [null]
  ): boolean {
    return call(
      "link_supports",
      JSON.stringify(this.toJSONValue()),
      supportsArgsToJSON(args),
    );
  }

  /**
   * Given the name of an element in a linked schema, this returns the name of that element
   * as it has been imported. This accounts for aliasing and namespacing unreferenced imports.
   */
  resolveImportName(elementName: string): string {
    return call(
      "link_resolve_import_name",
      JSON.stringify(this.toJSONValue()),
      elementName,
    );
  }

  get imports(): readonly FederatedLinkImport[] {
    return this._imports;
  }
}
