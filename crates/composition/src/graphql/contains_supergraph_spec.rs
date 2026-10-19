//! Port of `src/graphql/contains-supergraph-spec.ts`.

use std::sync::LazyLock;

use graphql_js::Kind;

use super::supergraph_spec::{
    get_supergraph_spec_nodes, EXTRA_FEDERATION_DIRECTIVE_NAMES, EXTRA_FEDERATION_TYPE_NAMES,
};

/// The alternatives of the cached `containsSupergraphSpecRegex`, split by their first atom.
struct Patterns {
    /// `\[NAME` and `\sNAME`.
    bracket_or_space: Vec<String>,
    /// `@NAME`.
    at: Vec<String>,
}

static PATTERNS: LazyLock<Patterns> = LazyLock::new(|| {
    let mut patterns = Patterns {
        bracket_or_space: Vec::new(),
        at: Vec::new(),
    };

    for node in get_supergraph_spec_nodes() {
        // NOTE(port): the TS compares with `Kind.DIRECTIVE` ("Directive"), which no definition
        // node has (directives are `DirectiveDefinition`), so every spec node, directives
        // included, gets the `[NAME`/`\sNAME` patterns. Kept as is.
        if node.kind == Kind::Directive {
            // "@NAME" for directives
            patterns.at.push(node.name.clone());
        } else {
            // "[NAME" or " NAME" for scalars, enums and inputs
            patterns.bracket_or_space.push(node.name.clone());
        }
    }

    for name in EXTRA_FEDERATION_TYPE_NAMES.iter() {
        patterns.bracket_or_space.push(name.to_string());
    }

    for name in EXTRA_FEDERATION_DIRECTIVE_NAMES.iter() {
        patterns.at.push(name.to_string());
    }

    patterns
});

/// `String.prototype.trim`: strips the same code points as `\s`.
pub(crate) fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// JS regex `\s`: WhiteSpace and LineTerminator code points.
pub(crate) fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{b}'
            | '\u{c}'
            | '\r'
            | ' '
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

/// `containsSupergraphSpec(sdl)`: whether the SDL mentions any supergraph spec directive
/// (`@NAME`) or type (`[NAME` or whitespace + `NAME`).
/// NOTE(port): the TS builds one cached regex `\[NAME|\sNAME|@NAME|...` (names are not
/// escaped; they contain only `[A-Za-z_]`). `test` is true when any alternative matches
/// anywhere, which is what the scan below checks.
pub fn contains_supergraph_spec(sdl: &str) -> bool {
    let patterns = &*PATTERNS;
    for (i, c) in sdl.char_indices() {
        let names = if c == '[' || is_js_whitespace(c) {
            &patterns.bracket_or_space
        } else if c == '@' {
            &patterns.at
        } else {
            continue;
        };
        let rest = &sdl[i + c.len_utf8()..];
        if names.iter().any(|name| rest.starts_with(name.as_str())) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Expected values generated with the TS function.
    fn detects_spec_names() {
        assert!(!contains_supergraph_spec("type A @join__type(graph: A) { a: Int }"));
        assert!(contains_supergraph_spec("scalar\u{3000}join__FieldSet"));
        assert!(!contains_supergraph_spec("scalar\u{85}join__FieldSet"));
        assert!(contains_supergraph_spec("f: [_FieldSet]"));
        assert!(contains_supergraph_spec("schema @core(feature: \"x\")"));
        assert!(contains_supergraph_spec("schema @join__owner_x"));
        // Directive names match after whitespace or `[`, not after `@` (see the NOTE).
        assert!(!contains_supergraph_spec("type A @tag(name: \"x\") { a: Int }"));
        assert!(contains_supergraph_spec("directive @x on FIELD tag"));
        assert!(!contains_supergraph_spec("type Query { a: Int }"));
    }
}
