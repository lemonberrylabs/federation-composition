//! Checks `utils::js` against values recorded from Node in `fixtures/js-semantics.json`.

use federation_composition_rs::utils::js::{locale_compare, number_to_string};
use serde_json::Value;
use std::cmp::Ordering;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/js-semantics.json")).unwrap()
}

#[test]
fn locale_compare_matches_node() {
    let fixture = fixture();
    let pairs = fixture["localeCompare"].as_array().unwrap();
    let mut mismatches = Vec::new();
    for pair in pairs {
        let a = pair[0].as_str().unwrap();
        let b = pair[1].as_str().unwrap();
        let expected = match pair[2].as_i64().unwrap() {
            -1 => Ordering::Less,
            0 => Ordering::Equal,
            _ => Ordering::Greater,
        };
        if locale_compare(a, b) != expected {
            mismatches.push(format!("{a:?} vs {b:?}: node {expected:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} pairs differ:\n{}",
        mismatches.len(),
        pairs.len(),
        mismatches[..mismatches.len().min(20)].join("\n")
    );
}

#[test]
fn number_to_string_matches_node() {
    for case in fixture()["numberToString"].as_array().unwrap() {
        let n = match &case[0] {
            Value::String(s) => match s.as_str() {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                "-0" => -0.0,
                other => panic!("unexpected {other}"),
            },
            v => v.as_f64().unwrap(),
        };
        assert_eq!(number_to_string(n), case[1].as_str().unwrap(), "String({n:?})");
    }
}
