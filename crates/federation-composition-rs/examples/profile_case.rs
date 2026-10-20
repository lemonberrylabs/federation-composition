//! Composes one corpus case in a loop so a sampling profiler can attach (see `bench/profile.sh`).
//!
//! `cargo run -p federation-composition-rs --release --example profile_case -- <case-id> <seconds>`

// Opt in with `--features mimalloc`. The library never sets a global allocator; final artifacts do.
#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use federation_composition_rs::compose::compose_services;
use federation_composition_rs::subgraph::validation::validate_subgraph::InternalOptions;
use federation_composition_rs::types::ServiceDefinition;
use graphql_rs::Document;
use serde_json::Value;
use std::time::{Duration, Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "fixture:huge-schema".to_string());
    let seconds: u64 = args.next().map(|s| s.parse().expect("seconds")).unwrap_or(20);

    let corpus = std::fs::read_to_string("bench/corpus.jsonl").expect("bench/corpus.jsonl is missing: run `pnpm bench:capture`");
    let case: Value = corpus
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).expect("corpus line is JSON"))
        .find(|c| c["id"] == id.as_str())
        .unwrap_or_else(|| panic!("no corpus case {id}"));
    let services: Vec<ServiceDefinition> = case["services"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| ServiceDefinition {
            name: s["name"].as_str().unwrap().to_string(),
            url: s["url"].as_str().map(str::to_string),
            type_defs: serde_json::from_value::<Document>(s["typeDefs"].clone()).unwrap(),
        })
        .collect();
    let internal = case["internal"]["disableValidationRules"].as_array().map(|rules| InternalOptions {
        disable_validation_rules: Some(rules.iter().map(|r| r.as_str().unwrap().to_string()).collect()),
    });

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut runs = 0u64;
    while Instant::now() < deadline {
        std::hint::black_box(compose_services(&services, internal.as_ref()).expect("composition does not throw"));
        runs += 1;
    }
    eprintln!("{runs} compositions in {seconds}s");
}
