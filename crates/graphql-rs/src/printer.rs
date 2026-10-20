//! Port of graphql-js 17.0.2 `language/printer` (`print`) and
//! `language/printString`.

use crate::ast::*;
use crate::block_string::print_block_string;
use crate::js::js_len;

const MAX_LINE_LENGTH: usize = 80;

/// graphql-js `print`: converts an AST into a string.
///
/// Recurses once per nesting level of the AST. A document the parser produced is within
/// [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH) levels, which fits any thread's stack; check
/// one built in code or deserialized with [`check_nesting_depth`](crate::check_nesting_depth)
/// first.
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
    // One function per kind, as graphql-js's `printDocASTReducer` has one `leave` per kind:
    // printing recurses once per nesting level, and a single `match` holding every kind's
    // temporaries would make each level's stack frame as large as all of them together.
    match node {
        NodeRef::Name(n) => print_name(n),
        NodeRef::Variable(n) => print_variable(n),
        NodeRef::Document(n) => print_document(n),
        NodeRef::OperationDefinition(n) => print_operation_definition(n),
        NodeRef::VariableDefinition(n) => print_variable_definition(n),
        NodeRef::SelectionSet(n) => print_selection_set(n),
        NodeRef::Field(n) => print_field(n),
        NodeRef::Argument(n) => print_argument(n),
        NodeRef::FragmentArgument(n) => print_fragment_argument(n),
        NodeRef::FragmentSpread(n) => print_fragment_spread(n),
        NodeRef::InlineFragment(n) => print_inline_fragment(n),
        NodeRef::FragmentDefinition(n) => print_fragment_definition(n),
        NodeRef::IntValue(n) => print_int_value(n),
        NodeRef::FloatValue(n) => print_float_value(n),
        NodeRef::StringValue(n) => print_string_value(n),
        NodeRef::BooleanValue(n) => print_boolean_value(n),
        NodeRef::NullValue(n) => print_null_value(n),
        NodeRef::EnumValue(n) => print_enum_value(n),
        NodeRef::ListValue(n) => print_list_value(n),
        NodeRef::ObjectValue(n) => print_object_value(n),
        NodeRef::ObjectField(n) => print_object_field(n),
        NodeRef::Directive(n) => print_directive(n),
        NodeRef::NamedType(n) => print_named_type(n),
        NodeRef::ListType(n) => print_list_type(n),
        NodeRef::NonNullType(n) => print_non_null_type(n),
        NodeRef::SchemaDefinition(n) => print_schema_definition(n),
        NodeRef::OperationTypeDefinition(n) => print_operation_type_definition(n),
        NodeRef::ScalarTypeDefinition(n) => print_scalar_type_definition(n),
        NodeRef::ObjectTypeDefinition(n) => print_object_type_definition(n),
        NodeRef::FieldDefinition(n) => print_field_definition(n),
        NodeRef::InputValueDefinition(n) => print_input_value_definition(n),
        NodeRef::InterfaceTypeDefinition(n) => print_interface_type_definition(n),
        NodeRef::UnionTypeDefinition(n) => print_union_type_definition(n),
        NodeRef::EnumTypeDefinition(n) => print_enum_type_definition(n),
        NodeRef::EnumValueDefinition(n) => print_enum_value_definition(n),
        NodeRef::InputObjectTypeDefinition(n) => print_input_object_type_definition(n),
        NodeRef::DirectiveDefinition(n) => print_directive_definition(n),
        NodeRef::SchemaExtension(n) => print_schema_extension(n),
        NodeRef::ScalarTypeExtension(n) => print_scalar_type_extension(n),
        NodeRef::ObjectTypeExtension(n) => print_object_type_extension(n),
        NodeRef::InterfaceTypeExtension(n) => print_interface_type_extension(n),
        NodeRef::UnionTypeExtension(n) => print_union_type_extension(n),
        NodeRef::EnumTypeExtension(n) => print_enum_type_extension(n),
        NodeRef::InputObjectTypeExtension(n) => print_input_object_type_extension(n),
        NodeRef::DirectiveExtension(n) => print_directive_extension(n),
        NodeRef::TypeCoordinate(n) => print_type_coordinate(n),
        NodeRef::MemberCoordinate(n) => print_member_coordinate(n),
        NodeRef::ArgumentCoordinate(n) => print_argument_coordinate(n),
        NodeRef::DirectiveCoordinate(n) => print_directive_coordinate(n),
        NodeRef::DirectiveArgumentCoordinate(n) => print_directive_argument_coordinate(n),
    }
}

fn print_name(n: &Name) -> String {
    n.value.clone()
}

fn print_variable(n: &Variable) -> String {
    format!("${}", pn(&n.name))
}

fn print_document(n: &Document) -> String {
    join(&ps(&n.definitions), "\n\n")
}

fn print_operation_definition(n: &OperationDefinition) -> String {
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

fn print_variable_definition(n: &VariableDefinition) -> String {
    wrap("", &po(&n.description), "\n")
        + &pn(&n.variable)
        + ": "
        + &pn(&n.ty)
        + &wrap(" = ", &po(&n.default_value), "")
        + &wrap(" ", &join(&ps(&n.directives), " "), "")
}

fn print_selection_set(n: &SelectionSet) -> String {
    block(&ps(&n.selections))
}

fn print_field(n: &Field) -> String {
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

fn print_argument(n: &Argument) -> String {
    pn(&n.name) + ": " + &pn(&n.value)
}

fn print_fragment_argument(n: &FragmentArgument) -> String {
    pn(&n.name) + ": " + &pn(&n.value)
}

fn print_fragment_spread(n: &FragmentSpread) -> String {
    let prefix = "...".to_owned() + &pn(&n.name);
    wrapped_line_and_args(prefix, &ps(&n.arguments)) + &wrap(" ", &join(&ps(&n.directives), " "), "")
}

fn print_inline_fragment(n: &InlineFragment) -> String {
    join(
        &[
            "...".to_owned(),
            wrap("on ", &po(&n.type_condition), ""),
            join(&ps(&n.directives), " "),
            pn(&n.selection_set),
        ],
        " ",
    )
}

fn print_fragment_definition(n: &FragmentDefinition) -> String {
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

fn print_int_value(n: &IntValue) -> String {
    n.value.clone()
}

fn print_float_value(n: &FloatValue) -> String {
    n.value.clone()
}

fn print_string_value(n: &StringValue) -> String {
    if n.block {
        print_block_string(&n.value, false)
    } else {
        print_string(&n.value)
    }
}

fn print_boolean_value(n: &BooleanValue) -> String {
    (if n.value { "true" } else { "false" }).to_owned()
}

fn print_null_value(_: &NullValue) -> String {
    "null".to_owned()
}

fn print_enum_value(n: &EnumValue) -> String {
    n.value.clone()
}

fn print_list_value(n: &ListValue) -> String {
    let values = ps(&n.values);
    let values_line = "[".to_owned() + &join(&values, ", ") + "]";
    if js_len(&values_line) > MAX_LINE_LENGTH {
        return "[\n".to_owned() + &indent(&join(&values, "\n")) + "\n]";
    }
    values_line
}

fn print_object_value(n: &ObjectValue) -> String {
    let fields = ps(&n.fields);
    let fields_line = "{ ".to_owned() + &join(&fields, ", ") + " }";
    if js_len(&fields_line) > MAX_LINE_LENGTH {
        block(&fields)
    } else {
        fields_line
    }
}

fn print_object_field(n: &ObjectField) -> String {
    pn(&n.name) + ": " + &pn(&n.value)
}

fn print_directive(n: &Directive) -> String {
    "@".to_owned() + &pn(&n.name) + &wrap("(", &join(&ps(&n.arguments), ", "), ")")
}

fn print_named_type(n: &NamedType) -> String {
    pn(&n.name)
}

fn print_list_type(n: &ListType) -> String {
    "[".to_owned() + &pn(&n.ty) + "]"
}

fn print_non_null_type(n: &NonNullType) -> String {
    pn(&n.ty) + "!"
}

fn print_schema_definition(n: &SchemaDefinition) -> String {
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

fn print_operation_type_definition(n: &OperationTypeDefinition) -> String {
    n.operation.as_str().to_owned() + ": " + &pn(&n.ty)
}

fn print_scalar_type_definition(n: &ScalarTypeDefinition) -> String {
    wrap("", &po(&n.description), "\n") + &join(&["scalar".to_owned(), pn(&n.name), join(&ps(&n.directives), " ")], " ")
}

fn print_object_type_definition(n: &ObjectTypeDefinition) -> String {
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

fn print_field_definition(n: &FieldDefinition) -> String {
    wrap("", &po(&n.description), "\n")
        + &pn(&n.name)
        + &print_args_definition(&ps(&n.arguments))
        + ": "
        + &pn(&n.ty)
        + &wrap(" ", &join(&ps(&n.directives), " "), "")
}

fn print_input_value_definition(n: &InputValueDefinition) -> String {
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

fn print_interface_type_definition(n: &InterfaceTypeDefinition) -> String {
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

fn print_union_type_definition(n: &UnionTypeDefinition) -> String {
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

fn print_enum_type_definition(n: &EnumTypeDefinition) -> String {
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

fn print_enum_value_definition(n: &EnumValueDefinition) -> String {
    wrap("", &po(&n.description), "\n") + &join(&[pn(&n.name), join(&ps(&n.directives), " ")], " ")
}

fn print_input_object_type_definition(n: &InputObjectTypeDefinition) -> String {
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

fn print_directive_definition(n: &DirectiveDefinition) -> String {
    wrap("", &po(&n.description), "\n")
        + "directive @"
        + &pn(&n.name)
        + &print_args_definition(&ps(&n.arguments))
        + &wrap(" ", &join(&ps(&n.directives), " "), "")
        + (if n.repeatable { " repeatable" } else { "" })
        + " on "
        + &join(&ps(&n.locations), " | ")
}

fn print_schema_extension(n: &SchemaExtension) -> String {
    join(
        &[
            "extend schema".to_owned(),
            join(&ps(&n.directives), " "),
            block(&ps(&n.operation_types)),
        ],
        " ",
    )
}

fn print_scalar_type_extension(n: &ScalarTypeExtension) -> String {
    join(
        &["extend scalar".to_owned(), pn(&n.name), join(&ps(&n.directives), " ")],
        " ",
    )
}

fn print_object_type_extension(n: &ObjectTypeExtension) -> String {
    join(
        &[
            "extend type".to_owned(),
            pn(&n.name),
            wrap("implements ", &join(&ps(&n.interfaces), " & "), ""),
            join(&ps(&n.directives), " "),
            block(&ps(&n.fields)),
        ],
        " ",
    )
}

fn print_interface_type_extension(n: &InterfaceTypeExtension) -> String {
    join(
        &[
            "extend interface".to_owned(),
            pn(&n.name),
            wrap("implements ", &join(&ps(&n.interfaces), " & "), ""),
            join(&ps(&n.directives), " "),
            block(&ps(&n.fields)),
        ],
        " ",
    )
}

fn print_union_type_extension(n: &UnionTypeExtension) -> String {
    join(
        &[
            "extend union".to_owned(),
            pn(&n.name),
            join(&ps(&n.directives), " "),
            wrap("= ", &join(&ps(&n.types), " | "), ""),
        ],
        " ",
    )
}

fn print_enum_type_extension(n: &EnumTypeExtension) -> String {
    join(
        &[
            "extend enum".to_owned(),
            pn(&n.name),
            join(&ps(&n.directives), " "),
            block(&ps(&n.values)),
        ],
        " ",
    )
}

fn print_input_object_type_extension(n: &InputObjectTypeExtension) -> String {
    join(
        &[
            "extend input".to_owned(),
            pn(&n.name),
            join(&ps(&n.directives), " "),
            block(&ps(&n.fields)),
        ],
        " ",
    )
}

fn print_directive_extension(n: &DirectiveExtension) -> String {
    join(
        &[
            "extend directive @".to_owned() + &pn(&n.name),
            join(&ps(&n.directives), " "),
        ],
        " ",
    )
}

fn print_type_coordinate(n: &TypeCoordinate) -> String {
    pn(&n.name)
}

fn print_member_coordinate(n: &MemberCoordinate) -> String {
    join(&[pn(&n.name), wrap(".", &pn(&n.member_name), "")], "")
}

fn print_argument_coordinate(n: &ArgumentCoordinate) -> String {
    join(
        &[
            pn(&n.name),
            wrap(".", &pn(&n.field_name), ""),
            wrap("(", &pn(&n.argument_name), ":)"),
        ],
        "",
    )
}

fn print_directive_coordinate(n: &DirectiveCoordinate) -> String {
    join(&["@".to_owned(), pn(&n.name)], "")
}

fn print_directive_argument_coordinate(n: &DirectiveArgumentCoordinate) -> String {
    join(
        &["@".to_owned(), pn(&n.name), wrap("(", &pn(&n.argument_name), ":)")],
        "",
    )
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
