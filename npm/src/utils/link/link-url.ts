// Shim of src/utils/link/link-url.ts.
import { call, parseJSON } from "../../runtime.js";

export interface LinkUrlJSON {
  identity: string;
  name: string | null;
  version: string | null;
}

/** The arguments of `supports`, as the Rust exports take them. */
export function supportsArgsToJSON(args: unknown[]): string {
  const majorOrVersion = args[0];
  if (typeof majorOrVersion === "string") {
    return JSON.stringify({ kind: "string", version: majorOrVersion });
  }
  if (typeof majorOrVersion === "number") {
    return JSON.stringify({
      kind: "numbers",
      major: majorOrVersion,
      minor: args[1] ?? null,
    });
  }
  if (majorOrVersion instanceof FederatedLinkUrl) {
    return JSON.stringify({ kind: "url", url: majorOrVersion.toJSONValue() });
  }
  if (majorOrVersion === null) {
    return JSON.stringify({ kind: "null" });
  }
  return JSON.stringify({ kind: "invalid", json: JSON.stringify(args) });
}

/**
 * A wrapper around the `@link` url -- this parses all necessary data to identify the link
 * and determine which version is most appropriate to use.
 */
export class FederatedLinkUrl {
  constructor(
    public readonly identity: string,
    public readonly name: string | null,
    public readonly version: string | null,
  ) {}

  /** @internal The JSON the Rust exports take. */
  toJSONValue(): LinkUrlJSON {
    return { identity: this.identity, name: this.name, version: this.version };
  }

  /** @internal */
  static fromJSONValue(json: LinkUrlJSON): FederatedLinkUrl {
    return new FederatedLinkUrl(json.identity, json.name, json.version);
  }

  public toString(): string {
    return call("link_url_to_string", JSON.stringify(this.toJSONValue()));
  }

  static fromUrl = (urlSource: string): FederatedLinkUrl =>
    FederatedLinkUrl.fromJSONValue(
      parseJSON(call("link_url_from_url", urlSource)),
    );

  /** Check if this version supports another version */
  supports(version: string): boolean;
  supports(major: number, minor: number): boolean;
  supports(version: FederatedLinkUrl): boolean;
  supports(version: null): boolean;
  supports(
    ...args: [string] | [number, number] | [FederatedLinkUrl] | [null]
  ): boolean {
    return call(
      "link_url_supports",
      JSON.stringify(this.toJSONValue()),
      supportsArgsToJSON(args),
    );
  }
}
