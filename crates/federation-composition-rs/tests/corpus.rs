//! Differential check against upstream's TypeScript implementation: replays every composition
//! upstream's test suite performs (the corpus, recorded by the capture step of
//! `pnpm conformance`) and compares the Rust output with the recorded TypeScript output.
//!
//! Ignored by default because the corpus is generated, not committed:
//! `pnpm conformance --only=capture && cargo test -p federation-composition-rs --release --test corpus -- --ignored`
//! (`pnpm conformance` runs both). The corpus is `conformance/out/corpus.jsonl`, or the file
//! named by `CONFORMANCE_CORPUS`. Compositions listed under `compositions` in
//! `conformance/known-exceptions.json` (or the file named by `CONFORMANCE_EXCEPTIONS`) may
//! differ; one that matches, or is not in the corpus, fails the test so the list stays current.
//! Writes every mismatch to `target/corpus-mismatches.jsonl` and the counts to
//! `target/corpus-summary.json`.

use federation_composition_rs::compose::{compose_services, CompositionResult};
use federation_composition_rs::subgraph::validation::validate_subgraph::InternalOptions;
use federation_composition_rs::types::ServiceDefinition;
use graphql_rs::Document;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
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
        .map(|s| ServiceDefinition {
            name: s.name.clone(),
            url: s.url.clone(),
            type_defs: s.type_defs.clone(),
        })
        .collect();
    let internal = case.internal.as_ref().map(|i| InternalOptions {
        disable_validation_rules: i.disable_validation_rules.clone(),
    });
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

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The ids of the compositions that may differ from TypeScript (known-exceptions.json).
fn excepted_ids() -> BTreeSet<String> {
    let path =
        std::env::var("CONFORMANCE_EXCEPTIONS").unwrap_or_else(|_| format!("{ROOT}/conformance/known-exceptions.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
    let json: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    json["compositions"]
        .as_array()
        .unwrap_or_else(|| panic!("{path}: \"compositions\" must be an array"))
        .iter()
        .map(|entry| {
            assert!(
                entry["reason"].as_str().is_some_and(|r| !r.is_empty()),
                "{path}: every entry needs a \"reason\": {entry}"
            );
            entry["id"]
                .as_str()
                .unwrap_or_else(|| panic!("{path}: an entry has no \"id\": {entry}"))
                .to_string()
        })
        .collect()
}

#[test]
#[ignore = "needs the corpus from `pnpm conformance --only=capture`"]
fn rust_matches_typescript_on_test_suite_compositions() {
    let path = std::env::var("CONFORMANCE_CORPUS").unwrap_or_else(|_| format!("{ROOT}/conformance/out/corpus.jsonl"));
    let corpus = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {path}: {e}; run `pnpm conformance --only=capture`"));
    let mut exceptions = excepted_ids();
    let mut out = std::fs::File::create(format!("{ROOT}/target/corpus-mismatches.jsonl")).unwrap();

    std::panic::set_hook(Box::new(|_| {}));
    let (mut total, mut matched, mut excepted, mut panicked) = (0, 0, 0, 0);
    let mut stale = Vec::new();
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
        let is_exception = exceptions.remove(&case.id);
        if actual == case.expected {
            matched += 1;
            if is_exception {
                stale.push(case.id.clone());
            }
        } else if is_exception {
            excepted += 1;
        } else {
            writeln!(
                out,
                "{}",
                json!({ "id": case.id, "expected": case.expected, "actual": actual })
            )
            .unwrap();
        }
    }
    let _ = std::panic::take_hook();
    let unknown: Vec<String> = exceptions.into_iter().collect();
    std::fs::write(
        format!("{ROOT}/target/corpus-summary.json"),
        json!({
            "total": total,
            "matched": matched,
            "excepted": excepted,
            "panicked": panicked,
            "stale": stale,
            "unknown": unknown,
        })
        .to_string(),
    )
    .unwrap();
    println!(
        "{matched}/{total} compositions match TypeScript ({excepted} known exceptions, {panicked} panicked); mismatches in target/corpus-mismatches.jsonl"
    );
    assert_eq!(
        matched + excepted,
        total,
        "{} of {total} compositions differ from TypeScript",
        total - matched - excepted
    );
    assert!(
        stale.is_empty(),
        "known exceptions that match TypeScript (remove them): {stale:?}"
    );
    assert!(unknown.is_empty(), "known exceptions not in the corpus: {unknown:?}");
}
