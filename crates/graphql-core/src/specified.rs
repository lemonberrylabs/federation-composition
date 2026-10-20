//! The definitions graphql-js 17.0.2 specifies: `specifiedScalarTypes`,
//! `specifiedDirectives`, `introspectionTypes` and `versionInfo`. The
//! graphql-js type system itself is not ported; directives and scalars are
//! given as AST definitions that print like graphql-js `printDirective` /
//! `printType`.

use std::sync::OnceLock;

use crate::ast::{Definition, DirectiveDefinition, ScalarTypeDefinition, Value};
use crate::error::GraphQLError;
use crate::parser::parse;
use crate::printer::print;

/// graphql-js `version`.
pub const VERSION: &str = "17.0.2";

/// graphql-js `versionInfo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionInfo {
    /// The major version.
    pub major: u32,
    /// The minor version.
    pub minor: u32,
    /// The patch version.
    pub patch: u32,
    /// The pre-release tag, `None` for a release.
    pub pre_release_tag: Option<&'static str>,
}

/// graphql-js `versionInfo` of the ported version.
pub const VERSION_INFO: VersionInfo = VersionInfo {
    major: 17,
    minor: 0,
    patch: 2,
    pre_release_tag: None,
};

/// graphql-js `GRAPHQL_MAX_INT`.
pub const GRAPHQL_MAX_INT: i32 = 2147483647;
/// graphql-js `GRAPHQL_MIN_INT`.
pub const GRAPHQL_MIN_INT: i32 = -2147483648;
/// graphql-js `DEFAULT_DEPRECATION_REASON`.
pub const DEFAULT_DEPRECATION_REASON: &str = "No longer supported";

/// The result of a scalar's `coerceInputLiteral`.
#[derive(Clone, Debug, PartialEq)]
pub enum CoercedLiteral {
    /// A JS number (`Int` and `Float`).
    Number(f64),
    /// A JS string (`String` and `ID`).
    String(String),
    /// A JS boolean (`Boolean`).
    Boolean(bool),
}

/// One of graphql-js `specifiedScalarTypes`.
///
/// Only this crate creates them (`#[non_exhaustive]`), so every value is one of the five
/// specified scalars, which [`SpecifiedScalarType::coerce_input_literal`] relies on.
#[derive(Debug)]
#[non_exhaustive]
pub struct SpecifiedScalarType {
    /// The scalar's name, e.g. `Int`.
    pub name: &'static str,
    /// The scalar's description, as graphql-js has it.
    pub description: &'static str,
}

/// graphql-js `specifiedScalarTypes`, in graphql-js order.
pub static SPECIFIED_SCALAR_TYPES: [SpecifiedScalarType; 5] = [
    SpecifiedScalarType {
        name: "String",
        description: "The `String` scalar type represents textual data, represented as UTF-8 character sequences. The String type is most often used by GraphQL to represent free-form human-readable text.",
    },
    SpecifiedScalarType {
        name: "Int",
        description: "The `Int` scalar type represents non-fractional signed whole numeric values. Int can represent values between -(2^31) and 2^31 - 1.",
    },
    SpecifiedScalarType {
        name: "Float",
        description: "The `Float` scalar type represents signed double-precision fractional values as specified by [IEEE 754](https://en.wikipedia.org/wiki/IEEE_floating_point).",
    },
    SpecifiedScalarType {
        name: "Boolean",
        description: "The `Boolean` scalar type represents `true` or `false`.",
    },
    SpecifiedScalarType {
        name: "ID",
        description: "The `ID` scalar type represents a unique identifier, often used to refetch an object or as key for a cache. The ID type appears in a JSON response as a String; however, it is not intended to be human-readable. When expected as an input type, any string (such as `\"4\"`) or integer (such as `4`) input value will be accepted as an ID.",
    },
];

/// graphql-js `specifiedScalarTypes`.
pub fn specified_scalar_types() -> &'static [SpecifiedScalarType] {
    &SPECIFIED_SCALAR_TYPES
}

/// graphql-js `isSpecifiedScalarType`, by name.
pub fn is_specified_scalar_type(name: &str) -> bool {
    SPECIFIED_SCALAR_TYPES.iter().any(|t| t.name == name)
}

/// The specified scalar named `name`.
pub fn specified_scalar_type(name: &str) -> Option<&'static SpecifiedScalarType> {
    SPECIFIED_SCALAR_TYPES.iter().find(|t| t.name == name)
}

impl SpecifiedScalarType {
    /// The scalar as an AST definition, with its description as a block
    /// string the way graphql-js `printType` prints it.
    pub fn definition(&self) -> ScalarTypeDefinition {
        ScalarTypeDefinition {
            description: Some(crate::ast::StringValue::new(self.description, true)),
            name: crate::ast::Name::new(self.name),
            directives: Vec::new(),
        }
    }

    /// The scalar's graphql-js `coerceInputLiteral`.
    pub fn coerce_input_literal(&self, value_node: &Value) -> Result<CoercedLiteral, GraphQLError> {
        match self.name {
            "Int" => {
                let Value::Int(int) = value_node else {
                    return Err(GraphQLError::new(format!(
                        "Int cannot represent non-integer value: {}",
                        print(value_node)
                    )));
                };
                let num = js_parse_int(&int.value);
                if num > GRAPHQL_MAX_INT as f64 || num < GRAPHQL_MIN_INT as f64 {
                    return Err(GraphQLError::new(format!(
                        "Int cannot represent non 32-bit signed integer value: {}",
                        int.value
                    )));
                }
                Ok(CoercedLiteral::Number(num))
            }
            "Float" => match value_node {
                Value::Float(f) => Ok(CoercedLiteral::Number(js_parse_float(&f.value))),
                Value::Int(i) => Ok(CoercedLiteral::Number(js_parse_float(&i.value))),
                _ => Err(GraphQLError::new(format!(
                    "Float cannot represent non numeric value: {}",
                    print(value_node)
                ))),
            },
            "String" => match value_node {
                Value::String(s) => Ok(CoercedLiteral::String(s.value.clone())),
                _ => Err(GraphQLError::new(format!(
                    "String cannot represent a non string value: {}",
                    print(value_node)
                ))),
            },
            "Boolean" => match value_node {
                Value::Boolean(b) => Ok(CoercedLiteral::Boolean(b.value)),
                _ => Err(GraphQLError::new(format!(
                    "Boolean cannot represent a non boolean value: {}",
                    print(value_node)
                ))),
            },
            "ID" => match value_node {
                Value::String(s) => Ok(CoercedLiteral::String(s.value.clone())),
                Value::Int(i) => Ok(CoercedLiteral::String(i.value.clone())),
                _ => Err(GraphQLError::new(format!(
                    "ID cannot represent a non-string and non-integer value: {}",
                    print(value_node)
                ))),
            },
            _ => unreachable!("a SpecifiedScalarType is one of SPECIFIED_SCALAR_TYPES"),
        }
    }
}

/// JS whitespace and line terminators, as skipped by `parseInt`/`parseFloat`.
fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{a}' | '\u{b}' | '\u{c}' | '\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// JS `parseInt(s, 10)`.
pub(crate) fn js_parse_int(s: &str) -> f64 {
    let s = s.trim_start_matches(is_js_whitespace);
    let (negative, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return f64::NAN;
    }
    let value: f64 = rest[..digits].parse().unwrap();
    if negative {
        -value
    } else {
        value
    }
}

/// JS `parseFloat(s)`.
pub(crate) fn js_parse_float(s: &str) -> f64 {
    let s = s.trim_start_matches(is_js_whitespace);
    let bytes = s.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        i = 1;
    }
    if s[i..].starts_with("Infinity") {
        return if bytes.first() == Some(&b'-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let int_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let mut mantissa_digits = i - int_start;
    if i < bytes.len() && bytes[i] == b'.' {
        let frac_start = i + 1;
        let mut j = frac_start;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if mantissa_digits > 0 || j > frac_start {
            mantissa_digits += j - frac_start;
            i = j;
        }
    }
    if mantissa_digits == 0 {
        return f64::NAN;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    let literal = &s[..i];
    // Rust does not accept a trailing "." after digits only as JS does
    // ("1." is 1); strip it.
    let literal = literal.strip_suffix('.').unwrap_or(literal);
    literal.parse().unwrap_or(f64::NAN)
}

const SPECIFIED_DIRECTIVES_SDL: &str = r#"
"""
Directs the executor to include this field or fragment only when the `if` argument is true.
"""
directive @include(
  """Included when true."""
  if: Boolean!
) on FIELD | FRAGMENT_SPREAD | INLINE_FRAGMENT

"""
Directs the executor to skip this field or fragment when the `if` argument is true.
"""
directive @skip(
  """Skipped when true."""
  if: Boolean!
) on FIELD | FRAGMENT_SPREAD | INLINE_FRAGMENT

"""Marks an element of a GraphQL schema as no longer supported."""
directive @deprecated(
  """
  Explains why this element was deprecated, usually also including a suggestion for how to access supported similar data. Formatted using the Markdown syntax, as specified by [CommonMark](https://commonmark.org/).
  """
  reason: String! = "No longer supported"
) on FIELD_DEFINITION | ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | ENUM_VALUE | DIRECTIVE_DEFINITION

"""Exposes a URL that specifies the behavior of this scalar."""
directive @specifiedBy(
  """The URL that specifies the behavior of this scalar."""
  url: String!
) on SCALAR

"""
Indicates exactly one field must be supplied and this field must not be `null`.
"""
directive @oneOf on INPUT_OBJECT
"#;

/// graphql-js `specifiedDirectives` (`@include`, `@skip`, `@deprecated`,
/// `@specifiedBy`, `@oneOf`) as directive definitions.
pub fn specified_directives() -> &'static [DirectiveDefinition] {
    static DIRECTIVES: OnceLock<Vec<DirectiveDefinition>> = OnceLock::new();
    DIRECTIVES.get_or_init(|| {
        parse(SPECIFIED_DIRECTIVES_SDL)
            .expect("specified directives parse")
            .definitions
            .into_iter()
            .map(|d| match d {
                Definition::DirectiveDefinition(d) => d,
                _ => unreachable!(),
            })
            .collect()
    })
}

/// graphql-js `isSpecifiedDirective`, by name.
pub fn is_specified_directive(name: &str) -> bool {
    specified_directives().iter().any(|d| d.name.value == name)
}

/// The names of graphql-js `introspectionTypes`, in graphql-js order.
pub const INTROSPECTION_TYPE_NAMES: [&str; 8] = [
    "__Schema",
    "__Directive",
    "__DirectiveLocation",
    "__Type",
    "__Field",
    "__InputValue",
    "__EnumValue",
    "__TypeKind",
];

/// graphql-js `isIntrospectionType`, by name.
pub fn is_introspection_type(name: &str) -> bool {
    INTROSPECTION_TYPE_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_number_parsing() {
        assert_eq!(js_parse_int("  42abc"), 42.0);
        assert!(js_parse_int("abc").is_nan());
        assert_eq!(js_parse_int("-0").to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_parse_float("1."), 1.0);
        assert_eq!(js_parse_float(".5e1x"), 5.0);
        assert_eq!(js_parse_float("1e"), 1.0);
        assert_eq!(js_parse_float("-Infinityx"), f64::NEG_INFINITY);
        assert!(js_parse_float(".").is_nan());
        assert!(js_parse_float("e5").is_nan());
    }
}
