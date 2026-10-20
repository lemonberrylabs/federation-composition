// Checks of the JS <-> Rust bridge itself (not of the composition port), on the built
// package (`pnpm wasm:build` first). Run with `pnpm test:npm`.
import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { GraphQLError, parse } from "graphql";
import { composeServices } from "../dist/compose.js";
import { call, parseJSON, toGraphQLErrors, toJSON } from "../dist/runtime.js";

const sdl = /* GraphQL */ `
  extend schema
    @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])
  "described"
  type Query @key(fields: "id") {
    id: ID!
    list(first: Int = 10): [String!]
  }
`;

/** What `fn` throws; fails if it does not throw. */
function thrownBy(fn) {
  try {
    fn();
  } catch (e) {
    return e;
  }
  assert.fail("expected a throw");
}

describe("wasm bridge", () => {
  test("documents cross as graphql-js JSON, without loc", () => {
    const doc = parse(sdl);
    assert.ok(!toJSON(doc).includes('"loc"'));
    assert.deepEqual(
      call("debug_roundtrip_document", toJSON(doc)),
      JSON.stringify(parse(sdl, { noLocation: true })),
    );
  });

  test("a thrown GraphQLError is rebuilt as one", () => {
    const thrown = thrownBy(() => call("debug_parse", "{"));
    assert.ok(thrown instanceof GraphQLError);
    assert.equal(thrown.message, "Syntax Error: Expected Name, found <EOF>.");
  });

  test("errors returned as results become GraphQLErrors with extensions", () => {
    const { errors } = parseJSON(
      call("debug_errors", "Bad.", "INVALID_GRAPHQL"),
    );
    const [error] = toGraphQLErrors(errors);
    assert.ok(error instanceof GraphQLError);
    assert.equal(error.message, "Bad.");
    assert.deepEqual(error.extensions, { code: "INVALID_GRAPHQL" });
  });

  test("a panic becomes an Error with the panic message, and the next call works", () => {
    const thrown = thrownBy(() => call("debug_panic", "something broke"));
    assert.ok(thrown instanceof Error);
    assert.equal(thrown.message, "something broke");
    assert.match(thrown.rustPanicLocation, /exports\.rs:\d+:\d+$/);
    assert.ok(thrown.cause instanceof WebAssembly.RuntimeError);
    // Repeated panics do not exhaust the (reloaded) instance.
    for (let i = 0; i < 50; i++) {
      const e = thrownBy(() => call("debug_panic", `panic ${i}`));
      assert.ok(e.message.includes(`panic ${i}`));
    }
    assert.ok(
      call("debug_roundtrip_document", toJSON(parse("scalar A"))).includes(
        "ScalarTypeDefinition",
      ),
    );
  });

  test("an invalid argument is reported, not a panic", () => {
    assert.throws(
      () =>
        call(
          "debug_roundtrip_document",
          '{"kind":"Document","definitions":[{"kind":"Nope"}]}',
        ),
      { message: /^composition-wasm: invalid document argument: / },
    );
  });

  test("a runtime error keeps its class", () => {
    assert.throws(() => call("debug_type_error", "x is undefined"), TypeError);
  });

  test("composition results are rebuilt with lazy getters", () => {
    const result = composeServices([{ name: "a", typeDefs: parse(sdl) }]);
    assert.equal(result.errors, undefined);
    assert.ok(result.supergraphSdl.includes("join__Graph"));
    assert.ok(result.publicSdl.includes("type Query"));
  });

  // The limits of crates/federation-composition-wasm/src/lib.rs (SECURITY.md, "Input limits").
  const header =
    'extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])\n';
  /** A field whose type, argument default value and directive argument nest `n` levels. */
  function nested(n) {
    const list = (inner) => "[".repeat(n) + inner + "]".repeat(n);
    const object = "{a: ".repeat(n - 1) + "{b: 1}" + "}".repeat(n - 1);
    return `${header}input I { a: I b: Int }\ndirective @d(a: I) on FIELD_DEFINITION
      type Query { f(a: ${list("Int")} = ${list("1")}): ${list("Int")} @d(a: ${object}) }`;
  }

  test("documents nested to the parser's limit compose, deeper ones throw", () => {
    const result = composeServices([
      { name: "a", typeDefs: parse(nested(128)) },
    ]);
    assert.equal(result.errors, undefined);
    assert.ok(result.supergraphSdl.includes("[".repeat(128) + "1"));
    assert.throws(
      () => composeServices([{ name: "a", typeDefs: parse(nested(129)) }]),
      (e) =>
        e instanceof GraphQLError &&
        e.message === "AST nesting depth exceeds 128.",
    );
  });

  test("an argument nested deeper than the JSON limit throws, not a trap", () => {
    const sdl = `${header}type Query { f: ${"[".repeat(500)}Int${"]".repeat(500)} }`;
    assert.throws(
      () => composeServices([{ name: "a", typeDefs: parse(sdl) }]),
      {
        message:
          "composition-wasm: invalid services argument: nested deeper than 448 levels",
      },
    );
  });

  test("a long chain of types composes", () => {
    let sdl = `${header}type Query { t: T0 }\n`;
    for (let i = 0; i < 5000; i++)
      sdl += `type T${i} { id: ID a: T${i + 1} }\n`;
    sdl += "type T5000 { id: ID }\n";
    const result = composeServices([{ name: "a", typeDefs: parse(sdl) }]);
    assert.equal(result.errors, undefined);
    assert.ok(result.supergraphSdl.includes("type T5000"));
  });
});
