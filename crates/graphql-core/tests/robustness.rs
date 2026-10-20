//! The nesting limit (`MAX_NESTING_DEPTH`, SECURITY.md "Input limits") and inputs that once made
//! the crate panic or overflow its stack, found by fuzzing (`fuzz/`) or by review.
//!
//! The tests run on threads with a 2 MiB stack, the default of a spawned thread, so they fail
//! the same way under any `RUST_MIN_STACK`.

use graphql_core::{
    assert_valid_sdl, build_ast_schema, build_schema, check_nesting_depth, parse, parse_type, parse_value,
    parse_with_options, print, validate_schema, validate_sdl, Definition, Document, ParseOptions, Type,
    MAX_NESTING_DEPTH,
};

/// Runs `test` on a thread with a 2 MiB stack.
fn on_small_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(test)
        .expect("spawn")
        .join()
        .expect("the test passes");
}

const TOO_DEEP: &str = "Syntax Error: Document nesting depth exceeds 128. Parsing aborted.";

/// Documents nested `levels` deep, one per kind of nesting level.
fn nested_documents(levels: usize) -> Vec<String> {
    let list = |inner: &str| format!("{}{inner}{}", "[".repeat(levels), "]".repeat(levels));
    let object = format!("{}1{}", "{a: ".repeat(levels), "}".repeat(levels));
    // An object value inside a list value: two levels per repetition.
    let mixed = format!(
        "{}1{}",
        "[{a: ".repeat(levels.div_ceil(2)),
        "}]".repeat(levels.div_ceil(2))
    );
    // A list value in a selection set: one level less than `levels` for the list.
    let in_selection = format!("{}$v{}", "[".repeat(levels - 1), "]".repeat(levels - 1));
    vec![
        format!("type Query {{ f(a: {}): Int }}", list("Int")),
        format!("type Query {{ f(a: {}): Int }}", list("Int!")),
        format!("type Query {{ f: Int @d(a: {}) }}", list("1")),
        format!("type Query {{ f: Int @d(a: {object}) }}"),
        format!("type Query {{ f: Int @d(a: {mixed}) }}"),
        format!("{}a{}", "{ a ".repeat(levels - 1) + "{ ", " }".repeat(levels)),
        format!("query Q($v: {}) {{ a(b: {in_selection}) }}", list("Int")),
    ]
}

#[test]
fn documents_nested_to_the_limit_parse_print_and_build() {
    on_small_stack(|| {
        for source in nested_documents(MAX_NESTING_DEPTH) {
            let document = parse(&source).unwrap_or_else(|e| panic!("{e}: {source}"));
            check_nesting_depth(&document, MAX_NESTING_DEPTH).expect("within the limit");
            let printed = print(&document);
            assert!(parse(&printed).is_ok_and(|reparsed| reparsed == document), "{source}");
            // Building visits and validates every definition (most of these are invalid SDL).
            let _ = build_ast_schema(&document);
        }
        let schema = build_schema(&format!(
            "input I {{ a: I b: Int }}\ntype Query {{ f(a: {}Int{} = {}1{}, b: I = {}{{b: 1}}{}): Int }}",
            "[".repeat(128),
            "]".repeat(128),
            "[".repeat(128),
            "]".repeat(128),
            "{a: ".repeat(127),
            "}".repeat(127),
        ))
        .expect("a valid schema");
        assert!(validate_schema(&schema).expect("validation runs").is_empty());
    });
}

#[test]
fn documents_nested_over_the_limit_are_syntax_errors() {
    on_small_stack(|| {
        for source in nested_documents(MAX_NESTING_DEPTH + 1) {
            let error = parse(&source).err().unwrap_or_else(|| panic!("parsed: {source}"));
            assert_eq!(error.message, TOO_DEEP, "{source}");
        }
        let list = format!("{}1{}", "[".repeat(129), "]".repeat(129));
        assert_eq!(parse_value(&list).unwrap_err().message, TOO_DEEP);
        let ty = format!("{}Int{}", "[".repeat(129), "]".repeat(129));
        assert_eq!(parse_type(&ty).unwrap_err().message, TOO_DEEP);
        // Far deeper input is rejected the same way, without recursing into it.
        let deep = "[".repeat(10_000_000);
        assert_eq!(parse_value(&deep).unwrap_err().message, TOO_DEEP);
    });
}

#[test]
fn the_limit_is_an_option() {
    let options = |max_nesting_depth| ParseOptions {
        max_nesting_depth,
        ..Default::default()
    };
    assert_eq!(
        parse_with_options("{ a { b } }", options(1)).unwrap_err().message,
        "Syntax Error: Document nesting depth exceeds 1. Parsing aborted."
    );
    parse_with_options("{ a { b } }", options(2)).expect("two levels");
    let document = parse_with_options(&nested_documents(200)[0], options(200)).expect("200 levels");
    assert_eq!(
        check_nesting_depth(&document, MAX_NESTING_DEPTH).unwrap_err().message,
        "AST nesting depth exceeds 128."
    );
}

#[test]
fn documents_built_over_the_limit_are_rejected_before_building() {
    on_small_stack(|| {
        let options = ParseOptions {
            max_nesting_depth: 1000,
            ..Default::default()
        };
        let document = parse_with_options(&nested_documents(1000)[0], options).expect("1000 levels");
        assert_eq!(
            build_ast_schema(&document).unwrap_err().message,
            "AST nesting depth exceeds 128."
        );
        assert_eq!(
            assert_valid_sdl(&document).unwrap_err().message,
            "AST nesting depth exceeds 128."
        );
    });
}

// Long chains of types, which graphql-js's recursive searches walk one recursion level per type.

const CHAIN: usize = 10_000;

#[test]
fn long_chain_of_object_types_builds() {
    on_small_stack(|| {
        let mut sdl = String::from("type Query { t: T0 }\n");
        for i in 0..CHAIN {
            sdl += &format!("type T{i} {{ a: T{} }}\n", i + 1);
        }
        sdl += &format!("type T{CHAIN} {{ id: ID }}\n");
        let schema = build_schema(&sdl).expect("a valid schema");
        assert!(validate_schema(&schema).expect("validation runs").is_empty());
    });
}

#[test]
fn long_cycle_of_non_null_input_fields_is_reported() {
    on_small_stack(|| {
        let mut sdl = String::from("type Query { f(a: I0): Int }\n");
        for i in 0..CHAIN {
            sdl += &format!("input I{i} {{ a: I{}! }}\n", (i + 1) % CHAIN);
        }
        let schema = build_schema(&sdl).expect("a schema");
        let errors = validate_schema(&schema).expect("validation runs");
        assert_eq!(errors.len(), 1);
        assert!(errors[0]
            .message
            .starts_with("Input Object I0 cannot be provided a finite value because it references itself through fields: I0.a, I1.a, "));
    });
}

#[test]
fn long_chain_of_input_default_values_is_validated() {
    on_small_stack(|| {
        let mut sdl = String::from("type Query { f(a: I0 = {}): Int }\n");
        for i in 0..CHAIN {
            sdl += &format!("input I{i} {{ a: I{} = {{}} }}\n", i + 1);
        }
        sdl += &format!("input I{CHAIN} {{ id: ID = 1 }}\n");
        let schema = build_schema(&sdl).expect("a schema");
        assert!(validate_schema(&schema).expect("validation runs").is_empty());

        // The same chain closed into a cycle.
        let sdl = sdl.replace(&format!("a: I{CHAIN} = {{}}"), "a: I0 = {}");
        let schema = build_schema(&sdl).expect("a schema");
        let errors = validate_schema(&schema).expect("validation runs");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.starts_with(
            "Invalid circular reference. The default value of Input Object field I0.a references itself via the default values of: I1.a, I2.a, "
        ));
    });
}

// Found by fuzzing (`parse`): the printer round trip.

#[test]
fn a_query_in_shorthand_after_a_definition_without_a_body_keeps_its_keyword() {
    // graphql-js prints the anonymous query as `{ … }`, and that `{` reads back as the previous
    // definition's body. This printer writes `query` there, so the document round-trips.
    let document = parse("type RootMutati query { D mu: RootMu }").unwrap();
    assert_eq!(print(&document), "type RootMutati\n\nquery {\n  D\n  mu: RootMu\n}");
    assert_eq!(parse(&print(&document)).unwrap(), document);
    for previous in [
        "type A",
        "interface A",
        "input A",
        "enum A",
        "extend type A @d",
        "extend interface A @d",
        "extend input A @d",
        "extend enum A @d",
        "extend schema @d",
    ] {
        let document = previous_then_query(previous);
        let printed = print(&document);
        assert_eq!(printed, format!("{previous}\n\nquery {{\n  a\n}}"));
        assert_eq!(parse(&printed).unwrap(), document, "{previous}");
    }
    // Elsewhere the shorthand stays, as in graphql-js.
    for previous in [
        "type A { a: Int }",
        "union U = A",
        "scalar S",
        "{ b }",
        "fragment F on A { a }",
    ] {
        let document = previous_then_query(previous);
        assert!(print(&document).ends_with("\n\n{\n  a\n}"), "{previous}");
    }
    // A query printed on its own keeps the shorthand.
    assert_eq!(print(&parse("query { a }").unwrap()), "{\n  a\n}");
}

/// `previous` followed by the query `{ a }`, parsed separately: in one text the `{` would be
/// read as part of `previous`.
fn previous_then_query(previous: &str) -> Document {
    let mut document = parse(previous).unwrap();
    document.definitions.extend(parse("{ a }").unwrap().definitions);
    document
}

// Found by review: documents built in code, which the parser cannot produce.

#[test]
fn directive_location_that_is_not_one() {
    let mut document = parse("directive @d on FIELD_DEFINITION\ntype Query { f: Int @d }").unwrap();
    let Definition::DirectiveDefinition(directive) = &mut document.definitions[0] else {
        panic!("a directive definition");
    };
    directive.locations[0].value = "NOWHERE".into();
    // graphql-js compares location names as strings: no location matches it.
    let errors: Vec<String> = validate_sdl(&document).into_iter().map(|e| e.message).collect();
    assert_eq!(errors, ["Directive \"@d\" may not be used on FIELD_DEFINITION."]);
    document.definitions.pop();
    assert_eq!(
        build_ast_schema(&document).unwrap_err().message,
        "Unexpected directive location \"NOWHERE\"."
    );
}

#[test]
fn non_null_type_inside_a_non_null_type_is_a_level() {
    let mut ty = Type::named("Int");
    for _ in 0..MAX_NESTING_DEPTH {
        ty = Type::non_null(ty);
    }
    // The innermost `!` wraps a named type; each of the others is a level.
    check_nesting_depth(&ty, MAX_NESTING_DEPTH).expect("127 levels");
    let ty = Type::non_null(Type::non_null(ty));
    assert!(check_nesting_depth(&ty, MAX_NESTING_DEPTH).is_err());
}
