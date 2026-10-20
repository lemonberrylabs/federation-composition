//! Port of `src/graphql/printer.ts`: the library's own printer, used for the supergraph SDL.
//! It differs from graphql-js `print` (e.g. type extensions print as empty strings), so it is
//! ported here and never replaced by `graphql_rs::print`.
//!
//! PERF: the TS builds a string per node and combines them with `join`, `wrap`, `block` and
//! `indent` (`str.replace("\n", "\n  ")` on the whole printed block, once per nesting level).
//! The port appends everything to one buffer instead (see [`Printer`]), with the same output:
//! - `indent` is applied as the text is written: every `"\n"` written inside `level` blocks is
//!   followed by `level` times two spaces, which is what the nested `replace` calls produce;
//! - `join` drops empty parts and `wrap` drops its `start`/`end` around an empty string: the
//!   separator or `start` is written, then removed again if the part wrote nothing;
//! - `printArgs` measures the single-line form without the indentation the buffer added.

use graphql_rs::{Argument, InputValueDefinition, NodeRef, OperationType};

/// `printString(string)`: `JSON.stringify(string)`.
/// NOTE(port): `JSON.stringify` escapes `\"`, `\\`, `\b`, `\f`, `\n`, `\r`, `\t`, the other C0
/// controls as lowercase `\u00xx`, and writes the rest verbatim. Rust strings cannot hold the
/// lone surrogates `JSON.stringify` would also escape.
/// The printer writes strings with [`Printer::print_string`]; this form is kept for the tests.
#[cfg(test)]
fn print_string(string: &str) -> String {
    let mut printer = Printer::default();
    printer.print_string(string);
    printer.out
}

/// `printBlockString(string)`. The printer uses [`Printer::print_block_string`].
#[cfg(test)]
fn print_block_string(string: &str) -> String {
    let mut printer = Printer::default();
    printer.print_block_string(string);
    printer.out
}

/// `print(node?)`: `""` for a missing node or a kind without a printer in the `nodes` table.
pub fn print<'a>(node: impl Into<NodeRef<'a>>) -> String {
    let mut printer = Printer::default();
    printer.print(node.into());
    printer.out
}

/// The output buffer and the number of `indent` calls the text being written is inside.
#[derive(Default)]
struct Printer {
    out: String,
    level: usize,
}

impl Printer {
    /// Appends `s`, indenting every line break by the current level.
    fn push(&mut self, s: &str) {
        if self.level == 0 || !s.contains('\n') {
            self.out.push_str(s);
            return;
        }
        for (index, line) in s.split('\n').enumerate() {
            if index > 0 {
                self.push_line_break();
            }
            self.out.push_str(line);
        }
    }

    fn push_line_break(&mut self) {
        self.out.push('\n');
        for _ in 0..self.level {
            self.out.push_str("  ");
        }
    }

    /// One part of `join(parts, separator)`: written after `separator` unless it is the first
    /// non-empty part, and removed with its separator when it writes nothing.
    fn part(&mut self, any: &mut bool, separator: &str, write: impl FnOnce(&mut Self)) {
        let mark = self.out.len();
        if *any {
            self.push(separator);
        }
        let start = self.out.len();
        write(self);
        if self.out.len() == start {
            self.out.truncate(mark);
        } else {
            *any = true;
        }
    }

    /// `join(nodes.map(print), separator)`: the non-empty printed nodes.
    fn join<'a, T: 'a>(&mut self, nodes: &'a [T], separator: &str)
    where
        &'a T: Into<NodeRef<'a>>,
    {
        let mut any = false;
        for node in nodes {
            self.part(&mut any, separator, |p| p.print(node.into()));
        }
    }

    /// `nodes.map(print).join(separator)`: a plain array join, empty strings included.
    fn join_all<'a, T: 'a>(&mut self, nodes: &'a [T], separator: &str)
    where
        &'a T: Into<NodeRef<'a>>,
    {
        for (index, node) in nodes.iter().enumerate() {
            if index > 0 {
                self.push(separator);
            }
            self.print(node.into());
        }
    }

    /// `wrap(start, maybeString, end = "")`: `""` when `maybeString` is empty.
    fn wrap(&mut self, start: &str, write: impl FnOnce(&mut Self), end: &str) {
        let mark = self.out.len();
        self.push(start);
        let content = self.out.len();
        write(self);
        if self.out.len() == content {
            self.out.truncate(mark);
        } else {
            self.push(end);
        }
    }

    /// `wrap("", print(node), "\n")` for a description.
    fn description<'a, N: 'a>(&mut self, node: Option<&'a N>)
    where
        &'a N: Into<NodeRef<'a>>,
    {
        if let Some(node) = node {
            self.wrap("", |p| p.print(node.into()), "\n");
        }
    }

    /// `block(nodes.map(print))`: `wrap("{\n", indent(join(array, "\n")), "\n}")`, where
    /// `indent(str)` is `wrap("  ", str.replace("\n", "\n  "))`.
    fn block<'a, T: 'a>(&mut self, nodes: &'a [T])
    where
        &'a T: Into<NodeRef<'a>>,
    {
        let mark = self.out.len();
        self.push("{\n");
        self.out.push_str("  ");
        let content = self.out.len();
        self.level += 1;
        self.join(nodes, "\n");
        self.level -= 1;
        if self.out.len() == content {
            self.out.truncate(mark);
        } else {
            self.push("\n}");
        }
    }

    /// The body of `printArgs` once the nodes are printed:
    /// `argsLine = "(" + args.join(", ") + ")"`, or when it is longer than 80,
    /// `"(\n" + indent(args.join("\n")) + "\n)"`.
    /// NOTE(port): `argsLine.length` counts UTF-16 code units (descriptions can be non-ASCII).
    fn print_printed_args<'a, T: 'a>(&mut self, nodes: &'a [T])
    where
        &'a T: Into<NodeRef<'a>>,
    {
        if nodes.is_empty() {
            return;
        }
        let mark = self.out.len();
        self.push("(");
        self.join_all(nodes, ", ");
        self.push(")");

        let args_line = &self.out[mark..];
        let length = if args_line.is_ascii() {
            args_line.len()
        } else {
            args_line.encode_utf16().count()
        };
        // The indentation this buffer added after each line break is not part of `argsLine`.
        let added_indentation = args_line.bytes().filter(|b| *b == b'\n').count() * 2 * self.level;
        if length - added_indentation > 80 {
            self.out.truncate(mark);
            self.push("(\n");
            self.out.push_str("  ");
            self.level += 1;
            self.join_all(nodes, "\n");
            self.level -= 1;
            self.push("\n)");
        }
    }

    /// `printArgs(nodes)` for `ArgumentNode[]`.
    fn print_args(&mut self, nodes: &[Argument]) {
        self.print_printed_args(nodes);
    }

    /// `printArgs(nodes)` for `InputValueDefinitionNode[]`.
    fn print_input_value_args(&mut self, nodes: &[InputValueDefinition]) {
        self.print_printed_args(nodes);
    }

    /// `printString(string)`: `JSON.stringify(string)`.
    fn print_string(&mut self, string: &str) {
        self.out.push('"');
        let mut rest = string;
        while let Some(index) = rest.bytes().position(|b| b < 0x20 || b == b'"' || b == b'\\') {
            self.out.push_str(&rest[..index]);
            match rest.as_bytes()[index] {
                b'"' => self.out.push_str("\\\""),
                b'\\' => self.out.push_str("\\\\"),
                b'\x08' => self.out.push_str("\\b"),
                b'\x0c' => self.out.push_str("\\f"),
                b'\n' => self.out.push_str("\\n"),
                b'\r' => self.out.push_str("\\r"),
                b'\t' => self.out.push_str("\\t"),
                control => {
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    self.out.push_str("\\u00");
                    self.out.push(HEX[(control >> 4) as usize] as char);
                    self.out.push(HEX[(control & 0xf) as usize] as char);
                }
            }
            rest = &rest[index + 1..];
        }
        self.out.push_str(rest);
        self.out.push('"');
    }

    /// `printBlockString(string)`: `'"""\n' + string.replace(/"""/g, '\\"""') + '\n"""'`.
    fn print_block_string(&mut self, string: &str) {
        self.push("\"\"\"\n");
        let mut rest = string;
        while let Some(index) = rest.find("\"\"\"") {
            self.push(&rest[..index]);
            self.out.push_str("\\\"\"\"");
            rest = &rest[index + 3..];
        }
        self.push(rest);
        self.push("\n\"\"\"");
    }

    /// The `nodes` table: `nodes[node.kind]?.(node) ?? ""`.
    fn print(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Document(node) => {
                if !node.definitions.is_empty() {
                    self.join_all(&node.definitions, "\n\n");
                }
            }
            // ...
            NodeRef::OperationDefinition(node) => {
                if node.operation == OperationType::Query
                    && node.name.is_none()
                    && node.variable_definitions.is_empty()
                    && node.directives.is_empty()
                {
                    self.print((&node.selection_set).into());
                    return;
                }

                let mut any = false;
                self.part(&mut any, " ", |p| p.push(node.operation.as_str()));
                self.part(&mut any, " ", |p| {
                    if let Some(name) = &node.name {
                        p.print(name.into());
                    }
                    p.wrap("(", |p| p.join(&node.variable_definitions, ", "), ")");
                });
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.print((&node.selection_set).into());
            }
            NodeRef::VariableDefinition(node) => {
                // NOTE(port): a plain `[...].join(" ")`, so empty parts still add separators.
                self.print((&node.variable).into());
                self.push(" : ");
                self.print((&node.ty).into());
                self.push(" ");
                if let Some(default_value) = &node.default_value {
                    self.wrap(" = ", |p| p.print(default_value.into()), "");
                }
                self.push(" ");
                self.join(&node.directives, " ");
            }
            NodeRef::Field(node) => {
                let mut any = false;
                self.part(&mut any, " ", |p| {
                    if let Some(alias) = &node.alias {
                        p.push(&alias.value);
                        p.push(": ");
                    }
                    p.push(&node.name.value);
                    p.print_args(&node.arguments);
                });
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                if let Some(selection_set) = &node.selection_set {
                    self.print(selection_set.into());
                }
            }
            NodeRef::StringValue(node) => {
                if node.block {
                    self.print_block_string(&node.value);
                } else {
                    self.print_string(&node.value);
                }
            }
            NodeRef::BooleanValue(node) => self.push(if node.value { "true" } else { "false" }),
            NodeRef::NullValue(_) => self.push("null"),
            NodeRef::IntValue(node) => self.push(&node.value),
            NodeRef::FloatValue(node) => self.push(&node.value),
            NodeRef::EnumValue(node) => self.push(&node.value),
            NodeRef::Name(node) => self.push(&node.value),
            NodeRef::Variable(node) => {
                self.push("$");
                self.push(&node.name.value);
            }
            NodeRef::ListValue(node) => {
                self.push("[");
                self.join_all(&node.values, ", ");
                self.push("]");
            }
            NodeRef::ObjectValue(node) => {
                self.push("{");
                self.join_all(&node.fields, ", ");
                self.push("}");
            }
            NodeRef::ObjectField(node) => {
                self.push(&node.name.value);
                self.push(": ");
                self.print((&node.value).into());
            }
            NodeRef::SelectionSet(node) => self.block(&node.selections),
            NodeRef::Argument(node) => {
                self.push(&node.name.value);
                self.push(": ");
                self.print((&node.value).into());
            }
            NodeRef::FragmentSpread(node) => {
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("..."));
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
            }
            NodeRef::InlineFragment(node) => {
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("..."));
                self.part(&mut any, " ", |p| {
                    if let Some(type_condition) = &node.type_condition {
                        p.push("on ");
                        p.push(&type_condition.name.value);
                    }
                });
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.part(&mut any, " ", |p| p.print((&node.selection_set).into()));
            }
            // NOTE(port): `join` without a separator, as in the TS.
            NodeRef::FragmentDefinition(node) => {
                let mut any = false;
                self.part(&mut any, "", |p| p.push("fragment"));
                self.part(&mut any, "", |p| p.print((&node.name).into()));
                self.part(&mut any, "", |p| p.push("on"));
                self.part(&mut any, "", |p| p.push(&node.type_condition.name.value));
                self.part(&mut any, "", |p| p.join(&node.directives, " "));
                self.part(&mut any, "", |p| p.print((&node.selection_set).into()));
            }
            NodeRef::Directive(node) => {
                self.push("@");
                self.push(&node.name.value);
                self.print_args(&node.arguments);
            }
            NodeRef::NamedType(node) => self.push(&node.name.value),
            NodeRef::ListType(node) => {
                self.push("[");
                self.print((&*node.ty).into());
                self.push("]");
            }
            NodeRef::NonNullType(node) => {
                self.print((&*node.ty).into());
                self.push("!");
            }
            // Type system definitions
            NodeRef::SchemaDefinition(node) => {
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("schema"));
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.part(&mut any, " ", |p| p.block(&node.operation_types));
            }
            NodeRef::OperationTypeDefinition(node) => {
                self.push(node.operation.as_str());
                self.push(": ");
                self.print((&node.ty).into());
            }
            NodeRef::ScalarTypeDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("scalar"));
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
            }
            NodeRef::ObjectTypeDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("type"));
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| {
                    p.wrap("implements ", |p| p.join(&node.interfaces, " & "), "")
                });
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.part(&mut any, " ", |p| p.block(&node.fields));
            }
            NodeRef::InterfaceTypeDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("interface"));
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| {
                    p.wrap("implements ", |p| p.join(&node.interfaces, " & "), "")
                });
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.part(&mut any, " ", |p| p.block(&node.fields));
            }
            NodeRef::InputObjectTypeDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("input"));
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.part(&mut any, " ", |p| p.block(&node.fields));
            }
            NodeRef::UnionTypeDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("union"));
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.part(&mut any, " ", |p| p.wrap("= ", |p| p.join(&node.types, " | "), ""));
            }
            NodeRef::EnumTypeDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| p.push("enum"));
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
                self.part(&mut any, " ", |p| p.block(&node.values));
            }
            NodeRef::EnumValueDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| p.print((&node.name).into()));
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
            }
            NodeRef::FieldDefinition(node) => {
                self.description(node.description.as_ref());
                self.print((&node.name).into());
                self.print_input_value_args(&node.arguments);
                self.push(": ");
                self.print((&node.ty).into());
                self.wrap(" ", |p| p.join(&node.directives, " "), "");
            }
            NodeRef::InputValueDefinition(node) => {
                self.description(node.description.as_ref());
                let mut any = false;
                self.part(&mut any, " ", |p| {
                    p.print((&node.name).into());
                    p.push(": ");
                    p.print((&node.ty).into());
                });
                self.part(&mut any, " ", |p| {
                    if let Some(default_value) = &node.default_value {
                        p.wrap("= ", |p| p.print(default_value.into()), "");
                    }
                });
                self.part(&mut any, " ", |p| p.join(&node.directives, " "));
            }
            NodeRef::DirectiveDefinition(node) => {
                self.description(node.description.as_ref());
                self.push("directive @");
                self.print((&node.name).into());
                self.print_input_value_args(&node.arguments);
                if node.repeatable {
                    self.push(" repeatable");
                }
                self.push(" on ");
                self.join(&node.locations, " | ");
            }
            // Type system extensions (Supergraph does not need that)
            NodeRef::SchemaExtension(_)
            | NodeRef::ScalarTypeExtension(_)
            | NodeRef::ObjectTypeExtension(_)
            | NodeRef::InterfaceTypeExtension(_)
            | NodeRef::UnionTypeExtension(_)
            | NodeRef::EnumTypeExtension(_)
            | NodeRef::InputObjectTypeExtension(_) => {}
            // Kinds without an entry in the `nodes` table.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(sdl: &str) -> String {
        print(&graphql_rs::parse(sdl).unwrap())
    }

    #[test]
    fn strings() {
        assert_eq!(
            print_string("a\"b\\c\n\t\u{1}\u{7f}é"),
            "\"a\\\"b\\\\c\\n\\t\\u0001\u{7f}é\""
        );
        assert_eq!(print_block_string("a\"\"\"b"), "\"\"\"\na\\\"\"\"b\n\"\"\"");
    }

    /// Expected output generated with the TS printer.
    #[test]
    fn type_system() {
        assert_eq!(
            p(r#"
"""
desc
"""
type A implements B & C @key(fields: "id") @x {
  "f" id(a: Int = 1 @d, b: [String!]! = ["x"]): ID! @external
}
extend type A { b: Int }
scalar S @s
union U @u = A | B
enum E { "v" V @d W }
input I { a: Int = 1, b: In }
directive @d(a: Int) repeatable on FIELD | OBJECT
schema @s { query: Q mutation: M }
"#),
            "\"\"\"\ndesc\n\"\"\"\ntype A implements B & C @key(fields: \"id\") @x {\n  \"f\"\n  id(a: Int = 1 @d, b: [String!]! = [\"x\"]): ID! @external\n}\n\n\n\nscalar S @s\n\nunion U @u = A | B\n\nenum E {\n  \"v\"\n  V @d\n  W\n}\n\ninput I {\n  a: Int = 1\n  b: In\n}\n\ndirective @d(a: Int) repeatable on FIELD | OBJECT\n\nschema @s {\n  query: Q\n  mutation: M\n}"
        );
    }

    #[test]
    fn long_args_and_operations() {
        assert_eq!(
            p("type T { f(aaaaaaaaaaaaaaaaaaaa: Int, bbbbbbbbbbbbbbbbbbbbbbbbb: Int, ccccccccccccccccccccccccc: Int): Int }"),
            "type T {\n  f(\n    aaaaaaaaaaaaaaaaaaaa: Int\n    bbbbbbbbbbbbbbbbbbbbbbbbb: Int\n    ccccccccccccccccccccccccc: Int\n  ): Int\n}"
        );
        assert_eq!(p("{ a b { c } }"), "{\n  a\n  b{\n    c\n  }\n}");
        assert_eq!(
            p("query Q($a: Int = 1) @d { x: a(b: $a) ...F ... on T { c } }"),
            "query Q($a : Int  = 1 ) @d{\n  x: a(b: $a)\n  ... F\n  ... on T {\n    c\n  }\n}"
        );
        assert_eq!(p("fragment F on T { a }"), "fragmentFonT{\n  a\n}");
        assert_eq!(
            p("{ a(o: {k: null, l: [true, 1.5, E]}) }"),
            "{\n  a(o: {k: null, l: [true, 1.5, E]})\n}"
        );
        // 79 UTF-16 code units (139 bytes): stays on one line.
        let e = "é".repeat(60);
        assert_eq!(
            p(&format!("type T {{ f(\"{e}\" a: Int, b: Int): Int }}")),
            format!("type T {{\n  f(\"{e}\"\n  a: Int, b: Int): Int\n}}")
        );
    }
}
