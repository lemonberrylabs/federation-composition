//! Port of `src/subgraph/context.ts`: parsing of `@fromContext(field:)` values.
//! NOTE(port): the TS indexes strings by UTF-16 code unit; the input is SDL text, so index by
//! `char` (or bytes, checking only ASCII) and slice on char boundaries.

/// The result of `parseContextReference`: both `None` is `notAReference`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextReference {
    pub context: Option<String>,
    pub selection: Option<String>,
}

/// `parseContextReference(input)`: `$userCtx { id }` gives
/// `{ context: "userCtx", selection: " { id }" }`.
pub fn parse_context_reference(input: &str) -> ContextReference {
    todo!()
}

/// `isValidContextName(name)`.
pub fn is_valid_context_name(name: &str) -> bool {
    todo!()
}

/// `skipIgnoredTokens(input, start)`: whitespace, commas, BOM and comments.
fn skip_ignored_tokens(input: &[char], start: usize) -> usize {
    todo!()
}

fn is_name_start(c: Option<char>) -> bool {
    c == Some('_') || is_ascii_letter(c)
}

fn is_name_continue(c: Option<char>) -> bool {
    is_name_start(c) || is_digit(c)
}

fn is_ascii_letter(c: Option<char>) -> bool {
    matches!(c, Some(c) if c.is_ascii_alphabetic())
}

fn is_digit(c: Option<char>) -> bool {
    matches!(c, Some(c) if c.is_ascii_digit())
}
