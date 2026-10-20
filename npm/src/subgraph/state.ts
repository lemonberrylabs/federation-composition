// Shim of src/subgraph/state.ts: its enums, with the values of the TS enums.

export enum TypeKind {
  OBJECT = "OBJECT",
  INTERFACE = "INTERFACE",
  ENUM = "ENUM",
  UNION = "UNION",
  SCALAR = "SCALAR",
  INPUT_OBJECT = "INPUT_OBJECT",
  DIRECTIVE = "DIRECTIVE",
}

export enum ArgumentKind {
  SCALAR = "SCALAR",
  OBJECT = "OBJECT",
  ENUM = "ENUM",
}
