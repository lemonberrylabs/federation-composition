//! Differential check against the TypeScript implementation: replays every composition the
//! TypeScript test suite performs (`bench/corpus.jsonl`, recorded with `pnpm bench:capture`)
//! and compares the Rust output with the recorded TypeScript output.
//!
//! Ignored by default because the corpus is generated, not committed:
//! `pnpm bench:capture && cargo test -p federation_composition --release --test corpus -- --ignored`
//! Writes every mismatch to `target/corpus-mismatches.jsonl`.

use federation_composition::compose::{compose_services, CompositionResult};
use federation_composition::subgraph::validation::validate_subgraph::InternalOptions;
use federation_composition::types::ServiceDefinition;
use graphql_js::Document;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Deserialize)]
struct Case {
    id: String,
    services: Vec<Service>,
    internal: Option<Internal>,
    expected: Value,
}

#[derive(Deserialize)]
struct Service {
    name: String,
    url: Option<String>,
    #[serde(rename = "typeDefs")]
    type_defs: Document,
}

#[derive(Deserialize)]
struct Internal {
    #[serde(rename = "disableValidationRules")]
    disable_validation_rules: Option<Vec<String>>,
}

/// The Rust output in the shape the capture records for TypeScript.
fn run(case: &Case) -> Value {
    let services: Vec<ServiceDefinition> = case
        .services
        .iter()
        .map(|s| ServiceDefinition { name: s.name.clone(), url: s.url.clone(), type_defs: s.type_defs.clone() })
        .collect();
    let internal = case.internal.as_ref().map(|i| InternalOptions { disable_validation_rules: i.disable_validation_rules.clone() });
    match compose_services(&services, internal.as_ref()) {
        Err(thrown) => json!({ "thrown": thrown.to_js_string() }),
        Ok(CompositionResult::Failure(failure)) => json!({
            "errors": failure.errors.iter().map(|e| json!({ "message": e.message, "code": e.code() })).collect::<Vec<_>>()
        }),
        Ok(CompositionResult::Success(success)) => json!({
            "supergraphSdl": success.supergraph_sdl(),
            "publicSdl": match success.public_sdl() { Ok(sdl) => json!(sdl), Err(e) => json!({ "thrown": e.to_js_string() }) },
        }),
    }
}

#[test]
#[ignore = "needs bench/corpus.jsonl from `pnpm bench:capture`"]
fn rust_matches_typescript_on_test_suite_compositions() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/corpus.jsonl");
    let corpus = std::fs::read_to_string(path).expect("bench/corpus.jsonl is missing: run `pnpm bench:capture`");
    let out_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/corpus-mismatches.jsonl");
    let mut out = std::fs::File::create(out_path).unwrap();

    std::panic::set_hook(Box::new(|_| {}));
    let (mut total, mut matched, mut panicked) = (0, 0, 0);
    for line in corpus.lines().filter(|l| !l.is_empty()) {
        let case: Case = serde_json::from_str(line).expect("corpus line deserializes");
        total += 1;
        let actual = match catch_unwind(AssertUnwindSafe(|| run(&case))) {
            Ok(actual) => actual,
            Err(payload) => {
                panicked += 1;
                let message = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                json!({ "panic": message })
            }
        };
        if actual == case.expected {
            matched += 1;
        } else {
            writeln!(out, "{}", json!({ "id": case.id, "expected": case.expected, "actual": actual })).unwrap();
        }
    }
    let _ = std::panic::take_hook();
    println!("{matched}/{total} compositions match TypeScript ({panicked} panicked); mismatches in target/corpus-mismatches.jsonl");
    assert_eq!(matched, total, "{} of {total} compositions differ from TypeScript", total - matched);
}
