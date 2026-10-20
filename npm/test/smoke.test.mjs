// Smoke test of the package as a user imports it (by its name, through package.json
// "exports"), without the upstream test suite. Run with `pnpm test:npm` after `pnpm wasm:build`.
import assert from "node:assert/strict";
import { test } from "node:test";
import { GraphQLError, parse } from "graphql";
import * as api from "@lemonberrylabs/federation-composition-wasm";
import {
  assertCompositionSuccess,
  composeSchemaContract,
  composeServices,
  compositionHasErrors,
  containsSupergraphSpec,
  sortSDL,
  transformSupergraphToPublicSchema,
  validate,
  validateSubgraph,
} from "@lemonberrylabs/federation-composition-wasm";

const users = {
  name: "users",
  typeDefs: parse(/* GraphQL */ `
    extend schema
      @link(
        url: "https://specs.apollo.dev/federation/v2.3"
        import: ["@key", "@tag"]
      )

    type User @key(fields: "id") {
      id: ID!
      name: String! @tag(name: "public")
      email: String!
    }

    type Query {
      users: [User!]! @tag(name: "public")
    }
  `),
};

const reviews = {
  name: "reviews",
  typeDefs: parse(/* GraphQL */ `
    extend schema
      @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])

    type User @key(fields: "id") {
      id: ID!
      reviews: [String!]!
    }
  `),
};

test("exports the public API of @theguild/federation-composition", () => {
  // src/index.ts of upstream at the UPSTREAM release.
  assert.deepEqual(Object.keys(api).sort(), [
    "FEDERATION_V1",
    "addInaccessibleToUnreachableTypes",
    "applyTagFilterOnSubgraphs",
    "assertCompositionFailure",
    "assertCompositionSuccess",
    "composeSchemaContract",
    "composeServices",
    "compositionHasErrors",
    "containsSupergraphSpec",
    "createCompositionSuccessReadCacheContainer",
    "extractLinkImplementations",
    "sortSDL",
    "transformSupergraphToPublicSchema",
    "validate",
    "validateSubgraph",
  ]);
});

test("composes a supergraph", () => {
  const result = composeServices([users, reviews]);
  assertCompositionSuccess(result);
  assert.ok(containsSupergraphSpec(result.supergraphSdl));
  assert.match(
    result.supergraphSdl,
    /enum join__Graph \{\s+REVIEWS @join__graph\(name: "reviews", url: ""\)\s+USERS/,
  );
  assert.match(
    result.publicSdl,
    /type User \{\s+id: ID!\s+reviews: \[String!\]!\s+name: String!\s+email: String!\s+\}/,
  );
  assert.equal(
    transformSupergraphToPublicSchema(result.supergraphDocumentNode).kind,
    "Document",
  );
  assert.equal(sortSDL(result.publicDocumentNode).kind, "Document");
});

test("reports composition errors as GraphQLErrors", () => {
  const result = composeServices([
    users,
    {
      name: "other",
      typeDefs: parse(/* GraphQL */ `
        extend schema
          @link(
            url: "https://specs.apollo.dev/federation/v2.3"
            import: ["@key"]
          )

        type User @key(fields: "id") {
          id: ID!
          name: Int!
        }
      `),
    },
  ]);
  assert.ok(compositionHasErrors(result));
  assert.ok(result.errors[0] instanceof GraphQLError);
  assert.equal(result.errors[0].extensions.code, "FIELD_TYPE_MISMATCH");
});

test("validates", () => {
  assert.deepEqual(validateSubgraph(users), []);
  const result = validate([users, reviews]);
  assert.equal(result.success, true);
  assert.equal(result.federationVersion, "v2.3");
});

test("composes a schema contract", () => {
  const result = composeSchemaContract(
    [users, reviews],
    { include: new Set(["public"]), exclude: new Set() },
    true,
  );
  assertCompositionSuccess(result);
  assert.ok(!result.publicSdl.includes("email"));
  assert.ok(result.publicSdl.includes("name: String!"));
});
