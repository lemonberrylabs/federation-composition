//! graphql-js `type/assertName`.

use crate::error::GraphQLError;

/// graphql-js `isNameStart` (`language/characterClasses`).
fn is_name_start(code: u16) -> bool {
    u8::try_from(code).is_ok_and(|c| c.is_ascii_alphabetic() || c == b'_')
}

/// graphql-js `isNameContinue`.
fn is_name_continue(code: u16) -> bool {
    u8::try_from(code).is_ok_and(|c| c.is_ascii_alphanumeric() || c == b'_')
}

/// graphql-js `assertName`.
pub fn assert_name(name: &str) -> Result<&str, GraphQLError> {
    let units: Vec<u16> = name.encode_utf16().collect();
    if units.is_empty() {
        return Err(GraphQLError::new("Expected name to be a non-empty string."));
    }
    for &unit in &units[1..] {
        if !is_name_continue(unit) {
            return Err(GraphQLError::new(format!(
                "Names must only contain [_a-zA-Z0-9] but \"{name}\" does not."
            )));
        }
    }
    if !is_name_start(units[0]) {
        return Err(GraphQLError::new(format!(
            "Names must start with [_a-zA-Z] but \"{name}\" does not."
        )));
    }
    Ok(name)
}

/// graphql-js `assertEnumValueName`.
pub fn assert_enum_value_name(name: &str) -> Result<&str, GraphQLError> {
    if name == "true" || name == "false" || name == "null" {
        return Err(GraphQLError::new(format!("Enum values cannot be named: {name}")));
    }
    assert_name(name)
}
