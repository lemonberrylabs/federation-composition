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
import type * as ts from "../../../src/supergraph/composition/ast.js";
import { call, parseJSON, toJSON } from "../../runtime.js";

export type {
  DirectiveAST,
  EnumValueAST,
  FieldArgumentAST,
  FieldAST,
  InputFieldAST,
  JoinEnumValueAST,
  JoinFieldAST,
  JoinImplementsAST,
  JoinTypeAST,
  JoinUnionMemberAST,
  ObjectTypeAST,
} from "../../../src/supergraph/composition/ast.js";

type Arg<F> = F extends (arg: infer A) => any ? A : never;

export function createSchemaNode(
  schema: Arg<typeof ts.createSchemaNode>,
): SchemaDefinitionNode {
  return parseJSON(call("create_schema_node", toJSON(schema)));
}

export function createObjectTypeNode(
  objectType: Arg<typeof ts.createObjectTypeNode>,
): ObjectTypeDefinitionNode {
  return parseJSON(call("create_object_type_node", toJSON(objectType)));
}

export function createInterfaceTypeNode(
  interfaceType: Arg<typeof ts.createInterfaceTypeNode>,
): InterfaceTypeDefinitionNode {
  return parseJSON(call("create_interface_type_node", toJSON(interfaceType)));
}

export function createInputObjectTypeNode(
  inputObjectType: Arg<typeof ts.createInputObjectTypeNode>,
): InputObjectTypeDefinitionNode {
  return parseJSON(
    call("create_input_object_type_node", toJSON(inputObjectType)),
  );
}

export function createUnionTypeNode(
  unionType: Arg<typeof ts.createUnionTypeNode>,
): UnionTypeDefinitionNode {
  return parseJSON(call("create_union_type_node", toJSON(unionType)));
}

export function createEnumTypeNode(
  enumType: Arg<typeof ts.createEnumTypeNode>,
): EnumTypeDefinitionNode {
  return parseJSON(call("create_enum_type_node", toJSON(enumType)));
}

export function createScalarTypeNode(
  scalarType: Arg<typeof ts.createScalarTypeNode>,
): ScalarTypeDefinitionNode {
  return parseJSON(call("create_scalar_type_node", toJSON(scalarType)));
}

export function createJoinGraphEnumTypeNode(
  graphs: Arg<typeof ts.createJoinGraphEnumTypeNode>,
): EnumTypeDefinitionNode {
  return parseJSON(call("create_join_graph_enum_type_node", toJSON(graphs)));
}

export function stripFederation(supergraph: DocumentNode): DocumentNode {
  return parseJSON(call("strip_federation", toJSON(supergraph)));
}
