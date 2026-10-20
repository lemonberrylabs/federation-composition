//! `graphql_rs::build_schema` and schema validation on arbitrary SDL.
#![no_main]

use graphql_rs::{assert_valid_schema, build_schema, validate_schema};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(schema) = build_schema(source) else {
        return;
    };
    let _ = validate_schema(&schema);
    let _ = assert_valid_schema(&schema);
});
