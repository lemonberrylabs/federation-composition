// Builds crates/graphql-rs/tests/schema-corpus.json: SDL documents and what
// graphql-js 17.0.2's buildSchema / validateSchema / assertValidSchema do with
// them, the expected values of crates/graphql-rs/tests/schema_differential.rs.
//
// Run from the repository root, after `pnpm conformance` (graphql-js, upstream's
// tests and the composition corpus come from it):
//   node crates/graphql-rs/tests/gen-schema-corpus.mjs [path/to/corpus.jsonl]
// The composition corpus defaults to conformance/out/corpus.jsonl.
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../../..");
// graphql-js and the test documents come from the upstream checkout of `pnpm conformance`.
const upstream = process.env.UPSTREAM_DIR ?? path.join(repo, "conformance/upstream");
if (!fs.existsSync(path.join(upstream, "node_modules/graphql"))) {
  throw new Error(`${upstream} has no node_modules/graphql: run \`pnpm conformance\` first`);
}
const require = createRequire(path.join(upstream, "package.json"));
const g = require("graphql");
if (g.version !== "17.0.2") throw new Error(`expected graphql 17.0.2, got ${g.version}`);
const {
  buildSchema,
  validateSchema,
  assertValidSchema,
  print,
  isScalarType,
  isObjectType,
  isInterfaceType,
  isUnionType,
  isEnumType,
  isInputObjectType,
  isIntrospectionType,
} = g;

const corpusPath = process.argv[2] ?? path.join(repo, "conformance/out/corpus.jsonl");
if (!fs.existsSync(corpusPath)) {
  throw new Error(`${corpusPath} is missing; run \`pnpm conformance --only=capture\` or pass its path`);
}

// Messages may contain lone surrogates; converted to UTF-8 they become U+FFFD.
const message = (e) => String(e.message).toWellFormed();

const cases = [];
const seen = new Set();
function add(src, sdl) {
  if (seen.has(sdl)) return;
  seen.add(sdl);
  cases.push({ src, sdl });
}

function walk(dir, f) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    e.isDirectory() ? walk(p, f) : f(p);
  }
}

// (a) Every supergraph and public SDL of the composition corpus.
for (const line of fs.readFileSync(corpusPath, "utf8").split("\n")) {
  if (!line) continue;
  const { id, expected } = JSON.parse(line);
  if (expected?.supergraphSdl) add(`corpus ${id} supergraphSdl`, expected.supergraphSdl);
  if (expected?.publicSdl) add(`corpus ${id} publicSdl`, expected.publicSdl);
}
const corpusCount = cases.length;

// (b) The documents of the TypeScript tests (subgraph SDL, mostly invalid as a
// standalone schema: federation directives are not defined) and fixtures.
walk(path.join(upstream, "__tests__/fixtures"), (p) => {
  if (p.endsWith(".graphql")) add(path.relative(upstream, p), fs.readFileSync(p, "utf8"));
});
const templateRe = /graphql`([^`]*)`/g;
walk(path.join(upstream, "__tests__"), (p) => {
  if (!p.endsWith(".ts")) return;
  const source = fs.readFileSync(p, "utf8");
  for (const m of source.matchAll(templateRe)) {
    const raw = m[1].replace(/\$\{[^}]*\}/g, "v2.3");
    let cooked = raw;
    try {
      cooked = new Function("return `" + raw + "`")();
    } catch {}
    add(path.relative(upstream, p), cooked);
  }
});
const testsCount = cases.length - corpusCount;

// (c) Hand-written schemas reaching every SDL validation rule, every
// buildASTSchema throw and every validateSchema error. graphql-js's own test
// suite is not part of the published package.
const handWritten = [
  // Valid schemas.
  `type Query { a: String }`,
  `"desc" schema { query: Q mutation: M subscription: S } type Q { a: Int } type M { b: Int } type S { c: Int }`,
  `type Query { a: String } extend schema { mutation: M } type M { a: Int }`,
  `extend schema { query: Foo } type Foo { a: Int } type Query { b: Int }`,
  `extend schema { query: Foo } type Foo { a: Int }`,
  `type Query { a: String } type Mutation { a: Int } type Subscription { a: Int }`,
  `type Query { a(x: Int = 1, y: [Int] = 1, z: [[Int!]] = [[1], 2], w: In = { a: 1 }): String } input In { a: Int b: String = "x" c: [In] }`,
  `type Query { a: String @deprecated b: String @deprecated(reason: "because") c(x: Int @deprecated): Int }`,
  `type Query { a(e: E = A): E } enum E { A B @deprecated(reason: "no") C @deprecated }`,
  `type Query { a: S } scalar S @specifiedBy(url: "https://x") extend scalar S @specifiedBy(url: "https://y")`,
  `type Query { a: S } scalar S extend scalar S @specifiedBy(url: "https://y")`,
  `scalar String @specifiedBy(url: 1) type Query { a: String }`,
  `type Query { a: Boolean, b: __Type, c: __Schema }`,
  `type __Schema { a: Int } type Query { a: __Schema }`,
  `type String { a: Int } type Query { a: String }`,
  `type Query { a(i: In): Int } input In @oneOf { a: Int b: String }`,
  `type Query { a(i: In = { a: 1 }): Int } input In @oneOf { a: Int b: String }`,
  `directive @d(a: Int = 1, b: String @deprecated(reason: "x")) repeatable on FIELD_DEFINITION | OBJECT type Query @d @d { a: Int @d(a: 2) @d }`,
  `directive @d @deprecated(reason: "old") on FIELD_DEFINITION type Query { a: Int @d }`,
  `directive @d on FIELD_DEFINITION extend directive @d @deprecated(reason: "ext") type Query { a: Int @d }`,
  `directive @d on FIELD_DEFINITION extend directive @d @deprecated extend directive @d @deprecated(reason: "second") type Query { a: Int @d }`,
  `extend directive @include @deprecated type Query { a: Int }`,
  `directive @include(if: Int) on FIELD type Query { a: Int }`,
  `directive @deprecated(reason: Int) on FIELD_DEFINITION type Query { a: Int @deprecated(reason: 5) }`,
  `directive @deprecated(reason: String) on FIELD_DEFINITION type Query { a: Int @deprecated(reason: null) }`,
  `directive @specifiedBy(url: String) on SCALAR scalar S @specifiedBy type Query { a: S }`,
  `type Query { a: I } interface I { a: Int } type A implements I { a: Int } type B implements I & J { a: Int b: Int } interface J { b: Int }`,
  `type Query { a: I } interface I implements J { a: Int b: Int } interface J { b: Int } type A implements I & J { a: Int! b: Int }`,
  `type Query { u: U } union U = A | B type A { a: Int } type B { b: Int } extend union U = C type C { c: Int }`,
  `type Query { a: I } interface I { f: U } union U = A type A implements I { f: A }`,
  `type Query { a: I } interface I { f: [I] } type A implements I { f: [A!]! }`,
  `type Query { a: Int } extend type Query { b: Int } extend type Query implements I { c: Int } interface I { c: Int }`,
  `type Query { a(x: In): Int } input In { a: Int } extend input In { b: String = "x" }`,
  `type Query { a(x: E): Int } enum E { A } extend enum E { B }`,
  `type Query { a: Int } query { a } fragment F on Query { a }`,
  `query { a } type Query { a: Int } subscription { a } mutation { b }`,
  `type Query { a(x: [Int] = [1, 2]): Int, b(x: [Int!]! = 1): Int, c(x: ID = 1, y: ID = "a", z: Float = 1, w: Float = 1.5, v: Boolean = true): Int }`,
  `type Query { a(x: S = { a: [1, { b: null }] }, y: S = $v): Int } scalar S`,
  `type Query { a(x: In = { a: { b: 1 } }): Int } input In { a: In2 } input In2 { b: Int }`,
  `type Query { a: Int } input A { b: B } input B { a: A }`,
  `type Query { a: Int } input A { b: [A!]! }`,
  `type Query { a: Int } input A @oneOf { a: A b: Int }`,
  `type Query { a: Int } input A { b: B = { c: 1 } } input B { c: Int a: A }`,
  // LoneSchemaDefinitionRule
  `schema { query: Query } schema { query: Query } type Query { a: Int }`,
  `schema { query: Query } schema { query: Query } schema { mutation: Query } type Query { a: Int }`,
  // UniqueOperationTypesRule
  `schema { query: Query query: Query } type Query { a: Int }`,
  `schema { query: Query mutation: Query } extend schema { mutation: Query subscription: Query } extend schema { subscription: Query } type Query { a: Int }`,
  // UniqueTypeNamesRule
  `type Query { a: Int } type Query { b: Int }`,
  `type Query { a: Int } scalar A enum A { X } input A { a: Int } union A = Query interface A { a: Int } type A { a: Int }`,
  `scalar String type Query { a: String } scalar String`,
  // UniqueEnumValueNamesRule
  `type Query { a: E } enum E { A A B } extend enum E { B C } extend enum E { C }`,
  `type Query { a: Int } extend enum E { A A }`,
  // UniqueFieldDefinitionNamesRule
  `type Query { a: Int a: Int } extend type Query { a: Int b: Int } interface I { x: Int x: Int } extend interface I { x: Int } input In { x: Int x: Int } extend input In { x: Int }`,
  // UniqueArgumentDefinitionNamesRule
  `directive @d(a: Int, a: Int, b: Int, b: String, b: Int) on FIELD type Query { f(x: Int, x: Int): Int g(y: Int, y: Int): Int } extend type Query { h(z: Int z: Int): Int } interface I { f(x: Int x: Int): Int } extend interface I { g(x: Int x: Int): Int }`,
  // UniqueDirectiveNamesRule
  `directive @d on FIELD directive @d on OBJECT directive @d on SCALAR type Query { a: Int }`,
  `directive @include on FIELD directive @include on FIELD type Query { a: Int }`,
  // KnownTypeNamesRule
  `type Query { a: Strin, b: [Strng!], c: Inte, d: Query2 }`,
  `type Query { a: Unknown }`,
  `type Query { a(x: Inpt): Int } input Input { a: Int }`,
  `type Query implements Nod { a: Int } interface Node { a: Int }`,
  `type Query { a: Int } union U = Aa | Query`,
  `schema { query: Qery } type Query { a: Int }`,
  `type Query { a: Int } extend schema { mutation: Mutaton }`,
  `directive @d(a: Strng) on FIELD type Query { a: Int }`,
  `type Query { a: Int } query ($a: String, $b: Query, $c: Qury) { a }`,
  `type Query { a: Int } fragment F on Qury { a } query { ... on Strin { a } }`,
  `type Query { a: __Typ }`,
  `type Query { a: Int } extend type Query { b: Foo } extend input In { a: Bar } input In { b: Int }`,
  // KnownDirectivesRule
  `type Query @unknown { a: Int @keyy } directive @key on OBJECT`,
  `directive @onField on FIELD type Query @onField { a: Int @onField }`,
  `directive @d on FIELD_DEFINITION
   schema @d { query: Query }
   extend schema @d
   scalar S @d
   extend scalar S @d
   type Query @d { a(x: Int @d): S @d }
   extend type Query @d
   interface I @d { a: Int }
   extend interface I @d
   union U @d = Query
   extend union U @d
   enum E @d { A @d }
   extend enum E @d
   input In @d { a: Int @d }
   extend input In @d
   directive @e(a: Int @d) @d on FIELD
   extend directive @e @d`,
  `directive @d on SCHEMA
   query Q($v: Int @d) @d { a @d ...F @d ... on Query @d { a } }
   mutation @d { a }
   subscription @d { a }
   fragment F on Query @d { a }
   type Query { a: Int }`,
  `directive @d on QUERY | MUTATION | SUBSCRIPTION | FIELD | FRAGMENT_DEFINITION | FRAGMENT_SPREAD | INLINE_FRAGMENT | VARIABLE_DEFINITION
   query Q($v: Int @d) @d { a @d ...F @d ... on Query @d { a } }
   fragment F on Query @d { a }
   type Query { a: Int }`,
  `type Query { a: Int @include(if: true) @skip(if: false) @specifiedBy(url: "x") @oneOf }`,
  `scalar S @deprecated type Query { a: S } input In @deprecated { a: Int }`,
  // UniqueDirectivesPerLocationRule
  `directive @d on OBJECT | FIELD_DEFINITION | SCHEMA | DIRECTIVE_DEFINITION | ARGUMENT_DEFINITION
   directive @r repeatable on OBJECT
   type Query @d @d @r @r { a(x: Int @d @d): Int @d @d @deprecated @deprecated }
   extend type Query @d @r
   schema @d { query: Query }
   extend schema @d
   directive @e @d on FIELD
   extend directive @e @d`,
  `directive @d on OBJECT type Query @d { a: Int } extend type Query @d extend type Query @d`,
  `type Query { a: Int } query { a @include(if: true) @include(if: false) }`,
  `type Query { a: Int @deprecated @deprecated(reason: "x") }`,
  // PossibleTypeExtensionsRule
  `type Query { a: Int }
   scalar S type O { a: Int } interface I { a: Int } union U = O enum E { A } input In { a: Int }
   extend type S @x extend interface O { b: Int } extend union I = O extend enum U { B } extend input E { b: Int } extend scalar In @x`,
  `type Query { a: Int } scalar S extend type S { a: Int } extend input S { a: Int }`,
  `type Query { a: Int } extend type Qery { b: Int } extend scalar Unknown extend input Querx { a: Int }`,
  `type Query { a: Int } extend type String { a: Int }`,
  // KnownArgumentNamesOnDirectivesRule
  `type Query { a: Int @deprecated(reasn: "x") @deprecated(foo: 1) }`,
  `directive @d(abc: Int, abd: Int) on FIELD_DEFINITION type Query { a: Int @d(ab: 1, xyz: 2) }`,
  `type Query { a: Int } query { a @include(iff: true) }`,
  // UniqueArgumentNamesRule
  `type Query { a: Int @deprecated(reason: "a", reason: "b") }`,
  `type Query { a(x: Int): Int } query { a(x: 1, x: 2, y: 1, y: 2) }`,
  // UniqueInputFieldNamesRule
  `directive @d(a: In) on FIELD_DEFINITION input In { a: Int b: In } type Query { a(x: In = { a: 1, a: 2, b: { a: 1, a: 3 } }): Int @d(a: { b: { a: 1 }, b: { a: 2 } }) }`,
  `type Query { a: Int } query { a(x: { a: 1, a: 2 }) }`,
  // ProvidedRequiredArgumentsOnDirectivesRule
  `directive @d(a: Int!, b: String! = "x", c: [Int]!) on FIELD_DEFINITION type Query { a: Int @d }`,
  `scalar S @specifiedBy type Query { a: S }`,
  `type Query { a: Int } query { a @include @skip }`,
  `directive @d(a: Int!) on FIELD_DEFINITION type Query { a: Int @d(b: 1) }`,
  // Several rules at once (error order follows document order).
  `type Query { a: Unknown @unknown, a: Int } type Query { b: Int } directive @d on FIELD directive @d on FIELD`,
  // buildASTSchema throws: invalid @deprecated / @specifiedBy arguments.
  `type Query { a: Int @deprecated(reason: 1) }`,
  `type Query { a: Int @deprecated(reason: null) }`,
  `type Query { a: Int @deprecated(reason: ["x"]) }`,
  `type Query { a: Int @deprecated(reason: { a: 1 }) }`,
  `type Query { a: Int @deprecated(reason: X) }`,
  `type Query { a: Int @deprecated(reason: true) }`,
  `type Query { a(x: Int @deprecated(reason: 1.5)): Int }`,
  `type Query { a(x: In): Int } input In { a: Int @deprecated(reason: 2) }`,
  `type Query { a: Int } directive @d(a: Int @deprecated(reason: 3)) on FIELD`,
  `type Query { a: Int } directive @d @deprecated(reason: 4) on FIELD`,
  `type Query { a: Int } directive @d on FIELD extend directive @d @deprecated(reason: 5)`,
  `type Query { a: Int } directive @d @deprecated on FIELD extend directive @d @deprecated(reason: 6)`,
  `type Query { a: S } scalar S @specifiedBy(url: 1)`,
  `type Query { a: S } scalar S @specifiedBy(url: null)`,
  `type Query { a: S } scalar S extend scalar S @specifiedBy(url: 2)`,
  `directive @specifiedBy(url: String) on SCALAR type Query { a: S } scalar S @specifiedBy`,
  `directive @specifiedBy(url: Int) on SCALAR type Query { a: S } scalar S @specifiedBy(url: 1)`,
  `type Query { a: Int @deprecated(reason: 1) b: Int @deprecated(reason: 2) }`,
  `type Query { a: S } scalar S @specifiedBy(url: 1) type T { a: Int @deprecated(reason: 2) }`,
  `type A { a: Int @deprecated(reason: 2) } type Query { a: Int @deprecated(reason: 1) }`,
  // Enum values are resolved lazily: validateSchema throws.
  `type Query { a: E } enum E { A @deprecated(reason: 1) }`,
  `type Query { a: E } enum E { A } extend enum E { B @deprecated(reason: null) }`,
  `type Query { a(e: E = A): E } enum E { A @deprecated(reason: 1) }`,
  `directive @d(e: E = B) on FIELD type Query { a: E } enum E { A @deprecated(reason: 1) }`,
  `type Query { a: E b: F } enum F { X @deprecated(reason: 2) } enum E { A @deprecated(reason: 1) }`,
  `type Query { a: E __b: Int } enum E { A @deprecated(reason: 1) }`,
  // validateRootTypes
  `type Foo { a: Int }`,
  `schema { mutation: M } type M { a: Int }`,
  `schema { query: In } input In { a: Int }`,
  `schema { query: Query mutation: In subscription: E } type Query { a: Int } input In { a: Int } enum E { A }`,
  `schema { query: S } scalar S`,
  `schema { query: Query mutation: Query } type Query { a: Int }`,
  `schema { query: Query mutation: Query subscription: Query } type Query { a: Int }`,
  `schema { query: Query mutation: M subscription: M } type Query { a: Int } type M { a: Int }`,
  `extend schema { query: Q } type Q { a: Int }`,
  `extend schema { mutation: Query } type Query { a: Int }`,
  `type Query { a: Int } type Mutation { a: Int } extend schema { subscription: Mutation }`,
  // validateDirectives
  `directive @__d on FIELD type Query { a: Int }`,
  `directive @d(__a: Int, b: Query, c: [Query!], d: Int! @deprecated, e: Int! = 1 @deprecated, f: Int = "x", g: [Int] = ["a", 1, "b"]) on FIELD type Query { a: Int }`,
  `directive @d(a: In = {}) on FIELD input In { a: Int! b: Int } type Query { a: Int }`,
  // validateName on types, fields, arguments, enum values, input fields
  `type Query { __a: Int a(__b: Int): Int } type __T { a: Int } enum __E { __A } input __In { __a: Int } union __U = Query interface __I { a: Int } scalar __S`,
  // validateFields
  `type Query`,
  `type Query { a: Int } interface I extend interface I @deprecated`,
  `type Query { a: In b: [In!] c(x: Query, y: [[I]]): Int d(x: Int! @deprecated): Int } input In { a: Int } interface I { a: Int }`,
  `type Query { a(x: Int = "a", y: [Int!] = [1, null], z: Int! = null): Int }`,
  // Default values: validateInputLiteral messages and paths.
  `type Query { a(x: In = 1, y: In = { b: 1 }, z: In = { a: 1, c: 1 }, w: In = { a: 1, ab: 2 }, v: [In] = [{ a: 1 }, { a: "x" }], u: In = { a: 1, n: { a: "y" } }): Int } input In { a: Int! ab: Int n: In }`,
  `type Query { a(x: E = "A", y: E = B, z: E = Aa, w: [E] = [A, X], v: E = 1, u: E = null): Int } enum E { A AB }`,
  `type Query { a(x: Int = 2147483648, y: Int = -2147483649, z: Int = 1.5, w: Float = "1", v: String = 1, u: Boolean = 1, t: ID = 1.5, s: ID = true, r: Int = 2147483647): Int }`,
  `type Query { a(x: O = {}, y: O = { a: 1, b: 2 }, z: O = { a: null }, w: O = { c: 1 }, v: [O] = [{ a: 1 }, { a: null }]): Int } input O @oneOf { a: Int b: Int }`,
  `type Query { a(x: [[Int]] = [[1, "a"], ["b"]], y: [[Int!]!] = [null], z: [In] = { a: { a: 1 } }): Int } input In { a: Int }`,
  `type Query { a(x: Int = { a: 1 }, y: In = [1], z: S = 1): Int } input In { a: Int } scalar S`,
  `type Query { a(x: In = { a: 1 }): Int } input In { a: Int b: Int! c: Int! = 1 }`,
  `type Query { a(x: In = { aa: 1, bb: 2 }): Int } input In { a: Int b: Int ab: Int }`,
  `type Query { a(x: [In!] = [{ a: { b: { c: 1 } } }]): Int } input In { a: In2 } input In2 { b: In3 } input In3 { c: String }`,
  // validateInterfaces
  `type Query { a: Int } type A implements Query { a: Int }`,
  `type Query { a: Int } interface I implements I { a: Int }`,
  `type Query { a: Int } interface I { a: Int } type A implements I & I { a: Int }`,
  `type Query { a: Int } interface I { a: Int } type A implements I { a: Int } extend type A implements I`,
  `type Query { a: Int } interface I implements J { a: Int b: Int } interface J { b: Int } type A implements I { a: Int b: Int }`,
  `type Query { a: Int } interface I implements J { a: Int } interface J implements I { a: Int }`,
  `type Query { a: Int } interface I implements J & K { a: Int } interface J { a: Int } interface K implements J { a: Int } type A implements I { a: Int }`,
  `type Query { a: Int } interface I implements Query { a: Int } type A implements I { a: Int }`,
  `type Query { a: Int } interface I { a: Int b: String } type A implements I { a: Int }`,
  `type Query { a: Int } interface I { a: Int b: String c: [Int] d: Int! e: I f: U g: [Int!]! } union U = A type A implements I { a: String b: Int c: Int d: Int e: A f: A g: [Int]! }`,
  `type Query { a: Int } interface I { a: I } interface J implements I { a: I } type A implements I & J { a: J }`,
  `type Query { a: Int } interface I { a(x: Int, y: String): Int } type A implements I { a(x: String): Int }`,
  `type Query { a: Int } interface I { a(x: [Int]): Int } type A implements I { a(x: [Int!], y: Int!, z: Int! = 1, w: Int): Int }`,
  `type Query { a: Int } interface I { a: Int b: Int @deprecated } type A implements I { a: Int @deprecated b: Int @deprecated }`,
  `type Query { a: Int } interface I { a: Int } type A implements I { a: Int @deprecated(reason: "x") } type B implements I { a: Int }`,
  // validateUnionMembers
  `type Query { a: Int } union U`,
  `type Query { a: Int } union U = Query | Query | I | In | S | E | U extend union U = Query interface I { a: Int } input In { a: Int } scalar S enum E { A }`,
  // validateEnumValues
  `type Query { a: E } enum E`,
  `type Query { a: E } enum E extend enum E @deprecated`,
  // validateInputFields
  `type Query { a: Int } input In`,
  `type Query { a: Int } input In { a: Query b: [I] c: Int! @deprecated d: Int! = 1 @deprecated e: Int = "x" } interface I { a: Int }`,
  `type Query { a: Int } input In @oneOf { a: Int! b: Int = 1 c: Int! = 2 }`,
  `type Query { a: Int } input In @oneOf`,
  // detectInputObjectNonFiniteValues
  `type Query { a: Int } input A { a: A! }`,
  `type Query { a: Int } input A { b: B! } input B { c: C! } input C { a: A! }`,
  `type Query { a: Int } input A { b: B! c: C! } input B { a: A! } input C { a: A! }`,
  `type Query { a: Int } input A { b: B! x: Int } input B { a: A! } input C { a: A! }`,
  `type Query { a: Int } input A @oneOf { a: A b: B } input B @oneOf { a: A }`,
  `type Query { a: Int } input A @oneOf { a: A } input B { a: A! }`,
  `type Query { a: Int } input A @oneOf { a: A b: Int } input B { a: A! }`,
  `type Query { a: Int } input A { b: B! } input B @oneOf { a: A c: C } input C { b: B! }`,
  `type Query { a: Int } input A { a: [A!]! b: A }`,
  // createInputObjectDefaultValueCircularRefsValidator
  `type Query { a: Int } input A { a: A = {} }`,
  `type Query { a: Int } input A { b: B = {} } input B { a: A = {} }`,
  `type Query { a: Int } input A { b: B = {} } input B { c: C = {} } input C { a: A = {} }`,
  `type Query { a: Int } input A { b: B = { c: {} } } input B { c: C } input C { a: A = {} }`,
  `type Query { a: Int } input A { a: [A] = [{}] }`,
  `type Query { a: Int } input A { b: B = { a: { b: {} } } } input B { a: A }`,
  `type Query { a: Int } input A { b: B = { a: {} } } input B { a: A }`,
  `type Query { a: Int } input A { x: Int = 1 b: B = {} } input B { x: Int }`,
  `type Query { a: Int } input A { b: B = {} c: C = {} } input B { a: A = {} } input C { b: B = {} }`,
  // Executable definitions in SDL documents.
  `query { a }`,
  `type Query { a: Int } query Q { a } query Q { b }`,
];
handWritten.forEach((sdl, i) => add(`hand-written ${i}`, sdl));

// What graphql-js builds: the observable state of the schema.
function str(v) {
  return v === undefined ? null : v;
}
function defaultStr(input) {
  if (input.default === undefined) return null;
  if (input.default.literal) return print(input.default.literal);
  return JSON.stringify(input.default.value);
}
function argShape(a, intro) {
  return [a.name, String(a.type), intro ? null : str(a.description), defaultStr(a), str(a.deprecationReason)];
}
function shape(schema) {
  const types = Object.values(schema.getTypeMap()).map((t) => {
    const intro = isIntrospectionType(t);
    const o = { name: t.name, description: intro ? null : str(t.description) };
    if (isScalarType(t)) {
      o.kind = "scalar";
      o.specifiedByURL = str(t.specifiedByURL);
    } else if (isObjectType(t) || isInterfaceType(t)) {
      o.kind = isObjectType(t) ? "object" : "interface";
      o.interfaces = t.getInterfaces().map(String);
      o.fields = Object.values(t.getFields()).map((f) => [
        f.name,
        String(f.type),
        intro ? null : str(f.description),
        str(f.deprecationReason),
        f.args.map((a) => argShape(a, intro)),
      ]);
    } else if (isUnionType(t)) {
      o.kind = "union";
      o.types = t.getTypes().map(String);
    } else if (isEnumType(t)) {
      o.kind = "enum";
      try {
        o.values = t.getValues().map((v) => [v.name, intro ? null : str(v.description), str(v.deprecationReason)]);
      } catch (e) {
        o.valuesError = message(e);
      }
    } else if (isInputObjectType(t)) {
      o.kind = "input";
      o.isOneOf = t.isOneOf;
      o.fields = Object.values(t.getFields()).map((f) => argShape(f, false));
    }
    return o;
  });
  return {
    description: str(schema.description),
    query: str(schema.getQueryType()?.name),
    mutation: str(schema.getMutationType()?.name),
    subscription: str(schema.getSubscriptionType()?.name),
    types,
    directives: schema
      .getDirectives()
      .map((d) => [d.name, str(d.description), d.locations, d.isRepeatable, str(d.deprecationReason), d.args.map((a) => argShape(a, false))]),
  };
}

for (const c of cases) {
  let schema;
  try {
    schema = buildSchema(c.sdl);
  } catch (e) {
    c.buildError = message(e);
    continue;
  }
  c.shape = shape(schema);
  try {
    c.validateErrors = validateSchema(schema).map(message);
  } catch (e) {
    c.validateThrows = message(e);
  }
  try {
    assertValidSchema(schema);
    c.assertError = null;
  } catch (e) {
    c.assertError = message(e);
  }
}

fs.writeFileSync(path.join(here, "schema-corpus.json"), JSON.stringify({ cases }));
const built = cases.filter((c) => c.shape);
console.log(
  `${cases.length} documents (${corpusCount} composition outputs, ${testsCount} test documents, ` +
    `${cases.length - corpusCount - testsCount} hand-written): ${cases.length - built.length} fail to build, ` +
    `${built.filter((c) => c.assertError).length} build but are invalid, ` +
    `${built.filter((c) => !c.assertError).length} are valid`,
);
