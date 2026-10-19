// Shim of src/contracts/tag-extraction.ts.
import type { ConstDirectiveNode, DocumentNode } from "graphql";
import type { SchemaContractFilter } from "../../src/contracts/tag-extraction.js";
import { call, encodeJSON, parseJSON, toJSON } from "../runtime.js";

export type { SchemaContractFilter };

type SchemaCoordinateToTagsRegistry = Map<string, Set<string>>;

/** What the Rust exports return for one subgraph. */
interface TagFilterResultJSON {
  typeDefs: DocumentNode;
  typesWithAllFieldsInaccessible: Array<[string, boolean]>;
  typesWithInaccessibleApplied: string[];
  tagDirectiveName: string;
  inaccessibleDirectiveName: string;
}

export function filterToJSON(filter: SchemaContractFilter): string {
  return JSON.stringify({
    include: Array.from(filter.include),
    exclude: Array.from(filter.exclude),
  });
}

function fromTagFilterResultJSON(result: TagFilterResultJSON) {
  return {
    typeDefs: result.typeDefs,
    typesWithAllFieldsInaccessible: new Map(
      result.typesWithAllFieldsInaccessible,
    ),
    transformTagDirectives: function transformTagDirectives(
      node: { directives?: readonly ConstDirectiveNode[] },
      includeInaccessibleDirective: boolean = false,
    ): readonly ConstDirectiveNode[] {
      return parseJSON<ConstDirectiveNode[]>(
        call(
          "transform_tag_directives",
          result.tagDirectiveName,
          result.inaccessibleDirectiveName,
          toJSON({ directives: node.directives }),
          Boolean(includeInaccessibleDirective),
        ),
      );
    },
    typesWithInaccessibleApplied: new Set(result.typesWithInaccessibleApplied),
  };
}

/**
 * Get a map with tags per schema coordinates in all subgraphs.
 */
export function buildSchemaCoordinateTagRegister(
  documentNodes: Array<DocumentNode>,
): SchemaCoordinateToTagsRegistry {
  const entries = parseJSON<Array<[string, string[]]>>(
    call("build_schema_coordinate_tag_register", encodeJSON(documentNodes)),
  );
  return new Map(
    entries.map(([coordinate, tags]) => [coordinate, new Set(tags)]),
  );
}

export function applyTagFilterToInaccessibleTransformOnSubgraphSchema(
  documentNode: DocumentNode,
  tagRegister: SchemaCoordinateToTagsRegistry,
  filter: SchemaContractFilter,
) {
  const register = Array.from(tagRegister, ([coordinate, tags]) => [
    coordinate,
    Array.from(tags),
  ]);
  return fromTagFilterResultJSON(
    parseJSON<TagFilterResultJSON>(
      call(
        "apply_tag_filter_to_inaccessible_transform_on_subgraph_schema",
        encodeJSON(documentNode),
        JSON.stringify(register),
        filterToJSON(filter),
      ),
    ),
  );
}

/**
 * Apply a tag filter to a set of subgraphs, transforming `@tag` to `@inaccessible`
 */
export function applyTagFilterOnSubgraphs<
  TType extends {
    typeDefs: DocumentNode;
  },
>(subgraphs: Array<TType>, filter: SchemaContractFilter): Array<TType> {
  const results = parseJSON<TagFilterResultJSON[]>(
    call(
      "apply_tag_filter_on_subgraphs",
      encodeJSON(subgraphs.map((s) => s.typeDefs)),
      filterToJSON(filter),
    ),
  );
  return subgraphs.map((subgraph, i) => ({
    ...subgraph,
    ...fromTagFilterResultJSON(results[i]),
  }));
}
