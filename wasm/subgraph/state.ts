// Shim of src/subgraph/state.ts: its enums. The values are those of the TS
// enums; the types are the TS enums themselves, so code typed against src/
// accepts them.
import type * as ts from "../../src/subgraph/state.js";

export const TypeKind = {
  OBJECT: "OBJECT",
  INTERFACE: "INTERFACE",
  ENUM: "ENUM",
  UNION: "UNION",
  SCALAR: "SCALAR",
  INPUT_OBJECT: "INPUT_OBJECT",
  DIRECTIVE: "DIRECTIVE",
} as unknown as typeof ts.TypeKind;
export type TypeKind = ts.TypeKind;

export const ArgumentKind = {
  SCALAR: "SCALAR",
  OBJECT: "OBJECT",
  ENUM: "ENUM",
} as unknown as typeof ts.ArgumentKind;
export type ArgumentKind = ts.ArgumentKind;
