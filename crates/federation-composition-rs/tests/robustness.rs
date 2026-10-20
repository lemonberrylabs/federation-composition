//! Inputs that once made the library panic or overflow its stack, found by fuzzing (`fuzz/`) or
//! by review, and the nesting limits (SECURITY.md, "Input limits"). Each one must give a result
//! or an `Err`, the way upstream's TypeScript gives a result or throws. The expected outcomes of
//! the documents built in code are upstream's, for the same JavaScript AST.
//!
//! The tests run on threads with a 2 MiB stack, the default of a spawned thread, so they fail
//! the same way under any `RUST_MIN_STACK`.

use federation_composition_rs::graphql_rs::{
    parse, parse_with_options, Definition, Directive, Document, ListType, NamedType, ParseOptions, Type, Value,
    Variable, MAX_NESTING_DEPTH,
};
use federation_composition_rs::{compose_services, CompositionResult, Error, ServiceDefinition};

const HEADER: &str =
    "extend schema @link(url: \"https://specs.apollo.dev/federation/v2.3\", import: [\"@key\", \"@shareable\", \"@tag\"])\n";

/// Runs `test` on a thread with a 2 MiB stack.
fn on_small_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(test)
        .expect("spawn")
        .join()
        .expect("the test passes");
}

fn compose_documents(documents: Vec<Document>) -> Result<CompositionResult, Error> {
    let services: Vec<ServiceDefinition> = documents
        .into_iter()
        .zip(["a", "b", "c", "d"])
        .map(|(document, name)| ServiceDefinition::new(name, document))
        .collect();
    compose_services(&services)
}

fn compose(subgraphs: &[&str]) -> Result<CompositionResult, Error> {
    compose_documents(
        subgraphs
            .iter()
            .map(|sdl| parse(sdl).expect("the subgraph parses"))
            .collect(),
    )
}

fn messages(result: &CompositionResult) -> Vec<String> {
    result
        .errors()
        .unwrap_or_default()
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

/// `String(error)` of what the TypeScript throws.
fn thrown(result: Result<CompositionResult, Error>) -> String {
    match result {
        Ok(result) => panic!("expected a thrown error, got {:?}", messages(&result)),
        Err(error) => error.into_graphql_error().message,
    }
}

fn field_definitions(
    document: &mut Document,
    index: usize,
) -> &mut Vec<federation_composition_rs::graphql_rs::FieldDefinition> {
    match &mut document.definitions[index] {
        Definition::ObjectTypeDefinition(object) => &mut object.fields,
        other => panic!("not an object type: {other:?}"),
    }
}

// Found by review: upstream's TS throws these; the port panicked.

#[test]
fn fragment_named_as_an_argument_type_throws_a_type_error() {
    let result = compose(&[r#"
        fragment Foo on Query { f }
        directive @d(a: Foo) on FIELD_DEFINITION
        type Query { f: Int @d(a: 1) }
    "#]);
    assert_eq!(
        thrown(result),
        "TypeError: Cannot read properties of undefined (reading 'kind')"
    );
}

#[test]
fn variable_as_a_directive_argument_throws() {
    let mut document = parse(&format!(
        "{HEADER}directive @d(a: Int) on FIELD_DEFINITION\ntype Query {{ f: Int @d(a: 1) }}"
    ))
    .unwrap();
    field_definitions(&mut document, 2)[0].directives[0].arguments[0].value = Value::Variable(Variable {
        name: federation_composition_rs::graphql_rs::Name::new("x"),
    });
    assert_eq!(
        thrown(compose_documents(vec![document])),
        "Error: Unknown value node kind: [object Object]"
    );
}

#[test]
fn enum_default_value_that_is_not_a_name_throws_the_syntax_error() {
    let mut document = parse(&format!("{HEADER}enum E {{ A }}\ntype Query {{ f(a: E = A): Int }}")).unwrap();
    let argument = &mut field_definitions(&mut document, 2)[0].arguments[0];
    let Some(Value::Enum(value)) = &mut argument.default_value else {
        panic!("an enum default value");
    };
    value.value = "A B".into();
    assert_eq!(
        thrown(compose_documents(vec![document])),
        "Syntax Error: Expected <EOF>, found Name \"B\"."
    );
}

#[test]
fn input_field_default_value_that_is_not_a_name_throws_the_syntax_error() {
    let mut document = parse(&format!(
        "{HEADER}enum E {{ A }}\ninput I {{ e: E = A }}\ntype Query {{ f(a: I): Int }}"
    ))
    .unwrap();
    let Definition::InputObjectTypeDefinition(input) = &mut document.definitions[2] else {
        panic!("an input object");
    };
    let Some(Value::Enum(value)) = &mut input.fields[0].default_value else {
        panic!("an enum default value");
    };
    value.value = "A B".into();
    assert_eq!(
        thrown(compose_documents(vec![document])),
        "Syntax Error: Expected <EOF>, found Name \"B\"."
    );
}

#[test]
fn type_name_that_is_not_a_name_is_an_unknown_type() {
    let mut document = parse(&format!("{HEADER}type Query {{ f: Int }}")).unwrap();
    field_definitions(&mut document, 1)[0].ty = Type::Named(NamedType::new("Foo Bar"));
    let result = compose_documents(vec![document]).expect("a composition result");
    assert_eq!(messages(&result), ["[a] Unknown type Foo Bar"]);
}

#[test]
fn directive_location_that_is_not_one_matches_nothing() {
    let mut document = parse(&format!(
        "{HEADER}directive @d on FIELD_DEFINITION\ntype Query {{ f: Int @d }}"
    ))
    .unwrap();
    let Definition::DirectiveDefinition(directive) = &mut document.definitions[1] else {
        panic!("a directive definition");
    };
    directive.locations[0].value = "NOWHERE".into();
    let result = compose_documents(vec![document]).expect("a composition result");
    assert_eq!(
        messages(&result),
        ["[a] Directive \"@d\" may not be used on FIELD_DEFINITION."]
    );
}

// Nesting limits.

/// A subgraph whose field `f` has an argument of a list type `levels` deep with a default value
/// of the same depth, and whose directive argument is an object value `levels` deep.
fn nested_subgraph(levels: usize) -> String {
    let list_type = format!("{}Int{}", "[".repeat(levels), "]".repeat(levels));
    let list_value = format!("{}1{}", "[".repeat(levels), "]".repeat(levels));
    let object_value = format!("{}1{}", "{a: ".repeat(levels - 1), "}".repeat(levels - 1));
    format!(
        "{HEADER}input I {{ a: I b: Int }}\ndirective @d(a: I) on FIELD_DEFINITION\n\
         type Query {{ f(a: {list_type} = {list_value}): {list_type} @d(a: {{b: {object_value}}}) }}"
    )
}

#[test]
fn subgraphs_nested_to_the_limit_compose() {
    on_small_stack(|| {
        let result = compose(&[&nested_subgraph(MAX_NESTING_DEPTH)]).expect("a composition result");
        let success = result.as_success().unwrap_or_else(|| panic!("{:?}", messages(&result)));
        success.public_sdl().expect("the public schema");
        assert!(success
            .supergraph_sdl()
            .contains(&format!("{}1{}", "[".repeat(128), "]".repeat(128))));
    });
}

#[test]
fn subgraphs_nested_over_the_limit_are_syntax_errors() {
    let error = parse(&nested_subgraph(MAX_NESTING_DEPTH + 1)).unwrap_err();
    assert_eq!(
        error.message,
        "Syntax Error: Document nesting depth exceeds 128. Parsing aborted."
    );
}

#[test]
fn documents_built_over_the_limit_are_rejected() {
    on_small_stack(|| {
        // Parsed with a higher limit, as a document built in code could be.
        let options = ParseOptions {
            max_nesting_depth: 1000,
            ..Default::default()
        };
        let document = parse_with_options(&nested_subgraph(MAX_NESTING_DEPTH + 1), options).unwrap();
        let error = compose_documents(vec![document]).unwrap_err();
        assert_eq!(error.message(), "AST nesting depth exceeds 128.");

        // A non-null type inside a non-null type, which the parser cannot produce, is a level too.
        let mut document = parse(&format!("{HEADER}type Query {{ f: Int }}")).unwrap();
        let mut ty = Type::named("Int");
        for _ in 0..1000 {
            ty = Type::non_null(ty);
        }
        field_definitions(&mut document, 1)[0].ty = ty;
        let error = compose_documents(vec![document]).unwrap_err();
        assert_eq!(error.message(), "AST nesting depth exceeds 128.");
    });
}

#[test]
fn field_sets_nested_over_the_limit_are_composition_errors() {
    on_small_stack(|| {
        let selection = |levels: usize| format!("a {}id{}", "{ a ".repeat(levels), " }".repeat(levels));
        let subgraph = |levels: usize| {
            format!(
                "{HEADER}type Query {{ u: U }}\ntype U @key(fields: \"{}\") {{ id: ID a: U }}",
                selection(levels)
            )
        };
        // `{ ${fields} }` is parsed: one more level than the selection's own.
        let result = compose(&[&subgraph(MAX_NESTING_DEPTH - 1)]).expect("a composition result");
        assert!(result.is_success(), "{:?}", messages(&result));
        let result = compose(&[&subgraph(MAX_NESTING_DEPTH)]).expect("a composition result");
        assert_eq!(
            messages(&result),
            [format!(
                "[a] On type \"U\", for @key(fields: \"{}\"): Syntax Error: Document nesting depth exceeds 128. Parsing aborted.",
                selection(MAX_NESTING_DEPTH)
            )]
        );
    });
}

// Long chains of types, which upstream's recursive searches walk one recursion level per type.

const CHAIN: usize = 5000;

fn chain(each: impl Fn(usize) -> String, last: String, head: &str) -> String {
    let mut sdl = format!("{HEADER}{head}\n");
    for i in 0..CHAIN {
        sdl += &each(i);
    }
    sdl + &last
}

#[test]
fn long_chain_of_object_types_composes() {
    on_small_stack(|| {
        let sdl = chain(
            |i| {
                format!(
                    "type T{i} {{ id: ID @tag(name: \"public\") a: T{} @tag(name: \"public\") }}\n",
                    i + 1
                )
            },
            format!("type T{CHAIN} {{ id: ID @tag(name: \"public\") }}\n"),
            "type Query { t: T0 @tag(name: \"public\") }",
        );
        let result = compose(&[&sdl]).expect("a composition result");
        assert!(result.is_success(), "{:?}", messages(&result));
        let filter = federation_composition_rs::SchemaContractFilter::new(["public"], Vec::<String>::new());
        let services = [ServiceDefinition::new("a", parse(&sdl).unwrap())];
        let contract = federation_composition_rs::compose_schema_contract(&services, &filter, true).unwrap();
        assert!(contract.is_success(), "{:?}", messages(&contract));
    });
}

#[test]
fn long_chain_of_entities_across_subgraphs_composes() {
    on_small_stack(|| {
        // Each type's next type is resolved by the other subgraph, through the type's key.
        let (mut a, mut b) = (format!("{HEADER}type Query {{ t: T0 }}\n"), HEADER.to_string());
        for i in 0..CHAIN {
            let (owner, other) = if i % 2 == 0 { (&mut a, &mut b) } else { (&mut b, &mut a) };
            *owner += &format!("type T{i} @key(fields: \"id\") {{ id: ID! a: T{} }}\n", i + 1);
            *other += &format!("type T{i} @key(fields: \"id\") {{ id: ID! }}\n");
        }
        a += &format!("type T{CHAIN} @key(fields: \"id\") {{ id: ID! }}\n");
        b += &format!("type T{CHAIN} @key(fields: \"id\") {{ id: ID! }}\n");
        let result = compose(&[&a, &b]).expect("a composition result");
        assert!(result.is_success(), "{:?}", messages(&result));
    });
}

#[test]
fn long_chain_of_unions_composes() {
    on_small_stack(|| {
        let sdl = chain(
            |i| format!("union U{i} = T{i}\ntype T{i} {{ id: ID u: U{} }}\n", i + 1),
            format!("union U{CHAIN} = T{CHAIN}\ntype T{CHAIN} {{ id: ID }}\n"),
            "type Query { u: U0 }",
        );
        let result = compose(&[&sdl]).expect("a composition result");
        assert!(result.is_success(), "{:?}", messages(&result));
    });
}

#[test]
fn long_chain_of_non_null_input_fields_composes() {
    on_small_stack(|| {
        let sdl = chain(
            |i| format!("input I{i} {{ id: ID a: I{}! }}\n", i + 1),
            format!("input I{CHAIN} {{ id: ID }}\n"),
            "type Query { t(a: I0): Int }",
        );
        let result = compose(&[&sdl]).expect("a composition result");
        assert!(result.is_success(), "{:?}", messages(&result));
    });
}

#[test]
fn long_cycle_of_non_null_input_fields_is_reported() {
    on_small_stack(|| {
        let sdl = chain(
            |i| format!("input I{i} {{ a: I{}! }}\n", (i + 1) % CHAIN),
            String::new(),
            "type Query { t(a: I0): Int }",
        );
        let result = compose(&[&sdl]).expect("a composition result");
        let path: Vec<&str> = vec!["a"; CHAIN];
        assert_eq!(
            messages(&result),
            [format!(
                "[a] Cannot reference Input Object \"I0\" within itself through a series of non-null fields: \"{}\".",
                path.join(".")
            )]
        );
    });
}

#[test]
fn long_chain_of_input_default_values_composes() {
    on_small_stack(|| {
        let sdl = chain(
            |i| format!("input I{i} {{ id: ID a: I{} = {{}} }}\n", i + 1),
            format!("input I{CHAIN} {{ id: ID = 1 }}\n"),
            "type Query { t(a: I0 = {}): Int }",
        );
        let result = compose(&[&sdl]).expect("a composition result");
        assert!(result.is_success(), "{:?}", messages(&result));
    });
}

#[test]
fn dropping_a_hand_built_list_type_is_safe_below_the_limit() {
    // Not a library guarantee, a reminder: dropping an AST recurses too, so the documents of the
    // tests above stay far below depths that would overflow when they are dropped.
    let mut ty = Type::named("Int");
    for _ in 0..1000 {
        ty = Type::List(ListType { ty: Box::new(ty) });
    }
    let _ = Directive::new("d", vec![]);
    drop(ty);
}
