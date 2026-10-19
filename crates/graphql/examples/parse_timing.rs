//! Parses the corpus documents graphql-js times in `gen-corpus.mjs` and
//! reports milliseconds per pass, next to graphql-js's figure.
//!
//! cargo run --release -p graphql --example parse_timing

use std::time::Instant;

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus.json");
    let corpus: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path)
            .expect("run `node crates/graphql/tests/gen-corpus.mjs` first"),
    )
    .unwrap();
    // The same selection as gen-corpus.mjs: documents that parse, except the
    // generated prefixes and mutations.
    let docs: Vec<&str> = corpus["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d.get("json").is_some() && d.get("text").is_some())
        .filter(|d| !matches!(d["src"].as_str(), Some("prefix") | Some("mutation")))
        .map(|d| d["text"].as_str().unwrap())
        .collect();
    let bytes: usize = docs.iter().map(|d| d.len()).sum();
    for _ in 0..20 {
        for d in &docs {
            graphql::parse(d).unwrap();
        }
    }
    let iterations = 200;
    let start = Instant::now();
    for _ in 0..iterations {
        for d in &docs {
            std::hint::black_box(graphql::parse(std::hint::black_box(d)).unwrap());
        }
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    let js = &corpus["timing"];
    println!(
        "Rust parse: {ms:.3} ms per pass over {} documents ({:.1} MB); graphql-js: {:.2} ms per pass over {} documents",
        docs.len(),
        bytes as f64 / 1e6,
        js["jsParseMs"].as_f64().unwrap(),
        js["documents"]
    );
}
