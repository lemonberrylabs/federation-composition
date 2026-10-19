//! Port of `src/graphql/printer.ts`: the library's own printer, used for the supergraph SDL.
//! It differs from graphql-js `print` (e.g. type extensions print as empty strings), so it is
//! ported here and never replaced by `graphql_js::print`.

use graphql_js::{Argument, InputValueDefinition, NodeRef};

/// `printString(string)`: `JSON.stringify(string)`.
fn print_string(string: &str) -> String {
    todo!()
}

/// `printBlockString(string)`.
fn print_block_string(string: &str) -> String {
    format!("\"\"\"\n{}\n\"\"\"", string.replace("\"\"\"", "\\\"\"\""))
}

/// `print(node?)`: `""` for a missing node or a kind without a printer in the `nodes` table.
/// The `nodes` table is the `match` inside.
pub fn print<'a>(node: impl Into<NodeRef<'a>>) -> String {
    todo!()
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

/// `printArgs(nodes)` for `ArgumentNode[]`.
fn print_args(nodes: &[Argument]) -> String {
    todo!()
}

/// `printArgs(nodes)` for `InputValueDefinitionNode[]`.
fn print_input_value_args(nodes: &[InputValueDefinition]) -> String {
    todo!()
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
