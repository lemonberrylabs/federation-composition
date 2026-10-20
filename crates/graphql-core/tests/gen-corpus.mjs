// Builds crates/graphql-core/tests/corpus.json: inputs and graphql-js 17.0.2's
// results for them, the expected values of the Rust differential tests.
//
// Run from the repository root, after `pnpm conformance` (graphql-js and upstream's
// tests come from its upstream checkout):  node crates/graphql-core/tests/gen-corpus.mjs
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
const {
  parse,
  parseValue,
  parseConstValue,
  parseType,
  parseSchemaCoordinate,
  print,
  stripIgnoredCharacters,
  visit,
  visitInParallel,
  BREAK,
  Kind,
} = g;
if (g.version !== "17.0.2") throw new Error(`expected graphql 17.0.2, got ${g.version}`);

// Error messages may contain lone surrogates (graphql-js slices strings by
// UTF-16 code unit); converted to UTF-8 they become U+FFFD.
const msg = (e) => {
  if (!(e instanceof g.GraphQLError)) throw e;
  return e.message.toWellFormed();
};

function walk(dir, f) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    e.isDirectory() ? walk(p, f) : f(p);
  }
}

// ---------------------------------------------------------------- documents

const docInputs = [];
const seen = new Set();
// `derived` describes how the Rust test rebuilds `text` from a rich document,
// to keep the corpus small: { of, bytes } is a prefix of `bytes` UTF-8 bytes,
// { of, at, op } deletes or doubles the ASCII character at byte `at`.
function addDoc(src, text, derived) {
  if (seen.has(text)) return;
  seen.add(text);
  docInputs.push({ src, text, derived });
}

walk(path.join(upstream, "__tests__/fixtures"), (p) => {
  if (p.endsWith(".graphql")) addDoc(path.relative(upstream, p), fs.readFileSync(p, "utf8"));
});

const templateRe = /graphql`([^`]*)`/g;
const fieldsRe = /fields:\s*"((?:[^"\\]|\\.)*)"/g;
const fieldSets = new Set();
function collectFields(text) {
  for (const m of text.matchAll(fieldsRe)) {
    let f = m[1];
    try {
      f = JSON.parse(`"${f}"`);
    } catch {}
    fieldSets.add(f);
  }
}
walk(path.join(upstream, "__tests__"), (p) => {
  if (!p.endsWith(".ts")) return;
  const source = fs.readFileSync(p, "utf8");
  collectFields(source);
  for (const m of source.matchAll(templateRe)) {
    const raw = m[1].replace(/\$\{[^}]*\}/g, "v2.3");
    let cooked = raw;
    try {
      cooked = new Function("return `" + raw + "`")();
    } catch {}
    addDoc(path.relative(upstream, p), cooked);
    if (cooked !== raw) addDoc(path.relative(upstream, p) + " (raw)", raw);
    collectFields(cooked);
  }
});
for (const f of fieldSets) {
  const t = f.trim();
  if (t.startsWith("{")) {
    addDoc("fields", `query ${f}`);
  } else {
    addDoc("fields", `query { ${f} }`);
    addDoc("fields", `query {${f}}`);
  }
}

// Documents covering the whole grammar.
const richSdl = `
# A comment
"""
Schema description
  indented line
"""
schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key", "@shareable", { name: "@tag", as: "@mytag" }]) {
  query: Query
  mutation: Mutation
  subscription: Subscription
}

extend schema @link(url: "https://specs.apollo.dev/link/v1.0")

extend schema {
  subscription: Subscription
}

"Scalar description" scalar DateTime @specifiedBy(url: "https://example.com/datetime")
extend scalar DateTime @tag(name: "time")

"""
Multi-line description with "quotes" and \\"""escaped triple quotes\\""" and a trailing quote"
"""
type Query implements Node & Entity @key(fields: "id") @key(fields: "sku package { id }", resolvable: false) {
  "field description"
  node(id: ID!): Node
  nodes(ids: [ID!]! = ["1", "2"], first: Int = 10, after: String = null, filter: Filter = { name: "x", tags: [A, B], nested: { deep: [1.5, -2e10, 0] } }): [Node!]! @deprecated(reason: "use other")
  withLongArgs(argumentNumberOne: String = "aaaaaaaaaaaaaaaa", argumentNumberTwo: String = "bbbbbbbbbbbbbbbb", three: Int): Int
  withDescribedArgs(
    "described arg"
    a: Int
    b: Int
  ): Int
  list: [[Int!]]!
  empty: String @external @shareable @override(from: "other", label: "percent(50)")
}

type Mutation { doIt(input: DoItInput!): Boolean }
type Subscription { ticks: Int }

interface Node { id: ID! }
interface Entity implements Node @key(fields: "id") { id: ID! name: String }
extend interface Entity implements Other @tag(name: "x") { extra: Int }
extend interface Entity2 implements A & B
extend type Query @shareable
extend type Query implements Node & Other
extend type Query { more: Int }

union SearchResult @tag(name: "u") = | Query | Mutation
union U2 = A
extend union SearchResult = Subscription
extend union SearchResult @tag(name: "v")

enum Color @tag(name: "e") {
  "red description"
  RED @deprecated
  GREEN
  BLUE @tag(name: "blue")
}
extend enum Color { PURPLE }
extend enum Color @tag(name: "f")

input Filter @oneOf {
  "name description"
  name: String = "default" @deprecated(reason: "no")
  tags: [Color!] = [RED, GREEN]
  nested: Filter
}
extend input Filter { more: Int = 1 }
extend input Filter @tag(name: "g")

"""Directive description"""
directive @key(fields: FieldSet!, resolvable: Boolean = true) repeatable on OBJECT | INTERFACE
directive @tag(name: String!) repeatable on
  | FIELD_DEFINITION
  | OBJECT
  | INTERFACE
  | UNION
  | ARGUMENT_DEFINITION
  | SCALAR
  | ENUM
  | ENUM_VALUE
  | INPUT_OBJECT
  | INPUT_FIELD_DEFINITION
  | SCHEMA
directive @ext(
  """
  Argument description
  """
  arg: [String!]! = ["a"] @deprecated
) @meta on QUERY | MUTATION | SUBSCRIPTION | FIELD | FRAGMENT_DEFINITION | FRAGMENT_SPREAD | INLINE_FRAGMENT | VARIABLE_DEFINITION | FRAGMENT_VARIABLE_DEFINITION | DIRECTIVE_DEFINITION
extend directive @ext @meta(x: 1)
scalar _FieldSet
`;

const richQuery = `
query Named($id: ID! = "1" @var, $list: [Int!] = [1, 2, 3], $obj: In = { a: { b: [null, true, false, ENUM, 1.5e3, -0] } }) @op(x: $id) {
  alias: node(id: $id) {
    id
    ... on User @include(if: true) {
      name(format: { a: 1, b: "two", c: [$id] })
      friends(first: 10) @skip(if: false) { edges { node { id } } }
    }
    ...UserFields @defer(label: "x")
    ... @include(if: $cond) { __typename }
    ... { inner }
  }
  veryLongFieldName(argumentNumberOne: "aaaaaaaaaaaaaaaaaaaa", argumentNumberTwo: "bbbbbbbbbbbbbbbbbbbbbb", three: 3) { x }
  longList(values: ["aaaaaaaaaaaaaaaaaaaa", "bbbbbbbbbbbbbbbbbbbbbbbb", "cccccccccccccccccccccccc", "dddd"])
  longObject(value: { first: "aaaaaaaaaaaaaaaaaaaa", second: "bbbbbbbbbbbbbbbbbbbbbbbb", third: "cccccccccccc" })
  emptyThings(list: [], obj: {})
  strings(a: "with \\"escapes\\" \\\\ \\/ \\b \\f \\n \\r \\t \\u00e9 \\u{1F600} \\uD83D\\uDE00 é 😀", b: """block "string" with \\""" inside""")
}

mutation { doIt(input: { a: 1 }) }
subscription Sub { ticks }
"Operation description" query Described { a }
query ($v: Int, "described variable" $w: Int) { a }
fragment UserFields on User @frag { id ...Other }
"Fragment description" fragment Described on User { id }
{ shorthand }
`;

const richDocs = [richSdl, richQuery];
for (const d of richDocs) addDoc("rich", d);

// Every lexer and parser error message.
const invalid = [
  "",
  "   ",
  "# only a comment",
  "{",
  "{ a",
  "{ a(",
  "{ a(b",
  "{ a(b:",
  "{ a(b: 1",
  "{ a(b: 1)",
  "{}",
  "{ a(b: ) }",
  "{ a(b: }) }",
  "{ a .. }",
  "{ a(x: .5) }",
  "{ a(x: 'b') }",
  "{ a ~ }",
  "{ a é }",
  "{ a 😀 }",
  "{ a \u0007 }",
  "{ a ? }",
  "{ a . }",
  "{ a(x: 01) }",
  "{ a(x: -01) }",
  "{ a(x: 1.) }",
  "{ a(x: 1.a) }",
  "{ a(x: 1e) }",
  "{ a(x: 1ex) }",
  "{ a(x: 1e+) }",
  "{ a(x: 1a) }",
  "{ a(x: -a) }",
  "{ a(x: -) }",
  "{ a(x: 1.0.) }",
  "{ a(x: 0x10) }",
  "{ a(x: 1_000) }",
  "{ a(x: 1.5e3.1) }",
  "{ a(x: 1",
  "{ a(x: 1.",
  "{ a(x: 1e",
  "{ a(x: -",
  "{ a(x: 0.é) }",
  "{ a(x: 1é) }",
  "{ a(x: \"abc",
  "{ a(x: \"abc\n\") }",
  "{ a(x: \"abc\r\") }",
  "{ a(x: \"\"\"abc",
  "{ a(x: \"\"\"abc\\\"\"\"",
  "{ a(x: \"\\x\") }",
  "{ a(x: \"\\",
  "{ a(x: \"\\😀\") }",
  "{ a(x: \"\\é\") }",
  "{ a(x: \"\\u{}\") }",
  "{ a(x: \"\\u{110000}\") }",
  "{ a(x: \"\\u{D800}\") }",
  "{ a(x: \"\\u{DFFF}\") }",
  "{ a(x: \"\\u{12345678}\") }",
  "{ a(x: \"\\u{FFFFFFFF}\") }",
  "{ a(x: \"\\u{10000000}\") }",
  "{ a(x: \"\\u{00000041}\") }",
  "{ a(x: \"\\u{0000000041}\") }",
  "{ a(x: \"\\u{1F600\") }",
  "{ a(x: \"\\u{1F600",
  "{ a(x: \"\\u{xyz}\") }",
  "{ a(x: \"\\u{😀}\") }",
  "{ a(x: \"\\u{é}\") }",
  "{ a(x: \"\\uD800\") }",
  "{ a(x: \"\\uD800\\u0041\") }",
  "{ a(x: \"\\uD800\\uZZZZ\") }",
  "{ a(x: \"\\uD800\\uDBFF\") }",
  "{ a(x: \"\\uDC00\") }",
  "{ a(x: \"\\u12\") }",
  "{ a(x: \"\\u12",
  "{ a(x: \"\\u12é3\") }",
  "{ a(x: \"\\u1😀\") }",
  "{ a(x: \"\\uZZZZ\") }",
  "{ a(x: \"\\u{41}\\u0041\\u00e9\\u{1F600}\\uD83D\\uDE00\\u{0}\") }",
  "\"desc\" { a }",
  "\"\"\"desc\"\"\" { a }",
  "\"desc\" extend type A @x",
  "\"desc\" ~",
  "\"desc\"",
  "\"desc\" foo",
  "notakeyword",
  "extend",
  "extend foo",
  "extend \"x\"",
  "extend schema",
  "extend schema {}",
  "extend schema { query }",
  "extend scalar A",
  "extend type A",
  "extend interface A",
  "extend union A",
  "extend union A =",
  "extend enum A",
  "extend input A",
  "extend directive @a",
  "extend directive a",
  "schema { foo: Bar }",
  "schema { query Bar }",
  "schema {}",
  "schema @a",
  "scalar",
  "scalar \"A\"",
  "type",
  "type A {",
  "type A {}",
  "type A { f }",
  "type A { f: }",
  "type A { f(): Int }",
  "type A { f(a): Int }",
  "type A { f(a: Int = $x): Int }",
  "type A { f(a: Int = $): Int }",
  "type A { f(a: Int = [$x]): Int }",
  "type A { f(a: Int = { x: $y }): Int }",
  "type A { f: [Int }",
  "type A { f: Int! ! }",
  "type A implements { f: Int }",
  "type A implements & { f: Int }",
  "type A implements B & { f: Int }",
  "type A @ { f: Int }",
  "interface",
  "union U = ",
  "union U = | ",
  "union U = A |",
  "enum E { true }",
  "enum E { false }",
  "enum E { null }",
  "enum E { \"desc\" null }",
  "enum E { \"x\" \"true\" }",
  "enum E {}",
  "enum E { A B C",
  "input I { a }",
  "input I { a: Int = }",
  "directive",
  "directive a on FIELD",
  "directive @a FIELD",
  "directive @a on",
  "directive @a on FOO",
  "directive @a on FIELD | FOO",
  "directive @a on | | FIELD",
  "directive @a(b: Int) repeatable FIELD",
  "directive @a repeatable",
  "query",
  "query {",
  "query Q(",
  "query Q($",
  "query Q($a",
  "query Q($a:",
  "query Q($a: Int =",
  "query Q($a: Int = $b) { a }",
  "query Q($a: Int @dir(x: $b)) { a }",
  "query Q(a: Int) { a }",
  "query Q() { a }",
  "query Q @ { a }",
  "query Q { a: }",
  "query Q { a: b: c }",
  "query Q { ... }",
  "query Q { ...on }",
  "query Q { ... on }",
  "query Q { ... on A }",
  "query Q { ...A( }",
  "query Q { ...A(x: 1) }",
  "query Q { a @ }",
  "query Q { a @b( }",
  "mutation",
  "subscription S",
  "fragment",
  "fragment on on A { a }",
  "fragment F A { a }",
  "fragment F on { a }",
  "fragment F on A",
  "fragment F($a: Int) on A { a }",
  "fragment \"on\" on A { a }",
  "{ a } 1",
  "{ a } \"str\"",
  "{ a } \"\"\"block\"\"\"",
  "{ a } 1.5",
  "{ a } !",
  "{ a } $",
  "{ a } &",
  "{ a } (",
  "{ a } )",
  "{ a } ...",
  "{ a } :",
  "{ a } =",
  "{ a } @",
  "{ a } [",
  "{ a } ]",
  "{ a } }",
  "{ a } |",
  "{ a }}",
  "{ a(b: [1, 2) }",
  "{ a(b: {c 1}) }",
  "{ a(b: {c: 1) }",
  "{ a(b: $) }",
  "{ a(b: $1) }",
  "{ a(b: \"x\" \"y\") }",
  "\ufeff{ a }",
  "{ a\ufeff b }",
  "{ a\r\nb\rc\n}",
  "{ a, b,, c }",
  "{ a } # trailing comment",
  "{ a # comment\n b }",
];
for (const t of invalid) addDoc("invalid", t);

// Every prefix of the rich documents exercises unexpected EOF in each production.
const utf8Len = (s) => Buffer.byteLength(s, "utf8");
richDocs.forEach((d, of) => {
  for (let i = 0; i < d.length; i++) {
    // Do not cut surrogate pairs: Rust strings cannot hold lone surrogates.
    const c = d.charCodeAt(i);
    if (c >= 0xdc00 && c <= 0xdfff) continue;
    const text = d.slice(0, i);
    addDoc("prefix", text, { of, bytes: utf8Len(text) });
  }
});
// Deleting each punctuator, and doubling it.
richDocs.forEach((d, of) => {
  for (let i = 0; i < d.length; i++) {
    if ("{}()[]:=@$!|&\".".includes(d[i])) {
      const at = utf8Len(d.slice(0, i));
      addDoc("mutation", d.slice(0, i) + d.slice(i + 1), { of, at, op: "delete" });
      addDoc("mutation", d.slice(0, i) + d[i] + d.slice(i), { of, at, op: "double" });
    }
  }
});

// Block strings and strings exercising printBlockString / printString.
const stringValues = [
  "",
  " ",
  "  leading spaces",
  "\tleading tab",
  "trailing spaces  ",
  'trailing quote"',
  "trailing backslash\\",
  'contains """ triple quotes',
  'ends with """',
  '"""',
  "multi\nline",
  "multi\n  indented\n    more",
  "\n\nleading blank lines",
  "trailing blank lines\n\n",
  "first\n second starts with space",
  "a".repeat(70),
  "a".repeat(71),
  "é".repeat(71),
  "😀".repeat(36),
  "😀".repeat(35),
  "line one\r\nline two",
  "cr\ronly",
  "\u0001control",
  "tab\tinside",
  "\u007f\u0080\u009f\u00a0",
  "back\\slash",
];
for (const s of stringValues) {
  addDoc("strings", `"""${s.replaceAll('"""', '\\"""')}"""\nscalar A`);
  addDoc("strings", `${JSON.stringify(s)}\nscalar A`);
  addDoc("strings", `{ a(x: ${JSON.stringify(s)}, y: """${s.replaceAll('"""', '\\"""')}""") }`);
}

function runDoc({ src, text, derived }) {
  const out = derived ? { src, derived } : { src, text };
  try {
    out.stripped = stripIgnoredCharacters(text);
  } catch (e) {
    out.strippedError = msg(e);
  }
  try {
    const ast = parse(text, { noLocation: true });
    out.json = JSON.stringify(ast);
    out.printed = print(ast);
  } catch (e) {
    out.error = msg(e);
  }
  try {
    const ast = parse(text, { noLocation: true, experimentalFragmentArguments: true });
    const json = JSON.stringify(ast);
    if (json !== out.json) out.fragmentArgumentsJson = json;
    const printed = print(ast);
    if (printed !== out.printed) out.fragmentArgumentsPrinted = printed;
  } catch (e) {
    if (msg(e) !== out.error) out.fragmentArgumentsError = msg(e);
  }
  return out;
}
const documents = docInputs.map(runDoc);

// maxTokens.
const maxTokens = [];
for (const text of [richQuery, "{ a b c }", "{ a }", "query Q($a: Int) { a(b: $a) }"]) {
  for (const limit of [0, 1, 2, 3, 4, 5, 10, 50, 1000]) {
    const out = { text, maxTokens: limit };
    try {
      out.json = JSON.stringify(parse(text, { noLocation: true, maxTokens: limit }));
    } catch (e) {
      out.error = msg(e);
    }
    maxTokens.push(out);
  }
}

// ---------------------------------------------------------- values and types

const valueTexts = [
  "1", "-1", "0", "-0", "1.5", "1e10", "1E-10", "-1.5e+3", "123456789012345678901234567890",
  '"str"', '""', '"\\u00e9"', '"""block"""', '"""  block\n  indented"""', "true", "false", "null",
  "ENUM", "on", "fragment", "$var", "$", "$1", "[]", "[1, 2, [3]]", "[1 2 3]", "{}", "{ a: 1 }",
  "{ a: { b: [$c] } }", "{ a 1 }", "{ a: }", "[", "{", "", "  ", "1 2", "@", "...", "01", ".5",
  "'x'", "1.", "\"unterminated", "{ a: 1, a: 2 }", "[null, NULL, Null]", "# comment\n1",
];
const values = [];
for (const text of valueTexts) {
  for (const mode of ["value", "const"]) {
    const out = { text, mode };
    try {
      const v = (mode === "value" ? parseValue : parseConstValue)(text, { noLocation: true });
      out.json = JSON.stringify(v);
      out.printed = print(v);
    } catch (e) {
      out.error = msg(e);
    }
    values.push(out);
  }
}

const typeTexts = [
  "Int", "Int!", "[Int]", "[Int!]!", "[[Int]!]", "Int!!", "[Int", "[]", "", "!", "1", "[Int]]",
  "Int Int", "  Int  ", "# c\nInt",
];
const types = typeTexts.map((text) => {
  const out = { text };
  try {
    const t = parseType(text, { noLocation: true });
    out.json = JSON.stringify(t);
    out.printed = print(t);
  } catch (e) {
    out.error = msg(e);
  }
  return out;
});

const coordinateTexts = [
  "Type", "Type.field", "Type.field(arg:)", "@dir", "@dir(arg:)", "Type.", "Type.field(", "Type.field(arg",
  "Type.field(arg:", "@", "@dir.", "@dir(", " Type", "Type ", "Type.field(arg: )", "1", "Type#", "Type(arg:)",
  "@dir(arg:).x", "", "é",
];
const coordinates = coordinateTexts.map((text) => {
  const out = { text };
  try {
    const c = parseSchemaCoordinate(text);
    out.json = JSON.stringify(c, (k, v) => (k === "loc" ? undefined : v));
    out.printed = print(c);
  } catch (e) {
    out.error = msg(e);
  }
  return out;
});

// ---------------------------------------------- hand-built ASTs for print()

const N = (value) => ({ kind: "Name", value });
const printAsts = [
  { kind: "Document", definitions: [] },
  {
    kind: "Document",
    definitions: [
      {
        kind: "ObjectTypeDefinition",
        name: N("A"),
        interfaces: [],
        directives: null,
        fields: [{ kind: "FieldDefinition", name: N(""), type: { kind: "NamedType", name: N("Int") }, arguments: [] }],
      },
      { kind: "OperationDefinition", operation: "query", selectionSet: { kind: "SelectionSet", selections: [] } },
      { kind: "OperationDefinition", operation: "mutation", name: N(""), selectionSet: { kind: "SelectionSet", selections: [] } },
      { kind: "ScalarTypeDefinition", name: N("S"), description: { kind: "StringValue", value: "no block flag" } },
      { kind: "ScalarTypeDefinition", name: N("T"), description: { kind: "StringValue", value: "x\u0000y", block: true } },
      { kind: "ScalarTypeDefinition", name: N("U"), description: { kind: "StringValue", value: "", block: true } },
      { kind: "ScalarTypeDefinition", name: N("V"), description: { kind: "StringValue", value: "a\r\nb", block: true } },
      { kind: "DirectiveDefinition", name: N("d"), repeatable: true, locations: [N("FIELD"), N("")] },
      { kind: "SchemaExtension", directives: [{ kind: "Directive", name: N("x"), arguments: [] }] },
      { kind: "EnumTypeDefinition", name: N("E"), values: [] },
      { kind: "UnionTypeDefinition", name: N("U"), types: [] },
    ],
  },
  { kind: "ObjectValue", fields: [] },
  { kind: "ListValue", values: [] },
  { kind: "Field", name: N("f"), selectionSet: { kind: "SelectionSet", selections: [] } },
  { kind: "Field", alias: N(""), name: N("f") },
  { kind: "NonNullType", type: { kind: "ListType", type: { kind: "NamedType", name: N("A") } } },
  { kind: "FragmentDefinition", name: N("F"), typeCondition: { kind: "NamedType", name: N("A") }, variableDefinitions: [{ kind: "VariableDefinition", variable: { kind: "Variable", name: N("v") }, type: { kind: "NamedType", name: N("Int") } }], selectionSet: { kind: "SelectionSet", selections: [{ kind: "Field", name: N("a") }] } },
  { kind: "FragmentSpread", name: N("F"), arguments: [{ kind: "FragmentArgument", name: N("a"), value: { kind: "IntValue", value: "1" } }] },
  { kind: "MemberCoordinate", name: N("A"), memberName: N("") },
];
const printAstCases = printAsts.map((ast) => ({ json: JSON.stringify(ast), printed: print(ast) }));

// ------------------------------------------------------------------ visitor

// FNV-1a over the UTF-8 bytes of a string.
const encoder = new TextEncoder();
function fnv(s) {
  let h = 0x811c9dc5;
  for (const b of encoder.encode(s)) {
    h ^= b;
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16);
}
const desc = (x) => (Array.isArray(x) ? `[${x.length}]` : x === undefined ? "-" : x.kind);
function traceLine(phase, node, key, parent, path, ancestors) {
  return `${phase}|${node.kind}|${key === undefined ? "-" : key}|${desc(parent)}|${path.join(".")}|${ancestors.map(desc).join(",")}\n`;
}
function trace(ast) {
  let out = "";
  visit(ast, {
    enter(node, key, parent, path, ancestors) {
      out += traceLine("enter", node, key, parent, path, ancestors);
    },
    leave(node, key, parent, path, ancestors) {
      out += traceLine("leave", node, key, parent, path, ancestors);
    },
  });
  return out;
}

const traces = [];
documents.forEach((d, doc) => {
  if (d.json === undefined) return;
  const t = trace(JSON.parse(d.json));
  traces.push(d.src === "rich" ? { doc, trace: t } : { doc, hash: fnv(t) });
});

// Editing scenarios, mirrored in tests/differential.rs. Each returns
// { result, log } for a document.
const nameOf = (node) => node.name?.value ?? "";
const scenarios = {
  deleteDirectives(ast) {
    const log = [];
    const result = visit(ast, {
      Directive(node) {
        log.push(`Directive ${node.name.value}`);
        if (node.name.value.length % 2 === 0) return null;
      },
    });
    return { result, log };
  },
  replaceOnEnter(ast) {
    const log = [];
    const result = visit(ast, {
      FieldDefinition(node) {
        return { ...node, name: N(node.name.value + "_x") };
      },
      Name(node) {
        log.push(node.value);
      },
    });
    return { result, log };
  },
  leaveReplace(ast) {
    const log = [];
    const result = visit(ast, {
      Directive(node) {
        if (node.name.value.length % 2 === 0) return null;
      },
      ObjectTypeDefinition: {
        leave(node) {
          log.push(`${node.name.value}:${(node.directives ?? []).length}`);
          return { ...node, fields: node.fields ? [...node.fields].reverse() : node.fields };
        },
      },
    });
    return { result, log };
  },
  skipOnEnter(ast) {
    const log = [];
    const result = visit(ast, {
      enter(node) {
        log.push(`enter ${node.kind}`);
        if (nameOf(node).length % 2 === 1 && node.kind !== Kind.NAME) return false;
      },
      leave(node) {
        log.push(`leave ${node.kind}`);
      },
    });
    return { result, log };
  },
  leaveSkipDropsEdits(ast) {
    const log = [];
    const result = visit(ast, {
      Directive() {
        return null;
      },
      InputValueDefinition: {
        leave(node) {
          log.push(`${node.name.value}:${(node.directives ?? []).length}`);
          return false;
        },
      },
      FieldDefinition: {
        leave(node) {
          log.push(`field ${node.name.value}`);
        },
      },
    });
    return { result, log };
  },
  deleteOptional(ast) {
    const log = [];
    const result = visit(ast, {
      StringValue(node, key) {
        if (key === "description") return null;
      },
      SelectionSet(node, key, parent) {
        if (parent && !Array.isArray(parent) && parent.kind === Kind.FIELD) return null;
      },
      NamedType(node, key, parent) {
        if (key === "typeCondition" && parent.kind === Kind.INLINE_FRAGMENT && node.name.value.length % 2 === 0) return null;
      },
      VariableDefinition: {
        enter(node) {
          if (node.defaultValue) return { ...node, defaultValue: null };
        },
      },
      leave(node) {
        log.push(node.kind);
      },
    });
    return { result, log };
  },
  replaceRoot(ast) {
    const log = [];
    const result = visit(ast, {
      Document(node) {
        return { ...node, definitions: node.definitions.slice(1) };
      },
      Name(node) {
        log.push(node.value);
      },
    });
    return { result, log };
  },
  replaceThenDeleteOnLeave(ast) {
    const log = [];
    const result = visit(ast, {
      EnumValueDefinition: {
        enter(node) {
          return { ...node, name: N(node.name.value.toLowerCase()) };
        },
        leave(node) {
          log.push(node.name.value);
          if (node.name.value.length % 2 === 0) return null;
        },
      },
      Argument: {
        enter(node) {
          return { ...node, value: { kind: "StringValue", value: "replaced", block: false } };
        },
      },
      Name(node) {
        log.push(`name ${node.value}`);
      },
    });
    return { result, log };
  },
  parallel(ast) {
    const log = [];
    const mk = (id, f) => ({
      enter(node, key, parent, path) {
        log.push(`${id} enter ${node.kind} ${path.join(".")}`);
        return f?.enter?.(node, key, parent, path);
      },
      leave(node, key, parent, path) {
        log.push(`${id} leave ${node.kind} ${path.join(".")}`);
        return f?.leave?.(node, key, parent, path);
      },
    });
    const result = visit(
      ast,
      visitInParallel([
        mk("A", { enter: (node) => (node.kind === Kind.OBJECT_TYPE_DEFINITION || node.kind === Kind.FIELD ? false : undefined) }),
        mk("B", { enter: (node) => (node.kind === Kind.DIRECTIVE && node.name.value.length % 2 === 0 ? null : undefined) }),
        mk("C", {
          enter: (node) => (node.kind === Kind.ENUM_VALUE_DEFINITION ? { ...node, name: N("REPLACED") } : undefined),
          leave: (node) => (node.kind === Kind.INPUT_VALUE_DEFINITION && node.name.value.length % 2 === 1 ? null : undefined),
        }),
        mk("D", { leave: (node) => (node.kind === Kind.FIELD_DEFINITION && node.name.value.startsWith("w") ? BREAK : undefined) }),
      ]),
    );
    return { result, log };
  },
  parallelReadOnly(ast) {
    const log = [];
    const mk = (id, f) => ({
      enter(node, key, parent, path) {
        log.push(`${id} enter ${node.kind} ${path.join(".")}`);
        return f?.enter?.(node);
      },
      leave(node, key, parent, path) {
        log.push(`${id} leave ${node.kind} ${path.join(".")}`);
        return f?.leave?.(node);
      },
    });
    visit(
      ast,
      visitInParallel([
        mk("A", { enter: (node) => (node.kind === Kind.OBJECT_TYPE_DEFINITION || node.kind === Kind.SELECTION_SET ? false : undefined) }),
        mk("B", { enter: (node) => (node.kind === Kind.DIRECTIVE && node.name.value === "tag" ? BREAK : undefined) }),
        mk("C", { leave: (node) => (node.kind === Kind.FIELD_DEFINITION ? BREAK : undefined) }),
        mk("D"),
      ]),
    );
    return { result: ast, log };
  },
};
// BREAK makes graphql-js return the last edit at the depth where it stopped:
// here an edited list.
scenarios.breakAfterListEdit = (ast) => {
  const log = [];
  const result = visit(ast, {
    Directive(node) {
      return { ...node, name: N(node.name.value + "2") };
    },
    SelectionSet(node, key, parent) {
      if (parent && !Array.isArray(parent) && parent.kind === Kind.FIELD && (parent.directives ?? []).length > 0) {
        log.push(parent.name.value);
        return BREAK;
      }
    },
  });
  return { result, log };
};
// ... and here a deletion (null).
scenarios.breakAfterDelete = (ast) => {
  const log = [];
  let deletedIn = null;
  const result = visit(ast, {
    Directive(node, key, parent, path) {
      if (deletedIn === parent) {
        log.push(path.join("."));
        return BREAK;
      }
      if (Array.isArray(parent) && parent.length >= 2) {
        deletedIn = parent;
        return null;
      }
    },
  });
  return { result, log };
};
for (let n of [1, 2, 3, 5, 8, 13, 17, 30, 50, 77, 120, 200, 400]) {
  scenarios[`break${n}`] = (ast) => {
    const log = [];
    let count = 0;
    const result = visit(ast, {
      enter(node) {
        count++;
        if (count === n) return BREAK;
        if (node.kind === Kind.DIRECTIVE && count % 3 === 0) return null;
        if (node.kind === Kind.NAME && count % 4 === 0) return N(node.value + "!");
      },
      leave(node) {
        count++;
        if (count === n) return BREAK;
        if (node.kind === Kind.ARGUMENT && count % 2 === 0) return null;
      },
    });
    log.push(String(count));
    return { result, log };
  };
}

// The JSON form the Rust serializer writes: no null or undefined values, and
// no empty optional lists.
const requiredLists = {
  Document: "definitions",
  SelectionSet: "selections",
  ListValue: "values",
  ObjectValue: "fields",
  SchemaDefinition: "operationTypes",
  DirectiveDefinition: "locations",
};
function canonical(value) {
  return JSON.stringify(value, function (key, v) {
    if (v === null && key !== "") return undefined;
    if (Array.isArray(v) && v.length === 0 && typeof this?.kind === "string" && requiredLists[this.kind] !== key) return undefined;
    return v;
  });
}

// Read-only visits stopped by BREAK, with skips.
for (const n of [10, 100, 1000]) {
  scenarios[`readOnlyBreak${n}`] = (ast) => {
    const log = [];
    let count = 0;
    visit(ast, {
      enter(node, key) {
        count++;
        log.push(`enter ${node.kind} ${key}`);
        if (count === n) return BREAK;
        if (count % 7 === 0) return false;
      },
      leave(node) {
        count++;
        log.push(`leave ${node.kind}`);
        if (count === n) return BREAK;
      },
    });
    return { result: ast, log };
  };
}

const scenarioDocs = [];
documents.forEach((d, doc) => {
  if (d.json !== undefined && (d.src === "rich" || d.src === "strings")) scenarioDocs.push(doc);
});
documents.forEach((d, doc) => {
  if (d.json !== undefined && d.src.endsWith(".graphql") && scenarioDocs.length < 90) scenarioDocs.push(doc);
});
const edits = [];
for (const doc of scenarioDocs) {
  const d = documents[doc];
  for (const [name, f] of Object.entries(scenarios)) {
    const { result, log } = f(JSON.parse(d.json));
    const json = canonical(result);
    edits.push(d.src === "rich" ? { scenario: name, doc, result: json, log } : { scenario: name, doc, hash: fnv(json), logHash: fnv(log.join("\n")) });
  }
}

// ------------------------------------------------------ specified definitions

const specified = {
  directives: g.specifiedDirectives.map((d) => g.printDirective(d)),
  scalars: g.specifiedScalarTypes.map((t) => g.printType(t)),
  scalarNames: g.specifiedScalarTypes.map((t) => t.name),
  introspectionTypes: g.introspectionTypes.map((t) => t.name),
  version: g.versionInfo,
};

const literalTexts = [
  "1", "-1", "0", "-0", "2147483647", "2147483648", "-2147483648", "-2147483649", "99999999999999999999",
  "1.5", "1e3", "-1.5e-3", "0.1", '"str"', '""', "true", "false", "null", "ENUM", "[1]", "{ a: 1 }", "$v",
  '"""block"""', "123456789012345678901234567890.5",
];
const coercion = [];
for (const t of g.specifiedScalarTypes) {
  for (const text of literalTexts) {
    const node = parseValue(text);
    const out = { scalar: t.name, text };
    try {
      const v = t.coerceInputLiteral(node);
      out.result =
        typeof v === "number"
          ? Number.isFinite(v)
            ? Object.is(v, -0)
              ? "-0"
              : v
            : String(v)
          : v;
    } catch (e) {
      out.error = msg(e);
    }
    coercion.push(out);
  }
}

// ------------------------------------------------------------------- timing

const parseable = documents.filter((d) => d.json !== undefined && d.src !== "prefix" && d.src !== "mutation").map((d) => d.text);
for (let i = 0; i < 10; i++) for (const t of parseable) parse(t, { noLocation: true });
let t0 = performance.now();
const iterations = 30;
for (let i = 0; i < iterations; i++) for (const t of parseable) parse(t, { noLocation: true });
const jsParseMs = (performance.now() - t0) / iterations;

const corpus = { richDocs, documents, maxTokens, values, types, coordinates, printAsts: printAstCases, traces, edits, specified, coercion, timing: { jsParseMs, documents: parseable.length } };
fs.writeFileSync(path.join(here, "corpus.json"), JSON.stringify(corpus));
console.log(
  `${documents.length} documents (${documents.filter((d) => d.error !== undefined).length} errors), ` +
    `${values.length} values, ${types.length} types, ${coordinates.length} coordinates, ` +
    `${traces.length} traces, ${edits.length} edit cases, ${coercion.length} coercions; ` +
    `graphql-js parse: ${jsParseMs.toFixed(2)} ms per pass over ${parseable.length} documents`,
);
