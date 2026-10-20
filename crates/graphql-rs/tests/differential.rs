//! Differential tests against graphql-js 17.0.2. The expected values are in
//! `corpus.json`, built by `gen-corpus.mjs` (run it from the repository root
//! with `node crates/graphql-rs/tests/gen-corpus.mjs`).

use std::sync::OnceLock;

use graphql_rs::specified::{specified_scalar_types, CoercedLiteral, INTROSPECTION_TYPE_NAMES};
use graphql_rs::*;
use serde_json::Value as Json;

fn corpus() -> &'static Json {
    static CORPUS: OnceLock<Json> = OnceLock::new();
    CORPUS.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus.json");
        let text = std::fs::read_to_string(path).unwrap_or_else(|_| {
            panic!("{path} is missing; run `node crates/graphql-rs/tests/gen-corpus.mjs` from the repository root")
        });
        serde_json::from_str(&text).unwrap()
    })
}

fn s(v: &Json) -> &str {
    v.as_str().unwrap()
}

/// The source text of a corpus document.
fn doc_text(doc: &Json) -> String {
    if let Some(text) = doc.get("text") {
        return s(text).to_owned();
    }
    let derived = &doc["derived"];
    let rich = s(&corpus()["richDocs"][derived["of"].as_u64().unwrap() as usize]);
    if let Some(bytes) = derived.get("bytes") {
        return rich[..bytes.as_u64().unwrap() as usize].to_owned();
    }
    let at = derived["at"].as_u64().unwrap() as usize;
    match s(&derived["op"]) {
        "delete" => format!("{}{}", &rich[..at], &rich[at + 1..]),
        "double" => format!("{}{}", &rich[..at + 1], &rich[at..]),
        op => panic!("unknown op {op}"),
    }
}

fn label(doc: &Json, text: &str) -> String {
    let mut t: String = text.chars().take(80).collect();
    if t.len() < text.len() {
        t.push_str("...");
    }
    format!("[{}] {:?}", s(&doc["src"]), t)
}

fn check(failures: &mut Vec<String>, ok: bool, what: impl FnOnce() -> String) {
    if !ok {
        failures.push(what());
    }
}

fn assert_no_failures(name: &str, total: usize, failures: Vec<String>) {
    if !failures.is_empty() {
        let shown: Vec<_> = failures.iter().take(15).cloned().collect();
        panic!(
            "{name}: {} of {total} checks failed. First failures:\n{}",
            failures.len(),
            shown.join("\n\n")
        );
    }
    eprintln!("{name}: {total} cases passed");
}

const OPTIONAL_LISTS: &[&str] = &[
    "directives",
    "arguments",
    "variableDefinitions",
    "interfaces",
    "fields",
    "values",
    "types",
    "operationTypes",
];
const OPTIONAL_NODES: &[&str] = &[
    "description",
    "alias",
    "name",
    "selectionSet",
    "defaultValue",
    "typeCondition",
];

#[derive(Clone, Copy)]
enum Variant {
    /// Missing optional lists written as `[]`.
    EmptyLists,
    /// Missing optional fields written as `null`, plus `loc` and an unknown key.
    Nulls,
    /// `kind` moved to the end of every object.
    KindLast,
}

fn mutate(v: &Json, variant: Variant) -> Json {
    match v {
        Json::Array(items) => Json::Array(items.iter().map(|i| mutate(i, variant)).collect()),
        Json::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                if matches!(variant, Variant::KindLast) && k == "kind" {
                    continue;
                }
                out.insert(k.clone(), mutate(val, variant));
            }
            if map.contains_key("kind") {
                match variant {
                    Variant::EmptyLists => {
                        for key in OPTIONAL_LISTS {
                            out.entry(*key).or_insert_with(|| Json::Array(Vec::new()));
                        }
                    }
                    Variant::Nulls => {
                        for key in OPTIONAL_LISTS.iter().chain(OPTIONAL_NODES) {
                            out.entry(*key).or_insert(Json::Null);
                        }
                        out.insert("loc".into(), serde_json::json!({ "start": 0, "end": 1 }));
                        out.insert("someUnknownKey".into(), serde_json::json!([1, { "x": null }]));
                    }
                    Variant::KindLast => {
                        out.insert("kind".into(), map["kind"].clone());
                    }
                }
            }
            Json::Object(out)
        }
        other => other.clone(),
    }
}

#[test]
fn documents() {
    let mut failures = Vec::new();
    let docs = corpus()["documents"].as_array().unwrap();
    let mut checks = 0;
    for doc in docs {
        let text = doc_text(doc);
        let l = || label(doc, &text);
        checks += 1;

        // stripIgnoredCharacters
        match (
            strip_ignored_characters(&text),
            doc.get("stripped"),
            doc.get("strippedError"),
        ) {
            (Ok(out), Some(expected), None) => check(&mut failures, out == s(expected), || {
                format!("{}: strip\n  ours: {out:?}\n  js:   {:?}", l(), s(expected))
            }),
            (Err(e), None, Some(expected)) => check(&mut failures, e.message == s(expected), || {
                format!("{}: strip error\n  ours: {}\n  js:   {}", l(), e.message, s(expected))
            }),
            (ours, _, _) => failures.push(format!("{}: strip mismatch, ours: {ours:?}", l())),
        }

        // parse
        let parsed = parse(&text);
        match (&parsed, doc.get("json"), doc.get("error")) {
            (Ok(ast), Some(json), None) => {
                let ours = serde_json::to_string(ast).unwrap();
                check(&mut failures, ours == s(json), || {
                    format!("{}: JSON\n  ours: {ours}\n  js:   {}", l(), s(json))
                });
                let printed = print(ast);
                let expected_print = s(&doc["printed"]);
                check(&mut failures, printed == expected_print, || {
                    format!("{}: print\n  ours: {printed:?}\n  js:   {expected_print:?}", l())
                });
                let js_value: Json = serde_json::from_str(s(json)).unwrap();
                let from_js: Document = serde_json::from_value(js_value.clone()).unwrap();
                check(&mut failures, &from_js == ast, || format!("{}: JSON deserialize", l()));
                check(&mut failures, print(&from_js) == expected_print, || {
                    format!("{}: print of deserialized JSON", l())
                });
                let from_str: Document = serde_json::from_str(s(json)).unwrap();
                check(&mut failures, &from_str == ast, || format!("{}: JSON from_str", l()));
                for variant in [Variant::EmptyLists, Variant::Nulls, Variant::KindLast] {
                    let mutated = serde_json::to_string(&mutate(&js_value, variant)).unwrap();
                    match serde_json::from_str::<Document>(&mutated) {
                        Ok(d) => check(&mut failures, &d == ast, || {
                            format!("{}: JSON variant mismatch {mutated}", l())
                        }),
                        Err(e) => failures.push(format!("{}: JSON variant error {e}: {mutated}", l())),
                    }
                }
            }
            (Err(e), None, Some(expected)) => check(&mut failures, e.message == s(expected), || {
                format!("{}: parse error\n  ours: {}\n  js:   {}", l(), e.message, s(expected))
            }),
            (ours, _, expected) => failures.push(format!(
                "{}: parse outcome differs\n  ours: {:?}\n  js error: {expected:?}",
                l(),
                ours.as_ref().map(|_| "ok").map_err(|e| e.message.clone())
            )),
        }

        // experimentalFragmentArguments
        let with_args = parse_with_options(
            &text,
            ParseOptions {
                experimental_fragment_arguments: true,
                ..Default::default()
            },
        );
        let expected_json = doc.get("fragmentArgumentsJson").or(doc.get("json"));
        let expected_error = doc.get("fragmentArgumentsError").or(doc.get("error"));
        match &with_args {
            Ok(ast) => {
                let ours = serde_json::to_string(ast).unwrap();
                let printed = print(ast);
                let expected_printed = doc.get("fragmentArgumentsPrinted").or(doc.get("printed"));
                let json_ok = expected_json.is_some_and(|j| s(j) == ours)
                    && (doc.get("fragmentArgumentsError").is_none() || doc.get("json").is_some());
                check(&mut failures, json_ok, || {
                    format!(
                        "{}: fragment arguments JSON\n  ours: {ours}\n  js: {expected_json:?} / {expected_error:?}",
                        l()
                    )
                });
                check(&mut failures, expected_printed.is_some_and(|p| s(p) == printed), || {
                    format!(
                        "{}: fragment arguments print\n  ours: {printed:?}\n  js: {expected_printed:?}",
                        l()
                    )
                });
            }
            Err(e) => {
                let ok =
                    doc.get("fragmentArgumentsJson").is_none() && expected_error.is_some_and(|x| s(x) == e.message);
                check(&mut failures, ok, || {
                    format!(
                        "{}: fragment arguments error\n  ours: {}\n  js: {expected_error:?}",
                        l(),
                        e.message
                    )
                });
            }
        }
    }
    assert_no_failures("documents", checks, failures);
}

#[test]
fn max_tokens() {
    let mut failures = Vec::new();
    let cases = corpus()["maxTokens"].as_array().unwrap();
    for case in cases {
        let text = s(&case["text"]);
        let limit = case["maxTokens"].as_u64().unwrap() as usize;
        let result = parse_with_options(
            text,
            ParseOptions {
                max_tokens: Some(limit),
                ..Default::default()
            },
        );
        let ours = match &result {
            Ok(ast) => Ok(serde_json::to_string(ast).unwrap()),
            Err(e) => Err(e.message.clone()),
        };
        let js = match case.get("json") {
            Some(j) => Ok(s(j).to_owned()),
            None => Err(s(&case["error"]).to_owned()),
        };
        check(&mut failures, ours == js, || {
            format!("maxTokens {limit} {text:?}\n  ours: {ours:?}\n  js:   {js:?}")
        });
    }
    assert_no_failures("max_tokens", cases.len(), failures);
}

fn compare_parse<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    failures: &mut Vec<String>,
    case: &Json,
    what: &str,
    result: Result<T, GraphQLError>,
    print_fn: impl Fn(&T) -> String,
) {
    let text = s(&case["text"]);
    match (result, case.get("json")) {
        (Ok(v), Some(json)) => {
            let ours = serde_json::to_string(&v).unwrap();
            check(failures, ours == s(json), || {
                format!("{what} {text:?}: JSON\n  ours: {ours}\n  js:   {}", s(json))
            });
            let printed = print_fn(&v);
            check(failures, printed == s(&case["printed"]), || {
                format!("{what} {text:?}: print {printed:?} vs {:?}", case["printed"])
            });
            let back: T = serde_json::from_str(s(json)).unwrap();
            check(failures, back == v, || format!("{what} {text:?}: deserialize"));
        }
        (Err(e), None) => check(failures, e.message == s(&case["error"]), || {
            format!(
                "{what} {text:?}: error\n  ours: {}\n  js:   {}",
                e.message, case["error"]
            )
        }),
        (ours, _) => failures.push(format!("{what} {text:?}: outcome differs, ours {ours:?}, js {case}")),
    }
}

#[test]
fn values_types_and_coordinates() {
    let mut failures = Vec::new();
    let mut total = 0;
    for case in corpus()["values"].as_array().unwrap() {
        total += 1;
        let text = s(&case["text"]);
        let result = if s(&case["mode"]) == "value" {
            parse_value(text)
        } else {
            parse_const_value(text)
        };
        compare_parse(&mut failures, case, s(&case["mode"]), result, |v| print(v));
    }
    for case in corpus()["types"].as_array().unwrap() {
        total += 1;
        compare_parse(&mut failures, case, "type", parse_type(s(&case["text"])), |v| print(v));
    }
    for case in corpus()["coordinates"].as_array().unwrap() {
        total += 1;
        compare_parse(
            &mut failures,
            case,
            "coordinate",
            parse_schema_coordinate(s(&case["text"])),
            |v| print(v),
        );
    }
    assert_no_failures("values_types_and_coordinates", total, failures);
}

#[test]
fn print_hand_built_asts() {
    let mut failures = Vec::new();
    let cases = corpus()["printAsts"].as_array().unwrap();
    for case in cases {
        let node: Node = serde_json::from_str(s(&case["json"])).unwrap();
        let printed = print(&node);
        check(&mut failures, printed == s(&case["printed"]), || {
            format!(
                "{}\n  ours: {printed:?}\n  js:   {:?}",
                s(&case["json"]),
                case["printed"]
            )
        });
    }
    assert_no_failures("print_hand_built_asts", cases.len(), failures);
}

fn fnv(text: &str) -> String {
    let mut h: u32 = 0x811c9dc5;
    for b in text.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    format!("{h:x}")
}

fn path_str(path: &[PathKey]) -> String {
    path.iter()
        .map(|k| match k {
            PathKey::Key(k) => (*k).to_owned(),
            PathKey::Index(i) => i.to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn ancestor_desc(a: Option<Ancestor<'_>>) -> String {
    match a {
        None => "-".to_owned(),
        Some(Ancestor::Node(n)) => n.kind().as_str().to_owned(),
        Some(Ancestor::List(l)) => format!("[{}]", l.len()),
    }
}

fn trace_line(phase: &str, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> String {
    let key = match info.key {
        None => "-".to_owned(),
        Some(PathKey::Key(k)) => k.to_owned(),
        Some(PathKey::Index(i)) => i.to_string(),
    };
    format!(
        "{phase}|{}|{key}|{}|{}|{}\n",
        node.kind(),
        ancestor_desc(info.parent),
        path_str(info.path),
        info.ancestors
            .iter()
            .map(|a| ancestor_desc(Some(*a)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

struct Tracer;

impl<'a> Visitor<'a, String> for Tracer {
    fn enter(&mut self, node: NodeRef<'a>, out: &mut String, info: &VisitInfo<'a, '_>) -> VisitAction {
        out.push_str(&trace_line("enter", node, info));
        VisitAction::Continue
    }
    fn leave(&mut self, node: NodeRef<'a>, out: &mut String, info: &VisitInfo<'a, '_>) -> VisitAction {
        out.push_str(&trace_line("leave", node, info));
        VisitAction::Continue
    }
}

fn corpus_doc(index: &Json) -> Document {
    let doc = &corpus()["documents"][index.as_u64().unwrap() as usize];
    serde_json::from_str(s(&doc["json"])).unwrap()
}

#[test]
fn traversal_order() {
    let mut failures = Vec::new();
    let cases = corpus()["traces"].as_array().unwrap();
    for case in cases {
        let doc = corpus_doc(&case["doc"]);
        let mut out = String::new();
        let broke = visit(&doc, &mut Tracer, &mut out);
        assert!(!broke);
        // The editing traversal must visit identically when nothing is edited.
        let mut edit_out = String::new();
        let result = visit_edit(&doc, &mut EditTracer(&mut edit_out));
        check(
            &mut failures,
            result == EditResult::Node(Node::Document(doc.clone())),
            || "edit visit without edits changed the document".to_owned(),
        );
        check(&mut failures, edit_out == out, || "edit traversal differs".to_owned());
        match case.get("trace") {
            Some(expected) => check(&mut failures, out == s(expected), || {
                let diff = out
                    .lines()
                    .zip(s(expected).lines())
                    .find(|(a, b)| a != b)
                    .map(|(a, b)| format!("ours {a}\n  js   {b}"))
                    .unwrap_or_else(|| "length differs".to_owned());
                format!("trace of doc {}: {diff}", case["doc"])
            }),
            None => check(&mut failures, fnv(&out) == s(&case["hash"]), || {
                format!("trace hash of doc {}", case["doc"])
            }),
        }
    }
    assert_no_failures("traversal_order", cases.len(), failures);
}

struct EditTracer<'o>(&'o mut String);

impl EditVisitor for EditTracer<'_> {
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        self.0.push_str(&trace_line("enter", node, info));
        EditAction::Continue
    }
    fn leave(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        self.0.push_str(&trace_line("leave", node, info));
        EditAction::Continue
    }
}

// Editing scenarios; each mirrors the function of the same name in
// gen-corpus.mjs.

fn name_len(node: NodeRef<'_>) -> usize {
    node.name().map_or(0, |n| n.value.encode_utf16().count())
}

/// A visitor made of two closures.
struct FnEdit<E, L> {
    enter: E,
    leave: L,
}

impl<E, L> EditVisitor for FnEdit<E, L>
where
    E: FnMut(NodeRef<'_>, &VisitInfo<'_, '_>) -> EditAction,
    L: FnMut(NodeRef<'_>, &VisitInfo<'_, '_>) -> EditAction,
{
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        (self.enter)(node, info)
    }
    fn leave(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        (self.leave)(node, info)
    }
}

fn fn_edit<E, L>(enter: E, leave: L) -> FnEdit<E, L>
where
    E: FnMut(NodeRef<'_>, &VisitInfo<'_, '_>) -> EditAction,
    L: FnMut(NodeRef<'_>, &VisitInfo<'_, '_>) -> EditAction,
{
    FnEdit { enter, leave }
}

fn cont(_: NodeRef<'_>, _: &VisitInfo<'_, '_>) -> EditAction {
    EditAction::Continue
}

fn run_scenario(name: &str, doc: &Document) -> (EditResult, Vec<String>) {
    let mut log: Vec<String> = Vec::new();
    let result = match name {
        "deleteDirectives" => visit_edit(
            doc,
            &mut fn_edit(
                |node, _| match node {
                    NodeRef::Directive(d) => {
                        log.push(format!("Directive {}", d.name.value));
                        if d.name.value.len() % 2 == 0 {
                            EditAction::Delete
                        } else {
                            EditAction::Continue
                        }
                    }
                    _ => EditAction::Continue,
                },
                cont,
            ),
        ),
        "replaceOnEnter" => visit_edit(
            doc,
            &mut fn_edit(
                |node, _| match node {
                    NodeRef::FieldDefinition(f) => EditAction::Replace(
                        FieldDefinition {
                            name: Name::new(format!("{}_x", f.name.value)),
                            ..f.clone()
                        }
                        .into(),
                    ),
                    NodeRef::Name(n) => {
                        log.push(n.value.clone());
                        EditAction::Continue
                    }
                    _ => EditAction::Continue,
                },
                cont,
            ),
        ),
        "leaveReplace" => visit_edit(
            doc,
            &mut fn_edit(
                |node, _| match node {
                    NodeRef::Directive(d) if d.name.value.len() % 2 == 0 => EditAction::Delete,
                    _ => EditAction::Continue,
                },
                |node, _| match node {
                    NodeRef::ObjectTypeDefinition(o) => {
                        log.push(format!("{}:{}", o.name.value, o.directives.len()));
                        let mut o = o.clone();
                        o.fields.reverse();
                        EditAction::Replace(o.into())
                    }
                    _ => EditAction::Continue,
                },
            ),
        ),
        "skipOnEnter" => {
            let log = std::cell::RefCell::new(&mut log);
            visit_edit(
                doc,
                &mut fn_edit(
                    |node, _| {
                        log.borrow_mut().push(format!("enter {}", node.kind()));
                        if name_len(node) % 2 == 1 && node.kind() != Kind::Name {
                            EditAction::Skip
                        } else {
                            EditAction::Continue
                        }
                    },
                    |node, _| {
                        log.borrow_mut().push(format!("leave {}", node.kind()));
                        EditAction::Continue
                    },
                ),
            )
        }
        "leaveSkipDropsEdits" => visit_edit(
            doc,
            &mut fn_edit(
                |node, _| match node {
                    NodeRef::Directive(_) => EditAction::Delete,
                    _ => EditAction::Continue,
                },
                |node, _| match node {
                    NodeRef::InputValueDefinition(i) => {
                        log.push(format!("{}:{}", i.name.value, i.directives.len()));
                        EditAction::Skip
                    }
                    NodeRef::FieldDefinition(f) => {
                        log.push(format!("field {}", f.name.value));
                        EditAction::Continue
                    }
                    _ => EditAction::Continue,
                },
            ),
        ),
        "deleteOptional" => visit_edit(
            doc,
            &mut fn_edit(
                |node, info| match node {
                    NodeRef::StringValue(_) if info.key == Some(PathKey::Key("description")) => EditAction::Delete,
                    NodeRef::SelectionSet(_) if matches!(info.parent, Some(Ancestor::Node(NodeRef::Field(_)))) => {
                        EditAction::Delete
                    }
                    NodeRef::NamedType(n)
                        if info.key == Some(PathKey::Key("typeCondition"))
                            && matches!(info.parent, Some(Ancestor::Node(NodeRef::InlineFragment(_))))
                            && n.name.value.len() % 2 == 0 =>
                    {
                        EditAction::Delete
                    }
                    NodeRef::VariableDefinition(v) if v.default_value.is_some() => EditAction::Replace(
                        VariableDefinition {
                            default_value: None,
                            ..v.clone()
                        }
                        .into(),
                    ),
                    _ => EditAction::Continue,
                },
                |node, _| {
                    // The JS visitor's generic `leave` is not used for kinds
                    // that have their own entry.
                    if !matches!(
                        node.kind(),
                        Kind::StringValue | Kind::SelectionSet | Kind::NamedType | Kind::VariableDefinition
                    ) {
                        log.push(node.kind().to_string());
                    }
                    EditAction::Continue
                },
            ),
        ),
        "replaceRoot" => visit_edit(
            doc,
            &mut fn_edit(
                |node, _| match node {
                    NodeRef::Document(d) => EditAction::Replace(
                        Document {
                            definitions: d.definitions[1..].to_vec(),
                        }
                        .into(),
                    ),
                    NodeRef::Name(n) => {
                        log.push(n.value.clone());
                        EditAction::Continue
                    }
                    _ => EditAction::Continue,
                },
                cont,
            ),
        ),
        "replaceThenDeleteOnLeave" => {
            let log = std::cell::RefCell::new(&mut log);
            visit_edit(
                doc,
                &mut fn_edit(
                    |node, _| match node {
                        NodeRef::EnumValueDefinition(e) => EditAction::Replace(
                            EnumValueDefinition {
                                name: Name::new(e.name.value.to_lowercase()),
                                ..e.clone()
                            }
                            .into(),
                        ),
                        NodeRef::Argument(a) => EditAction::Replace(
                            Argument {
                                value: Value::string("replaced"),
                                ..a.clone()
                            }
                            .into(),
                        ),
                        NodeRef::Name(n) => {
                            log.borrow_mut().push(format!("name {}", n.value));
                            EditAction::Continue
                        }
                        _ => EditAction::Continue,
                    },
                    |node, _| match node {
                        NodeRef::EnumValueDefinition(e) => {
                            log.borrow_mut().push(e.name.value.clone());
                            if e.name.value.len() % 2 == 0 {
                                EditAction::Delete
                            } else {
                                EditAction::Continue
                            }
                        }
                        _ => EditAction::Continue,
                    },
                ),
            )
        }
        "parallel" => {
            let log = std::cell::RefCell::new(Vec::new());
            let result = {
                let logger = |id: &'static str| {
                    let log = &log;
                    move |phase: &str, node: NodeRef<'_>, info: &VisitInfo<'_, '_>| {
                        log.borrow_mut()
                            .push(format!("{id} {phase} {} {}", node.kind(), path_str(info.path)));
                    }
                };
                let a = logger("A");
                let a2 = logger("A");
                let b = logger("B");
                let b2 = logger("B");
                let c = logger("C");
                let c2 = logger("C");
                let d = logger("D");
                let d2 = logger("D");
                let mut parallel = visit_in_parallel_edit(vec![
                    Box::new(fn_edit(
                        move |node, info| {
                            a("enter", node, info);
                            if matches!(node.kind(), Kind::ObjectTypeDefinition | Kind::Field) {
                                EditAction::Skip
                            } else {
                                EditAction::Continue
                            }
                        },
                        move |node, info| {
                            a2("leave", node, info);
                            EditAction::Continue
                        },
                    )),
                    Box::new(fn_edit(
                        move |node, info| {
                            b("enter", node, info);
                            match node {
                                NodeRef::Directive(d) if d.name.value.len() % 2 == 0 => EditAction::Delete,
                                _ => EditAction::Continue,
                            }
                        },
                        move |node, info| {
                            b2("leave", node, info);
                            EditAction::Continue
                        },
                    )),
                    Box::new(fn_edit(
                        move |node, info| {
                            c("enter", node, info);
                            match node {
                                NodeRef::EnumValueDefinition(e) => EditAction::Replace(
                                    EnumValueDefinition {
                                        name: Name::new("REPLACED"),
                                        ..e.clone()
                                    }
                                    .into(),
                                ),
                                _ => EditAction::Continue,
                            }
                        },
                        move |node, info| {
                            c2("leave", node, info);
                            match node {
                                NodeRef::InputValueDefinition(i) if i.name.value.len() % 2 == 1 => EditAction::Delete,
                                _ => EditAction::Continue,
                            }
                        },
                    )),
                    Box::new(fn_edit(
                        move |node, info| {
                            d("enter", node, info);
                            EditAction::Continue
                        },
                        move |node, info| {
                            d2("leave", node, info);
                            match node {
                                NodeRef::FieldDefinition(f) if f.name.value.starts_with('w') => EditAction::Break,
                                _ => EditAction::Continue,
                            }
                        },
                    )),
                ]);
                visit_edit(doc, &mut parallel)
            };
            return (result, log.into_inner());
        }
        "parallelReadOnly" => {
            struct Logged<F> {
                id: &'static str,
                enter: F,
                leave_break: Option<Kind>,
            }
            impl<'a, F: FnMut(NodeRef<'a>) -> VisitAction> Visitor<'a, Vec<String>> for Logged<F> {
                fn enter(&mut self, node: NodeRef<'a>, log: &mut Vec<String>, info: &VisitInfo<'a, '_>) -> VisitAction {
                    log.push(format!("{} enter {} {}", self.id, node.kind(), path_str(info.path)));
                    (self.enter)(node)
                }
                fn leave(&mut self, node: NodeRef<'a>, log: &mut Vec<String>, info: &VisitInfo<'a, '_>) -> VisitAction {
                    log.push(format!("{} leave {} {}", self.id, node.kind(), path_str(info.path)));
                    if Some(node.kind()) == self.leave_break {
                        VisitAction::Break
                    } else {
                        VisitAction::Continue
                    }
                }
            }
            let mut parallel = visit_in_parallel::<Vec<String>>(vec![
                Box::new(Logged {
                    id: "A",
                    enter: |node: NodeRef<'_>| {
                        if matches!(node.kind(), Kind::ObjectTypeDefinition | Kind::SelectionSet) {
                            VisitAction::Skip
                        } else {
                            VisitAction::Continue
                        }
                    },
                    leave_break: None,
                }),
                Box::new(Logged {
                    id: "B",
                    enter: |node: NodeRef<'_>| match node {
                        NodeRef::Directive(d) if d.name.value == "tag" => VisitAction::Break,
                        _ => VisitAction::Continue,
                    },
                    leave_break: None,
                }),
                Box::new(Logged {
                    id: "C",
                    enter: |_: NodeRef<'_>| VisitAction::Continue,
                    leave_break: Some(Kind::FieldDefinition),
                }),
                Box::new(Logged {
                    id: "D",
                    enter: |_: NodeRef<'_>| VisitAction::Continue,
                    leave_break: None,
                }),
            ]);
            let broke = visit(doc, &mut parallel, &mut log);
            assert!(!broke, "the merged visitor never breaks");
            EditResult::Node(Node::Document(doc.clone()))
        }
        "breakAfterListEdit" => visit_edit(
            doc,
            &mut fn_edit(
                |node, info| match node {
                    NodeRef::Directive(d) => EditAction::Replace(
                        Directive {
                            name: Name::new(format!("{}2", d.name.value)),
                            ..d.clone()
                        }
                        .into(),
                    ),
                    NodeRef::SelectionSet(_) => match info.parent {
                        Some(Ancestor::Node(NodeRef::Field(f))) if !f.directives.is_empty() => {
                            log.push(f.name.value.clone());
                            EditAction::Break
                        }
                        _ => EditAction::Continue,
                    },
                    _ => EditAction::Continue,
                },
                cont,
            ),
        ),
        "breakAfterDelete" => {
            // The parent list's identity, by its path.
            let mut deleted_in: Option<String> = None;
            visit_edit(
                doc,
                &mut fn_edit(
                    |node, info| {
                        if !matches!(node, NodeRef::Directive(_)) {
                            return EditAction::Continue;
                        }
                        let parent_path = path_str(&info.path[..info.path.len() - 1]);
                        if deleted_in.as_deref() == Some(parent_path.as_str()) {
                            log.push(path_str(info.path));
                            return EditAction::Break;
                        }
                        match info.parent {
                            Some(Ancestor::List(l)) if l.len() >= 2 => {
                                deleted_in = Some(parent_path);
                                EditAction::Delete
                            }
                            _ => EditAction::Continue,
                        }
                    },
                    cont,
                ),
            )
        }
        name if name.starts_with("readOnlyBreak") => {
            struct Counter {
                count: usize,
                limit: usize,
            }
            impl<'a> Visitor<'a, Vec<String>> for Counter {
                fn enter(&mut self, node: NodeRef<'a>, log: &mut Vec<String>, info: &VisitInfo<'a, '_>) -> VisitAction {
                    self.count += 1;
                    let key = match info.key {
                        None => "undefined".to_owned(),
                        Some(PathKey::Key(k)) => k.to_owned(),
                        Some(PathKey::Index(i)) => i.to_string(),
                    };
                    log.push(format!("enter {} {key}", node.kind()));
                    if self.count == self.limit {
                        VisitAction::Break
                    } else if self.count.is_multiple_of(7) {
                        VisitAction::Skip
                    } else {
                        VisitAction::Continue
                    }
                }
                fn leave(&mut self, node: NodeRef<'a>, log: &mut Vec<String>, _: &VisitInfo<'a, '_>) -> VisitAction {
                    self.count += 1;
                    log.push(format!("leave {}", node.kind()));
                    if self.count == self.limit {
                        VisitAction::Break
                    } else {
                        VisitAction::Continue
                    }
                }
            }
            let limit = name["readOnlyBreak".len()..].parse().unwrap();
            let broke = visit(doc, &mut Counter { count: 0, limit }, &mut log);
            assert_eq!(broke, log.len() == limit);
            EditResult::Node(Node::Document(doc.clone()))
        }
        name if name.starts_with("break") => {
            let n: usize = name["break".len()..].parse().unwrap();
            let count = std::cell::Cell::new(0usize);
            let result = visit_edit(
                doc,
                &mut fn_edit(
                    |node, _| {
                        count.set(count.get() + 1);
                        let c = count.get();
                        if c == n {
                            return EditAction::Break;
                        }
                        match node {
                            NodeRef::Directive(_) if c.is_multiple_of(3) => EditAction::Delete,
                            NodeRef::Name(nm) if c.is_multiple_of(4) => {
                                EditAction::Replace(Name::new(format!("{}!", nm.value)).into())
                            }
                            _ => EditAction::Continue,
                        }
                    },
                    |node, _| {
                        count.set(count.get() + 1);
                        let c = count.get();
                        if c == n {
                            return EditAction::Break;
                        }
                        match node {
                            NodeRef::Argument(_) if c.is_multiple_of(2) => EditAction::Delete,
                            _ => EditAction::Continue,
                        }
                    },
                ),
            );
            log.push(count.get().to_string());
            result
        }
        other => panic!("unknown scenario {other}"),
    };
    (result, log)
}

fn result_json(result: &EditResult) -> String {
    match result {
        EditResult::Node(n) => serde_json::to_string(n).unwrap(),
        EditResult::Deleted => "null".to_owned(),
        EditResult::List(l) => serde_json::to_string(l).unwrap(),
    }
}

#[test]
fn editing_visits() {
    let mut failures = Vec::new();
    let cases = corpus()["edits"].as_array().unwrap();
    for case in cases {
        let scenario = s(&case["scenario"]);
        let doc = corpus_doc(&case["doc"]);
        let (result, log) = run_scenario(scenario, &doc);
        let json = result_json(&result);
        let what = || format!("scenario {scenario} on doc {}", case["doc"]);
        match case.get("result") {
            Some(expected) => {
                check(&mut failures, json == s(expected), || {
                    format!("{}: result\n  ours: {json}\n  js:   {}", what(), s(expected))
                });
                let expected_log: Vec<String> = serde_json::from_value(case["log"].clone()).unwrap();
                check(&mut failures, log == expected_log, || {
                    let diff = log
                        .iter()
                        .zip(&expected_log)
                        .position(|(a, b)| a != b)
                        .map(|i| format!("first difference at {i}: ours {:?} js {:?}", log[i], expected_log[i]))
                        .unwrap_or_else(|| format!("lengths {} vs {}", log.len(), expected_log.len()));
                    format!("{}: log {diff}", what())
                });
            }
            None => {
                check(&mut failures, fnv(&json) == s(&case["hash"]), || {
                    format!("{}: result hash", what())
                });
                check(&mut failures, fnv(&log.join("\n")) == s(&case["logHash"]), || {
                    format!("{}: log hash", what())
                });
            }
        }
    }
    assert_no_failures("editing_visits", cases.len(), failures);
}

#[test]
fn specified_definitions() {
    let spec = &corpus()["specified"];
    let directives: Vec<String> = graphql_rs::specified_directives().iter().map(print).collect();
    let expected: Vec<String> = serde_json::from_value(spec["directives"].clone()).unwrap();
    assert_eq!(directives, expected);
    let scalars: Vec<String> = specified_scalar_types()
        .iter()
        .map(|t| print(&t.definition()))
        .collect();
    let expected: Vec<String> = serde_json::from_value(spec["scalars"].clone()).unwrap();
    assert_eq!(scalars, expected);
    let names: Vec<&str> = specified_scalar_types().iter().map(|t| t.name).collect();
    assert_eq!(serde_json::to_value(names).unwrap(), spec["scalarNames"]);
    assert_eq!(
        serde_json::to_value(INTROSPECTION_TYPE_NAMES).unwrap(),
        spec["introspectionTypes"]
    );
    assert_eq!(VERSION_INFO.major as u64, spec["version"]["major"].as_u64().unwrap());
    assert_eq!(VERSION_INFO.minor as u64, spec["version"]["minor"].as_u64().unwrap());
    assert_eq!(VERSION_INFO.patch as u64, spec["version"]["patch"].as_u64().unwrap());
    assert!(spec["version"]["preReleaseTag"].is_null() && VERSION_INFO.pre_release_tag.is_none());
    for d in graphql_rs::specified_directives() {
        assert!(is_specified_directive(&d.name.value));
    }
    assert!(!is_specified_directive("defer"));
    assert!(is_specified_scalar_type("ID") && !is_specified_scalar_type("Date"));
    assert!(is_introspection_type("__Type") && !is_introspection_type("Type"));
}

#[test]
fn scalar_literal_coercion() {
    let mut failures = Vec::new();
    let cases = corpus()["coercion"].as_array().unwrap();
    for case in cases {
        let scalar = graphql_rs::specified::specified_scalar_type(s(&case["scalar"])).unwrap();
        let value = parse_value(s(&case["text"])).unwrap();
        let ours = scalar.coerce_input_literal(&value);
        let ok = match (&ours, case.get("result"), case.get("error")) {
            (Ok(CoercedLiteral::Number(n)), Some(Json::Number(e)), None) => *n == e.as_f64().unwrap(),
            (Ok(CoercedLiteral::Number(n)), Some(Json::String(e)), None) => match e.as_str() {
                "-0" => *n == 0.0 && n.is_sign_negative(),
                "NaN" => n.is_nan(),
                "Infinity" => *n == f64::INFINITY,
                "-Infinity" => *n == f64::NEG_INFINITY,
                _ => false,
            },
            (Ok(CoercedLiteral::String(v)), Some(Json::String(e)), None) => v == e,
            (Ok(CoercedLiteral::Boolean(v)), Some(Json::Bool(e)), None) => v == e,
            (Err(err), None, Some(e)) => err.message == s(e),
            _ => false,
        };
        check(&mut failures, ok, || format!("{case}: ours {ours:?}"));
    }
    assert_no_failures("scalar_literal_coercion", cases.len(), failures);
}
