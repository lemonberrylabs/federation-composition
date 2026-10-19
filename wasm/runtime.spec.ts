// Checks of the JS <-> Rust bridge itself (not of the composition port).
import { GraphQLError, parse } from "graphql";
import { describe, expect, test } from "vitest";
import { composeServices } from "./compose.js";
import {
  call,
  parseJSON,
  toGraphQLErrors,
  toJSON,
  type RustPanicError,
} from "./runtime.js";

const sdl = /* GraphQL */ `
  extend schema
    @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])
  "described"
  type Query @key(fields: "id") {
    id: ID!
    list(first: Int = 10): [String!]
  }
`;

describe("wasm bridge", () => {
  test("documents cross as graphql-js JSON, without loc", () => {
    const doc = parse(sdl);
    expect(toJSON(doc)).not.toContain('"loc"');
    expect(call("debug_roundtrip_document", toJSON(doc))).toEqual(
      JSON.stringify(parse(sdl, { noLocation: true })),
    );
  });

  test("a thrown GraphQLError is rebuilt as one", () => {
    let thrown: unknown;
    try {
      call("debug_parse", "{");
    } catch (e) {
      thrown = e;
    }
    expect(thrown).toBeInstanceOf(GraphQLError);
    expect((thrown as GraphQLError).message).toBe(
      "Syntax Error: Expected Name, found <EOF>.",
    );
  });

  test("errors returned as results become GraphQLErrors with extensions", () => {
    const { errors } = parseJSON<{ errors: never[] }>(
      call("debug_errors", "Bad.", "INVALID_GRAPHQL"),
    );
    const [error] = toGraphQLErrors(errors);
    expect(error).toBeInstanceOf(GraphQLError);
    expect(error.message).toBe("Bad.");
    expect(error.extensions).toEqual({ code: "INVALID_GRAPHQL" });
  });

  test("a panic becomes an Error with the panic message, and the next call works", () => {
    let thrown: RustPanicError | undefined;
    try {
      call("debug_panic", "something broke");
    } catch (e) {
      thrown = e as RustPanicError;
    }
    expect(thrown).toBeInstanceOf(Error);
    expect(thrown!.message).toBe("something broke");
    expect(thrown!.rustPanicLocation).toMatch(/exports\.rs:\d+:\d+$/);
    expect(thrown!.cause).toBeInstanceOf(WebAssembly.RuntimeError);
    // Repeated panics do not exhaust the (reloaded) instance.
    for (let i = 0; i < 50; i++) {
      expect(() => call("debug_panic", `panic ${i}`)).toThrow(`panic ${i}`);
    }
    expect(
      call("debug_roundtrip_document", toJSON(parse("scalar A"))),
    ).toContain("ScalarTypeDefinition");
  });

  test("an invalid argument is reported, not a panic", () => {
    expect(() =>
      call(
        "debug_roundtrip_document",
        '{"kind":"Document","definitions":[{"kind":"Nope"}]}',
      ),
    ).toThrow(/^composition-wasm: invalid document argument: /);
  });

  test("a runtime error keeps its class", () => {
    expect(() => call("debug_type_error", "x is undefined")).toThrow(TypeError);
  });

  test("composition results are rebuilt with lazy getters", () => {
    const result = composeServices([{ name: "a", typeDefs: parse(sdl) }]);
    expect(result.errors).toBeUndefined();
    expect(result.supergraphSdl).toContain("join__Graph");
    expect(result.publicSdl).toContain("type Query");
  });
});
