//! Differential tests of `build_schema`, `validate_schema` and
//! `assert_valid_schema` against graphql-js 17.0.2, whose outcomes are
//! recorded in `schema-corpus.json`, built by `gen-schema-corpus.mjs` (run it
//! from the repository root with
//! `node crates/graphql-rs/tests/gen-schema-corpus.mjs`).

use std::sync::OnceLock;

use graphql_rs::__internal::jsutils::JsValue;
use graphql_rs::__internal::r#type::definition::{DefaultInput, GraphQLArgument, GraphQLNamedType};
use graphql_rs::GraphQLSchema;
use graphql_rs::{assert_valid_schema, build_schema, is_introspection_type, print, validate_schema};
use serde_json::{json, Value as Json};

fn corpus() -> &'static Json {
    static CORPUS: OnceLock<Json> = OnceLock::new();
    CORPUS.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/schema-corpus.json");
        let text = std::fs::read_to_string(path).unwrap_or_else(|_| {
            panic!(
                "{path} is missing; run `node crates/graphql-rs/tests/gen-schema-corpus.mjs` from the repository root"
            )
        });
        serde_json::from_str(&text).expect("valid JSON")
    })
}

/// `JSON.stringify` of a default given as a value.
fn js_json(value: &JsValue) -> Json {
    match value {
        JsValue::Null => Json::Null,
        JsValue::Boolean(b) => Json::Bool(*b),
        JsValue::Number(n) => json!(n),
        JsValue::String(s) => Json::String(s.clone()),
        JsValue::List(items) => Json::Array(items.iter().map(js_json).collect()),
        JsValue::Object(object) => Json::Object(object.iter().map(|(k, v)| (k.clone(), js_json(v))).collect()),
    }
}

fn opt(s: &Option<String>) -> Json {
    match s {
        Some(s) => Json::String(s.clone()),
        None => Json::Null,
    }
}

fn default_str(default: &Option<DefaultInput>) -> Json {
    match default {
        None => Json::Null,
        Some(DefaultInput::Literal(literal)) => Json::String(print(literal)),
        Some(DefaultInput::Value(value)) => Json::String(serde_json::to_string(&js_json(value)).unwrap()),
    }
}

fn arg_shape(arg: &GraphQLArgument, intro: bool) -> Json {
    json!([
        arg.name,
        arg.ty.to_string(),
        if intro { Json::Null } else { opt(&arg.description) },
        default_str(&arg.default),
        opt(&arg.deprecation_reason),
    ])
}

/// The `shape` of `gen-schema-corpus.mjs`.
fn shape(schema: &GraphQLSchema) -> Json {
    let types: Vec<Json> = schema
        .get_type_map()
        .values()
        .map(|t| {
            let intro = is_introspection_type(t.name());
            let mut o = serde_json::Map::new();
            o.insert("name".into(), json!(t.name()));
            o.insert(
                "description".into(),
                if intro { Json::Null } else { json!(t.description()) },
            );
            match t {
                GraphQLNamedType::Scalar(s) => {
                    o.insert("kind".into(), json!("scalar"));
                    o.insert("specifiedByURL".into(), opt(&s.specified_by_url));
                }
                GraphQLNamedType::Object(_) | GraphQLNamedType::Interface(_) => {
                    let (kind, interfaces, fields) = match t {
                        GraphQLNamedType::Object(t) => ("object", t.interfaces(), t.fields()),
                        GraphQLNamedType::Interface(t) => ("interface", t.interfaces(), t.fields()),
                        _ => unreachable!(),
                    };
                    o.insert("kind".into(), json!(kind));
                    o.insert("interfaces".into(), json!(interfaces));
                    let fields: Vec<Json> = fields
                        .values()
                        .map(|f| {
                            json!([
                                f.name,
                                f.ty.to_string(),
                                if intro { Json::Null } else { opt(&f.description) },
                                opt(&f.deprecation_reason),
                                f.args.iter().map(|a| arg_shape(a, intro)).collect::<Vec<_>>(),
                            ])
                        })
                        .collect();
                    o.insert("fields".into(), Json::Array(fields));
                }
                GraphQLNamedType::Union(u) => {
                    o.insert("kind".into(), json!("union"));
                    o.insert("types".into(), json!(u.types()));
                }
                GraphQLNamedType::Enum(e) => {
                    o.insert("kind".into(), json!("enum"));
                    match e.get_values() {
                        Ok(values) => {
                            let values: Vec<Json> = values
                                .iter()
                                .map(|v| {
                                    json!([
                                        v.name,
                                        if intro { Json::Null } else { opt(&v.description) },
                                        opt(&v.deprecation_reason),
                                    ])
                                })
                                .collect();
                            o.insert("values".into(), Json::Array(values));
                        }
                        Err(error) => {
                            o.insert("valuesError".into(), json!(error.message));
                        }
                    }
                }
                GraphQLNamedType::InputObject(i) => {
                    o.insert("kind".into(), json!("input"));
                    o.insert("isOneOf".into(), json!(i.is_one_of));
                    let fields: Vec<Json> = i
                        .fields()
                        .values()
                        .map(|f| {
                            json!([
                                f.name,
                                f.ty.to_string(),
                                opt(&f.description),
                                default_str(&f.default),
                                opt(&f.deprecation_reason),
                            ])
                        })
                        .collect();
                    o.insert("fields".into(), Json::Array(fields));
                }
            }
            Json::Object(o)
        })
        .collect();
    let directives: Vec<Json> = schema
        .get_directives()
        .iter()
        .map(|d| {
            json!([
                d.name,
                opt(&d.description),
                d.locations.iter().map(|l| l.as_str()).collect::<Vec<_>>(),
                d.is_repeatable,
                opt(&d.deprecation_reason),
                d.args.iter().map(|a| arg_shape(a, false)).collect::<Vec<_>>(),
            ])
        })
        .collect();
    json!({
        "description": opt(&schema.description),
        "query": schema.get_query_type().map(|t| t.name()),
        "mutation": schema.get_mutation_type().map(|t| t.name()),
        "subscription": schema.get_subscription_type().map(|t| t.name()),
        "types": types,
        "directives": directives,
    })
}

fn first_difference(expected: &Json, actual: &Json, path: &str) -> String {
    match (expected, actual) {
        (Json::Object(e), Json::Object(a)) => {
            for (k, ev) in e {
                match a.get(k) {
                    Some(av) if av == ev => {}
                    Some(av) => return first_difference(ev, av, &format!("{path}.{k}")),
                    None => return format!("{path}.{k}: missing"),
                }
            }
            format!("{path}: extra keys")
        }
        (Json::Array(e), Json::Array(a)) => {
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                if ev != av {
                    return first_difference(ev, av, &format!("{path}[{i}]"));
                }
            }
            format!("{path}: lengths {} vs {}", e.len(), a.len())
        }
        _ => format!("{path}: expected {expected}, got {actual}"),
    }
}

#[test]
fn build_and_validate_schemas_like_graphql_js() {
    let cases = corpus()["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut counts = [0usize; 3];
    for case in cases {
        let src = case["src"].as_str().unwrap();
        let sdl = case["sdl"].as_str().unwrap();
        let label = || format!("{src}: {}", sdl.chars().take(120).collect::<String>());
        let schema = match build_schema(sdl) {
            Ok(schema) => schema,
            Err(error) => {
                counts[0] += 1;
                if case["buildError"].as_str() != Some(error.message.as_str()) {
                    failures.push(format!(
                        "{}\n  build: expected {}, got error {:?}",
                        label(),
                        case["buildError"],
                        error.message
                    ));
                }
                continue;
            }
        };
        if !case["buildError"].is_null() {
            failures.push(format!(
                "{}\n  build: expected error {}, got a schema",
                label(),
                case["buildError"]
            ));
            continue;
        }
        let actual_shape = shape(&schema);
        if actual_shape != case["shape"] {
            failures.push(format!(
                "{}\n  shape: {}",
                label(),
                first_difference(&case["shape"], &actual_shape, "")
            ));
        }
        match validate_schema(&schema) {
            Ok(errors) => {
                let messages: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
                if case["validateErrors"] != json!(messages) {
                    failures.push(format!(
                        "{}\n  validateSchema: expected {} {}, got {:?}",
                        label(),
                        case["validateErrors"],
                        case["validateThrows"],
                        messages
                    ));
                }
            }
            Err(error) => {
                if case["validateThrows"].as_str() != Some(error.message.as_str()) {
                    failures.push(format!(
                        "{}\n  validateSchema: expected {} {}, got throw {:?}",
                        label(),
                        case["validateErrors"],
                        case["validateThrows"],
                        error.message
                    ));
                }
            }
        }
        let asserted = assert_valid_schema(&schema);
        if asserted.is_err() {
            counts[1] += 1;
        } else {
            counts[2] += 1;
        }
        if json!(asserted.err().map(|error| error.message)) != case["assertError"] {
            failures.push(format!(
                "{}\n  assertValidSchema: expected {}, got {:?}",
                label(),
                case["assertError"],
                assert_valid_schema(&schema)
            ));
        }
    }
    eprintln!(
        "{} schemas: {} fail to build, {} invalid, {} valid; {} differ from graphql-js",
        cases.len(),
        counts[0],
        counts[1],
        counts[2],
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {} cases differ from graphql-js:\n{}",
        failures.len(),
        cases.len(),
        failures.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
    );
}
