// Shim of src/utils/link/link-import.ts.
import type { ConstValueNode } from "graphql";
import { call, parseJSON, toJSON } from "../../runtime.js";

export interface LinkImportJSON {
  name: string;
  as: string | null;
}

export class FederatedLinkImport {
  constructor(
    public name: string,
    public as: string | null,
  ) {}

  /** @internal The JSON the Rust exports take. */
  toJSONValue(): LinkImportJSON {
    return { name: this.name, as: this.as };
  }

  public toString(): string {
    return call("link_import_to_string", JSON.stringify(this.toJSONValue()));
  }

  static fromTypedefs(node: ConstValueNode): FederatedLinkImport[] {
    return parseJSON<LinkImportJSON[]>(
      call("link_import_from_typedefs", toJSON(node)),
    ).map((i) => new FederatedLinkImport(i.name, i.as));
  }
}
