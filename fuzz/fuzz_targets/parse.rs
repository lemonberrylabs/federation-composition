//! `graphql_core::parse` and the other parsing entry points on arbitrary text, and the round-trip
//! property of the printer: printing a parsed document and parsing the result gives the same AST.
#![no_main]

use graphql_core::{
    check_nesting_depth, parse, parse_const_value, parse_schema_coordinate, parse_type, parse_value, print,
    strip_ignored_characters, Document, MAX_NESTING_DEPTH,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let _ = parse_value(source);
    let _ = parse_const_value(source);
    let _ = parse_type(source);
    let _ = parse_schema_coordinate(source);
    let _ = strip_ignored_characters(source);
    let Ok(document) = parse(source) else {
        return;
    };
    check_nesting_depth(&document, MAX_NESTING_DEPTH).expect("a parsed document is within the nesting limit");
    assert_round_trip(&document);
});

fn assert_round_trip(document: &Document) {
    let printed = print(document);
    let reparsed = match parse(&printed) {
        Ok(reparsed) => reparsed,
        Err(e) => panic!("the printed document does not parse: {e}\n--- printed:\n{printed}"),
    };
    assert_eq!(
        *document, reparsed,
        "printing and reparsing changed the AST\n--- printed:\n{printed}"
    );
}
