//! Port of `src/graphql/printer.ts`: the library's own printer, used for the supergraph SDL.
//! It differs from graphql-js `print` (e.g. type extensions print as empty strings), so it is
//! ported here and never replaced by `graphql_js::print`.

use graphql_js::{Argument, InputValueDefinition, NodeRef, OperationType};

/// `printString(string)`: `JSON.stringify(string)`.
/// NOTE(port): serde_json escapes exactly like `JSON.stringify` for valid Unicode strings
/// (`\"`, `\\`, `\b`, `\f`, `\n`, `\r`, `\t`, other C0 controls as lowercase `\u00xx`, the
/// rest verbatim). Rust strings cannot hold the lone surrogates `JSON.stringify` would escape.
fn print_string(string: &str) -> String {
    serde_json::to_string(string).unwrap_or_default()
}

/// `printBlockString(string)`.
fn print_block_string(string: &str) -> String {
    format!("\"\"\"\n{}\n\"\"\"", string.replace("\"\"\"", "\\\"\"\""))
}

/// `node.directives?.map(print)`.
fn print_all<'a, T: 'a>(nodes: &'a [T]) -> Vec<String>
where
    &'a T: Into<NodeRef<'a>>,
{
    nodes.iter().map(|n| print(n)).collect()
}

/// `print(node?)`: `""` for a missing node or a kind without a printer in the `nodes` table.
/// The `nodes` table is the `match` inside.
pub fn print<'a>(node: impl Into<NodeRef<'a>>) -> String {
    match node.into() {
        NodeRef::Document(node) => {
            if !node.definitions.is_empty() {
                node.definitions.iter().map(|d| print(d)).collect::<Vec<_>>().join("\n\n")
            } else {
                String::new()
            }
        }
        // ...
        NodeRef::OperationDefinition(node) => {
            if node.operation == OperationType::Query
                && node.name.is_none()
                && node.variable_definitions.is_empty()
                && node.directives.is_empty()
            {
                return print(&node.selection_set);
            }

            join(
                &[
                    node.operation.as_str().to_string(),
                    print_opt(node.name.as_ref())
                        + &wrap("(", &join(&print_all(&node.variable_definitions), ", "), ")"),
                    join(&print_all(&node.directives), " "),
                ],
                " ",
            ) + &print(&node.selection_set)
        }
        NodeRef::VariableDefinition(node) => {
            // NOTE(port): a plain `[...].join(" ")`, so empty parts still add separators.
            [
                print(&node.variable),
                ":".to_string(),
                print(&node.ty),
                wrap(" = ", &print_opt(node.default_value.as_ref()), ""),
                join(&print_all(&node.directives), " "),
            ]
            .join(" ")
        }
        NodeRef::Field(node) => {
            join(
                &[
                    match &node.alias {
                        Some(alias) => format!("{}: ", alias.value),
                        None => String::new(),
                    } + &node.name.value
                        + &print_args(&node.arguments),
                    join(&print_all(&node.directives), " "),
                ],
                " ",
            ) + &print_opt(node.selection_set.as_ref())
        }
        NodeRef::StringValue(node) => {
            if node.block {
                print_block_string(&node.value)
            } else {
                print_string(&node.value)
            }
        }
        NodeRef::BooleanValue(node) => node.value.to_string(),
        NodeRef::NullValue(_) => "null".to_string(),
        NodeRef::IntValue(node) => node.value.clone(),
        NodeRef::FloatValue(node) => node.value.clone(),
        NodeRef::EnumValue(node) => node.value.clone(),
        NodeRef::Name(node) => node.value.clone(),
        NodeRef::Variable(node) => format!("${}", node.name.value),
        NodeRef::ListValue(node) => format!("[{}]", print_all(&node.values).join(", ")),
        NodeRef::ObjectValue(node) => format!("{{{}}}", print_all(&node.fields).join(", ")),
        NodeRef::ObjectField(node) => format!("{}: {}", node.name.value, print(&node.value)),
        NodeRef::SelectionSet(node) => block(&print_all(&node.selections)),
        NodeRef::Argument(node) => format!("{}: {}", node.name.value, print(&node.value)),
        NodeRef::FragmentSpread(node) => join(
            &["...".to_string(), print(&node.name), join(&print_all(&node.directives), " ")],
            " ",
        ),
        NodeRef::InlineFragment(node) => join(
            &[
                "...".to_string(),
                match &node.type_condition {
                    Some(type_condition) => format!("on {}", type_condition.name.value),
                    None => String::new(),
                },
                join(&print_all(&node.directives), " "),
                print(&node.selection_set),
            ],
            " ",
        ),
        // NOTE(port): `join` without a separator, as in the TS.
        NodeRef::FragmentDefinition(node) => join(
            &[
                "fragment".to_string(),
                print(&node.name),
                "on".to_string(),
                node.type_condition.name.value.clone(),
                join(&print_all(&node.directives), " "),
                print(&node.selection_set),
            ],
            "",
        ),
        NodeRef::Directive(node) => format!("@{}{}", node.name.value, print_args(&node.arguments)),
        NodeRef::NamedType(node) => node.name.value.clone(),
        NodeRef::ListType(node) => format!("[{}]", print(&*node.ty)),
        NodeRef::NonNullType(node) => format!("{}!", print(&*node.ty)),
        // Type system definitions
        NodeRef::SchemaDefinition(node) => join(
            &[
                "schema".to_string(),
                join(&print_all(&node.directives), " "),
                block(&print_all(&node.operation_types)),
            ],
            " ",
        ),
        NodeRef::OperationTypeDefinition(node) => format!("{}: {}", node.operation.as_str(), print(&node.ty)),
        NodeRef::ScalarTypeDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(
                    &["scalar".to_string(), print(&node.name), join(&print_all(&node.directives), " ")],
                    " ",
                )
        }
        NodeRef::ObjectTypeDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(
                    &[
                        "type".to_string(),
                        print(&node.name),
                        wrap("implements ", &join(&print_all(&node.interfaces), " & "), ""),
                        join(&print_all(&node.directives), " "),
                        block(&print_all(&node.fields)),
                    ],
                    " ",
                )
        }
        NodeRef::InterfaceTypeDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(
                    &[
                        "interface".to_string(),
                        print(&node.name),
                        wrap("implements ", &join(&print_all(&node.interfaces), " & "), ""),
                        join(&print_all(&node.directives), " "),
                        block(&print_all(&node.fields)),
                    ],
                    " ",
                )
        }
        NodeRef::InputObjectTypeDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(
                    &[
                        "input".to_string(),
                        print(&node.name),
                        join(&print_all(&node.directives), " "),
                        block(&print_all(&node.fields)),
                    ],
                    " ",
                )
        }
        NodeRef::UnionTypeDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(
                    &[
                        "union".to_string(),
                        print(&node.name),
                        join(&print_all(&node.directives), " "),
                        wrap("= ", &join(&print_all(&node.types), " | "), ""),
                    ],
                    " ",
                )
        }
        NodeRef::EnumTypeDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(
                    &[
                        "enum".to_string(),
                        print(&node.name),
                        join(&print_all(&node.directives), " "),
                        block(&print_all(&node.values)),
                    ],
                    " ",
                )
        }
        NodeRef::EnumValueDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(&[print(&node.name), join(&print_all(&node.directives), " ")], " ")
        }
        NodeRef::FieldDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &print(&node.name)
                + &print_input_value_args(&node.arguments)
                + ": "
                + &print(&node.ty)
                + &wrap(" ", &join(&print_all(&node.directives), " "), "")
        }
        NodeRef::InputValueDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + &join(
                    &[
                        format!("{}: {}", print(&node.name), print(&node.ty)),
                        wrap("= ", &print_opt(node.default_value.as_ref()), ""),
                        join(&print_all(&node.directives), " "),
                    ],
                    " ",
                )
        }
        NodeRef::DirectiveDefinition(node) => {
            wrap("", &print_opt(node.description.as_ref()), "\n")
                + "directive @"
                + &print(&node.name)
                + &print_input_value_args(&node.arguments)
                + if node.repeatable { " repeatable" } else { "" }
                + " on "
                + &join(&print_all(&node.locations), " | ")
        }
        // Type system extensions (Supergraph does not need that)
        NodeRef::SchemaExtension(_)
        | NodeRef::ScalarTypeExtension(_)
        | NodeRef::ObjectTypeExtension(_)
        | NodeRef::InterfaceTypeExtension(_)
        | NodeRef::UnionTypeExtension(_)
        | NodeRef::EnumTypeExtension(_)
        | NodeRef::InputObjectTypeExtension(_) => String::new(),
        // Kinds without an entry in the `nodes` table.
        _ => String::new(),
    }
}

/// `print(node)` for an optional node.
pub fn print_opt<'a, N: 'a>(node: Option<&'a N>) -> String
where
    &'a N: Into<NodeRef<'a>>,
{
    match node {
        Some(n) => print(n),
        None => String::new(),
    }
}

/// The body of `printArgs` once the nodes are printed.
/// NOTE(port): `argsLine.length` counts UTF-16 code units (descriptions can be non-ASCII).
fn print_printed_args(args: Vec<String>) -> String {
    if args.is_empty() {
        return String::new();
    }
    let args_line = format!("({})", args.join(", "));
    if args_line.encode_utf16().count() > 80 {
        format!("(\n  {}\n)", args.join("\n").replace('\n', "\n  "))
    } else {
        args_line
    }
}

/// `printArgs(nodes)` for `ArgumentNode[]`.
fn print_args(nodes: &[Argument]) -> String {
    print_printed_args(print_all(nodes))
}

/// `printArgs(nodes)` for `InputValueDefinitionNode[]`.
fn print_input_value_args(nodes: &[InputValueDefinition]) -> String {
    print_printed_args(print_all(nodes))
}

/// `block(array)`.
fn block(array: &[String]) -> String {
    wrap("{\n", &indent(&join(array, "\n")), "\n}")
}

/// `indent(str)`.
fn indent(s: &str) -> String {
    wrap("  ", &s.replace('\n', "\n  "), "")
}

/// `wrap(start, maybeString, end = "")`: `""` when `maybeString` is empty.
fn wrap(start: &str, maybe_string: &str, end: &str) -> String {
    if maybe_string.is_empty() {
        String::new()
    } else {
        format!("{start}{maybe_string}{end}")
    }
}

/// `join(maybeArray, separator = "")`: joins the non-empty strings.
fn join(maybe_array: &[String], separator: &str) -> String {
    maybe_array
        .iter()
        .filter(|x| !x.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(sdl: &str) -> String {
        print(&graphql_js::parse(sdl).unwrap())
    }

    #[test]
    fn strings() {
        assert_eq!(print_string("a\"b\\c\n\t\u{1}\u{7f}é"), "\"a\\\"b\\\\c\\n\\t\\u0001\u{7f}é\"");
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
        assert_eq!(p("{ a(o: {k: null, l: [true, 1.5, E]}) }"), "{\n  a(o: {k: null, l: [true, 1.5, E]})\n}");
        // 79 UTF-16 code units (139 bytes): stays on one line.
        let e = "é".repeat(60);
        assert_eq!(
            p(&format!("type T {{ f(\"{e}\" a: Int, b: Int): Int }}")),
            format!("type T {{\n  f(\"{e}\"\n  a: Int, b: Int): Int\n}}")
        );
    }
}
