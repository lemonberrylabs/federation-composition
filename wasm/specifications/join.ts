// Shim of src/specifications/join.ts.
import type { FederationVersion } from "../../src/specifications/federation.js";
import { call } from "../runtime.js";

export const sdl = (version: FederationVersion): string =>
  call("join_sdl", version);
