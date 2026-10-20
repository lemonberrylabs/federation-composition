// Shim of src/graphql/contains-supergraph-spec.ts.
import { call } from "../runtime.js";

export function containsSupergraphSpec(sdl: string): boolean {
  return call("contains_supergraph_spec", sdl);
}
