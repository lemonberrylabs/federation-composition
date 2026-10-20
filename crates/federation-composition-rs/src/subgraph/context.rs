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
    let chars: Vec<char> = input.chars().collect();
    let mut pos = skip_ignored_tokens(&chars, 0);

    if chars.get(pos) != Some(&'$') {
        return ContextReference::default();
    }

    pos = skip_ignored_tokens(&chars, pos + 1);

    if !is_name_start(chars.get(pos).copied()) {
        return ContextReference::default();
    }

    let context_start = pos;
    pos += 1;

    while is_name_continue(chars.get(pos).copied()) {
        pos += 1;
    }

    ContextReference {
        context: Some(chars[context_start..pos].iter().collect()),
        selection: Some(chars[pos..].iter().collect()),
    }
}

/// `isValidContextName(name)`.
pub fn is_valid_context_name(name: &str) -> bool {
    let mut chars = name.chars();
    if !is_ascii_letter(chars.next()) {
        return false;
    }

    for c in chars {
        if !is_ascii_letter(Some(c)) && !is_digit(Some(c)) {
            return false;
        }
    }

    true
}

/// `skipIgnoredTokens(input, start)`: whitespace, commas, BOM and comments.
fn skip_ignored_tokens(input: &[char], start: usize) -> usize {
    let mut pos = start;

    while pos < input.len() {
        let c = input[pos];

        // '\u{feff}' is the byte order mark
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == ',' || c == '\u{feff}' {
            pos += 1;
            continue;
        }

        if c == '#' {
            pos += 1;

            while pos < input.len() && input[pos] != '\n' && input[pos] != '\r' {
                pos += 1;
            }

            continue;
        }

        break;
    }

    pos
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(context: &str, selection: &str) -> ContextReference {
        ContextReference {
            context: Some(context.to_string()),
            selection: Some(selection.to_string()),
        }
    }

    #[test]
    fn parses_references() {
        assert_eq!(
            parse_context_reference("$userCtx { id }"),
            reference("userCtx", " { id }")
        );
        assert_eq!(
            parse_context_reference(" ,# c\n\u{feff}$ \t_ctx1{ id }"),
            reference("_ctx1", "{ id }")
        );
        assert_eq!(parse_context_reference("$ctx"), reference("ctx", ""));
        assert_eq!(parse_context_reference("ctx { id }"), ContextReference::default());
        assert_eq!(parse_context_reference("$1ctx"), ContextReference::default());
        assert_eq!(parse_context_reference(""), ContextReference::default());
        assert_eq!(parse_context_reference("$é"), ContextReference::default());
    }

    #[test]
    fn context_names() {
        assert!(is_valid_context_name("ctx1"));
        assert!(!is_valid_context_name(""));
        assert!(!is_valid_context_name("1ctx"));
        assert!(!is_valid_context_name("ctx_1"));
        assert!(!is_valid_context_name("ctxé"));
    }
}
