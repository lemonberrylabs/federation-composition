//! Port of graphql-js 17.0.2 `language/printer` (`print`) and
//! `language/printString`.

use crate::ast::*;
use crate::block_string::print_block_string;
use crate::js::js_len;

const MAX_LINE_LENGTH: usize = 80;

/// graphql-js `print`: converts an AST into a string.
pub fn print<'a>(node: impl Into<NodeRef<'a>>) -> String {
    p(node.into())
}

fn pn<T: AsNodeRef + ?Sized>(node: &T) -> String {
    p(node.as_node_ref())
}

fn po<T: AsNodeRef>(node: &Option<T>) -> String {
    match node {
        Some(n) => pn(n),
        None => String::new(),
    }
}

fn ps<T: AsNodeRef>(nodes: &[T]) -> Vec<String> {
    nodes.iter().map(pn).collect()
}

fn p(node: NodeRef<'_>) -> String {
    match node {
        NodeRef::Name(n) => n.value.clone(),
        NodeRef::Variable(n) => format!("${}", pn(&n.name)),
        NodeRef::Document(n) => join(&ps(&n.definitions), "\n\n"),
        NodeRef::OperationDefinition(n) => {
            let var_defs = ps(&n.variable_definitions);
            let var_defs = if has_multiline_items(&var_defs) {
                wrap("(\n", &join(&var_defs, "\n"), "\n)")
            } else {
                wrap("(", &join(&var_defs, ", "), ")")
            };
            let prefix = wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        n.operation.as_str().to_owned(),
                        join(&[po(&n.name), var_defs], ""),
                        join(&ps(&n.directives), " "),
                    ],
                    " ",
                );
            let selection_set = pn(&n.selection_set);
            if prefix == "query" {
                selection_set
            } else {
                prefix + " " + &selection_set
            }
        }
        NodeRef::VariableDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &pn(&n.variable)
                + ": "
                + &pn(&n.ty)
                + &wrap(" = ", &po(&n.default_value), "")
                + &wrap(" ", &join(&ps(&n.directives), " "), "")
        }
        NodeRef::SelectionSet(n) => block(&ps(&n.selections)),
        NodeRef::Field(n) => {
            let prefix = join(&[wrap("", &po(&n.alias), ": "), pn(&n.name)], "");
            join(
                &[
                    wrapped_line_and_args(prefix, &ps(&n.arguments)),
                    wrap(" ", &join(&ps(&n.directives), " "), ""),
                    wrap(" ", &po(&n.selection_set), ""),
                ],
                "",
            )
        }
        NodeRef::Argument(n) => pn(&n.name) + ": " + &pn(&n.value),
        NodeRef::FragmentArgument(n) => pn(&n.name) + ": " + &pn(&n.value),
        NodeRef::FragmentSpread(n) => {
            let prefix = "...".to_owned() + &pn(&n.name);
            wrapped_line_and_args(prefix, &ps(&n.arguments))
                + &wrap(" ", &join(&ps(&n.directives), " "), "")
        }
        NodeRef::InlineFragment(n) => join(
            &[
                "...".to_owned(),
                wrap("on ", &po(&n.type_condition), ""),
                join(&ps(&n.directives), " "),
                pn(&n.selection_set),
            ],
            " ",
        ),
        NodeRef::FragmentDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + "fragment "
                + &pn(&n.name)
                + &wrap("(", &join(&ps(&n.variable_definitions), ", "), ")")
                + " on "
                + &pn(&n.type_condition)
                + " "
                + &wrap("", &join(&ps(&n.directives), " "), " ")
                + &pn(&n.selection_set)
        }
        NodeRef::IntValue(n) => n.value.clone(),
        NodeRef::FloatValue(n) => n.value.clone(),
        NodeRef::StringValue(n) => {
            if n.block {
                print_block_string(&n.value, false)
            } else {
                print_string(&n.value)
            }
        }
        NodeRef::BooleanValue(n) => (if n.value { "true" } else { "false" }).to_owned(),
        NodeRef::NullValue(_) => "null".to_owned(),
        NodeRef::EnumValue(n) => n.value.clone(),
        NodeRef::ListValue(n) => {
            let values = ps(&n.values);
            let values_line = "[".to_owned() + &join(&values, ", ") + "]";
            if js_len(&values_line) > MAX_LINE_LENGTH {
                return "[\n".to_owned() + &indent(&join(&values, "\n")) + "\n]";
            }
            values_line
        }
        NodeRef::ObjectValue(n) => {
            let fields = ps(&n.fields);
            let fields_line = "{ ".to_owned() + &join(&fields, ", ") + " }";
            if js_len(&fields_line) > MAX_LINE_LENGTH {
                block(&fields)
            } else {
                fields_line
            }
        }
        NodeRef::ObjectField(n) => pn(&n.name) + ": " + &pn(&n.value),
        NodeRef::Directive(n) => {
            "@".to_owned() + &pn(&n.name) + &wrap("(", &join(&ps(&n.arguments), ", "), ")")
        }
        NodeRef::NamedType(n) => pn(&n.name),
        NodeRef::ListType(n) => "[".to_owned() + &pn(&n.ty) + "]",
        NodeRef::NonNullType(n) => pn(&n.ty) + "!",
        NodeRef::SchemaDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        "schema".to_owned(),
                        join(&ps(&n.directives), " "),
                        block(&ps(&n.operation_types)),
                    ],
                    " ",
                )
        }
        NodeRef::OperationTypeDefinition(n) => n.operation.as_str().to_owned() + ": " + &pn(&n.ty),
        NodeRef::ScalarTypeDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        "scalar".to_owned(),
                        pn(&n.name),
                        join(&ps(&n.directives), " "),
                    ],
                    " ",
                )
        }
        NodeRef::ObjectTypeDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        "type".to_owned(),
                        pn(&n.name),
                        wrap("implements ", &join(&ps(&n.interfaces), " & "), ""),
                        join(&ps(&n.directives), " "),
                        block(&ps(&n.fields)),
                    ],
                    " ",
                )
        }
        NodeRef::FieldDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &pn(&n.name)
                + &print_args_definition(&ps(&n.arguments))
                + ": "
                + &pn(&n.ty)
                + &wrap(" ", &join(&ps(&n.directives), " "), "")
        }
        NodeRef::InputValueDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        pn(&n.name) + ": " + &pn(&n.ty),
                        wrap("= ", &po(&n.default_value), ""),
                        join(&ps(&n.directives), " "),
                    ],
                    " ",
                )
        }
        NodeRef::InterfaceTypeDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        "interface".to_owned(),
                        pn(&n.name),
                        wrap("implements ", &join(&ps(&n.interfaces), " & "), ""),
                        join(&ps(&n.directives), " "),
                        block(&ps(&n.fields)),
                    ],
                    " ",
                )
        }
        NodeRef::UnionTypeDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        "union".to_owned(),
                        pn(&n.name),
                        join(&ps(&n.directives), " "),
                        wrap("= ", &join(&ps(&n.types), " | "), ""),
                    ],
                    " ",
                )
        }
        NodeRef::EnumTypeDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        "enum".to_owned(),
                        pn(&n.name),
                        join(&ps(&n.directives), " "),
                        block(&ps(&n.values)),
                    ],
                    " ",
                )
        }
        NodeRef::EnumValueDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(&[pn(&n.name), join(&ps(&n.directives), " ")], " ")
        }
        NodeRef::InputObjectTypeDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + &join(
                    &[
                        "input".to_owned(),
                        pn(&n.name),
                        join(&ps(&n.directives), " "),
                        block(&ps(&n.fields)),
                    ],
                    " ",
                )
        }
        NodeRef::DirectiveDefinition(n) => {
            wrap("", &po(&n.description), "\n")
                + "directive @"
                + &pn(&n.name)
                + &print_args_definition(&ps(&n.arguments))
                + &wrap(" ", &join(&ps(&n.directives), " "), "")
                + (if n.repeatable { " repeatable" } else { "" })
                + " on "
                + &join(&ps(&n.locations), " | ")
        }
        NodeRef::SchemaExtension(n) => join(
            &[
                "extend schema".to_owned(),
                join(&ps(&n.directives), " "),
                block(&ps(&n.operation_types)),
            ],
            " ",
        ),
        NodeRef::ScalarTypeExtension(n) => join(
            &[
                "extend scalar".to_owned(),
                pn(&n.name),
                join(&ps(&n.directives), " "),
            ],
            " ",
        ),
        NodeRef::ObjectTypeExtension(n) => join(
            &[
                "extend type".to_owned(),
                pn(&n.name),
                wrap("implements ", &join(&ps(&n.interfaces), " & "), ""),
                join(&ps(&n.directives), " "),
                block(&ps(&n.fields)),
            ],
            " ",
        ),
        NodeRef::InterfaceTypeExtension(n) => join(
            &[
                "extend interface".to_owned(),
                pn(&n.name),
                wrap("implements ", &join(&ps(&n.interfaces), " & "), ""),
                join(&ps(&n.directives), " "),
                block(&ps(&n.fields)),
            ],
            " ",
        ),
        NodeRef::UnionTypeExtension(n) => join(
            &[
                "extend union".to_owned(),
                pn(&n.name),
                join(&ps(&n.directives), " "),
                wrap("= ", &join(&ps(&n.types), " | "), ""),
            ],
            " ",
        ),
        NodeRef::EnumTypeExtension(n) => join(
            &[
                "extend enum".to_owned(),
                pn(&n.name),
                join(&ps(&n.directives), " "),
                block(&ps(&n.values)),
            ],
            " ",
        ),
        NodeRef::InputObjectTypeExtension(n) => join(
            &[
                "extend input".to_owned(),
                pn(&n.name),
                join(&ps(&n.directives), " "),
                block(&ps(&n.fields)),
            ],
            " ",
        ),
        NodeRef::DirectiveExtension(n) => join(
            &[
                "extend directive @".to_owned() + &pn(&n.name),
                join(&ps(&n.directives), " "),
            ],
            " ",
        ),
        NodeRef::TypeCoordinate(n) => pn(&n.name),
        NodeRef::MemberCoordinate(n) => {
            join(&[pn(&n.name), wrap(".", &pn(&n.member_name), "")], "")
        }
        NodeRef::ArgumentCoordinate(n) => join(
            &[
                pn(&n.name),
                wrap(".", &pn(&n.field_name), ""),
                wrap("(", &pn(&n.argument_name), ":)"),
            ],
            "",
        ),
        NodeRef::DirectiveCoordinate(n) => join(&["@".to_owned(), pn(&n.name)], ""),
        NodeRef::DirectiveArgumentCoordinate(n) => join(
            &[
                "@".to_owned(),
                pn(&n.name),
                wrap("(", &pn(&n.argument_name), ":)"),
            ],
            "",
        ),
    }
}

/// The argument list of `FieldDefinition` and `DirectiveDefinition`.
fn print_args_definition(args: &[String]) -> String {
    if has_multiline_items(args) {
        wrap("(\n", &indent(&join(args, "\n")), "\n)")
    } else {
        wrap("(", &join(args, ", "), ")")
    }
}

/// graphql-js printer `join`: drops empty strings.
fn join(items: &[String], separator: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for item in items {
        if item.is_empty() {
            continue;
        }
        if !first {
            out.push_str(separator);
        }
        first = false;
        out.push_str(item);
    }
    out
}

fn block(items: &[String]) -> String {
    wrap("{\n", &indent(&join(items, "\n")), "\n}")
}

fn wrap(start: &str, s: &str, end: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        let mut out = String::with_capacity(start.len() + s.len() + end.len());
        out.push_str(start);
        out.push_str(s);
        out.push_str(end);
        out
    }
}

fn indent(s: &str) -> String {
    wrap("  ", &s.replace('\n', "\n  "), "")
}

fn has_multiline_items(items: &[String]) -> bool {
    items.iter().any(|s| s.contains('\n'))
}

fn wrapped_line_and_args(prefix: String, args: &[String]) -> String {
    let args_line = prefix.clone() + &wrap("(", &join(args, ", "), ")");
    if js_len(&args_line) > MAX_LINE_LENGTH {
        return prefix + &wrap("(\n", &indent(&join(args, "\n")), "\n)");
    }
    args_line
}

/// graphql-js `printString`: a double-quoted string with JSON-style escapes.
pub fn print_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
