//! Native counterpart of `bench/run-corpus.ts`: times `compose_services` on every case of
//! the conformance corpus (`conformance/out/corpus.jsonl`, see tests/corpus.rs) with the same
//! protocol and writes results in the same format.
//!
//! `cargo run -p federation-composition-rs --release --example corpus_bench -- bench/results-guild-rust.json`
//! (`pnpm bench:native`), or with mimalloc as the global allocator:
//! `cargo run -p federation-composition-rs --release --features mimalloc --example corpus_bench -- bench/results-guild-rust-mimalloc.json`
//! (`pnpm bench:native:mimalloc`).

// Opt in with `--features mimalloc`. The library never sets a global allocator; final artifacts do.
#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use federation_composition_rs::compose::{compose_services, CompositionResult};
use federation_composition_rs::subgraph::validation::validate_subgraph::InternalOptions;
use federation_composition_rs::types::ServiceDefinition;
use graphql_rs::Document;
use serde::Deserialize;
use serde_json::json;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct Case {
    id: String,
    source: String,
    services: Vec<Service>,
    internal: Option<Internal>,
    success: bool,
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

// Same as bench/run-corpus.ts.
const WARMUP_RUNS: usize = 3;
const MIN_RUNS: usize = 5;
const MIN_TIME: Duration = Duration::from_millis(50);
const MAX_RUNS: usize = 200;

const LABEL: &str = if cfg!(feature = "mimalloc") {
    "guild-rust-mimalloc"
} else {
    "guild-rust"
};

fn main() {
    let out_file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("bench/results-{LABEL}.json"));
    // The conformance corpus (see tests/corpus.rs).
    let corpus_path = std::env::var("CONFORMANCE_CORPUS")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/out/corpus.jsonl").to_string());
    let corpus = std::fs::read_to_string(&corpus_path)
        .unwrap_or_else(|e| panic!("reading {corpus_path}: {e}; run `pnpm conformance --only=capture`"));

    let started = Instant::now();
    let mut results = Vec::new();
    for line in corpus.lines().filter(|l| !l.is_empty()) {
        let case: Case = serde_json::from_str(line).expect("corpus line deserializes");
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
        let sdl_bytes: usize = services.iter().map(|s| graphql_rs::print(&s.type_defs).len()).sum();

        let run = || compose_services(black_box(&services), internal.as_ref()).expect("composition does not throw");
        let success = matches!(run(), CompositionResult::Success(_));
        assert_eq!(success, case.success, "{}: success differs from TypeScript", case.id);
        for _ in 0..WARMUP_RUNS {
            black_box(run());
        }

        let mut samples = Vec::new();
        let case_start = Instant::now();
        while samples.len() < MIN_RUNS || (case_start.elapsed() < MIN_TIME && samples.len() < MAX_RUNS) {
            let t = Instant::now();
            black_box(run());
            samples.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        results.push(json!({
            "id": case.id,
            "source": case.source,
            "subgraphs": services.len(),
            "sdlBytes": sdl_bytes,
            "success": success,
            "runs": samples.len(),
            "medianMs": samples[samples.len() / 2],
            "minMs": samples[0],
        }));
    }

    let wall_ms = started.elapsed().as_secs_f64() * 1000.0;
    std::fs::write(
        &out_file,
        serde_json::to_string_pretty(&json!({ "label": LABEL, "node": null, "wallMs": wall_ms, "results": results }))
            .unwrap(),
    )
    .unwrap();
    println!(
        "{LABEL}: {} cases in {:.1}s -> {out_file}",
        results.len(),
        wall_ms / 1000.0
    );
}
