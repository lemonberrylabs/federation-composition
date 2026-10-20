// Shim of src/supergraph/composition/ast.ts: the exports the tests use.
import type {
  DocumentNode,
  EnumTypeDefinitionNode,
  InputObjectTypeDefinitionNode,
  InterfaceTypeDefinitionNode,
  ObjectTypeDefinitionNode,
  ScalarTypeDefinitionNode,
  SchemaDefinitionNode,
  UnionTypeDefinitionNode,
} from "graphql";
import { call, encodeJSON, parseJSON, toJSON } from "../../runtime.js";

/**
 * The parameter objects of the `create*Node` functions. Their exact shapes are those of
 * upstream's src/supergraph/composition/ast.ts; this module exists so its tests can run
 * against the WebAssembly build and is not part of the public API, so it does not repeat them.
 */
type AST = { readonly [key: string]: unknown };

export type DirectiveAST = AST;
export type EnumValueAST = AST;
export type FieldArgumentAST = AST;
export type FieldAST = AST;
export type InputFieldAST = AST;
export type JoinEnumValueAST = AST;
export type JoinFieldAST = AST;
export type JoinImplementsAST = AST;
export type JoinTypeAST = AST;
export type JoinUnionMemberAST = AST;
export type ObjectTypeAST = AST;

export function createSchemaNode(schema: AST): SchemaDefinitionNode {
  return parseJSON(call("create_schema_node", toJSON(schema)));
}

export function createObjectTypeNode(
  objectType: AST,
): ObjectTypeDefinitionNode {
  return parseJSON(call("create_object_type_node", toJSON(objectType)));
}

export function createInterfaceTypeNode(
  interfaceType: AST,
): InterfaceTypeDefinitionNode {
  return parseJSON(call("create_interface_type_node", toJSON(interfaceType)));
}

export function createInputObjectTypeNode(
  inputObjectType: AST,
): InputObjectTypeDefinitionNode {
  return parseJSON(
    call("create_input_object_type_node", toJSON(inputObjectType)),
  );
}

export function createUnionTypeNode(unionType: AST): UnionTypeDefinitionNode {
  return parseJSON(call("create_union_type_node", toJSON(unionType)));
}

export function createEnumTypeNode(enumType: AST): EnumTypeDefinitionNode {
  return parseJSON(call("create_enum_type_node", toJSON(enumType)));
}

export function createScalarTypeNode(
  scalarType: AST,
): ScalarTypeDefinitionNode {
  return parseJSON(call("create_scalar_type_node", toJSON(scalarType)));
}

export function createJoinGraphEnumTypeNode(
  graphs: ReadonlyArray<{ name: string; enumValue: string; url?: string }>,
): EnumTypeDefinitionNode {
  return parseJSON(call("create_join_graph_enum_type_node", toJSON(graphs)));
}

export function stripFederation(supergraph: DocumentNode): DocumentNode {
  return parseJSON(call("strip_federation", encodeJSON(supergraph)));
}
