//! Port of `src/specifications/link.ts`.

use graphql_js::{
    Argument, ConstArgument, ConstDirective, Directive, ListValue, Name, ObjectField, ObjectValue, StringValue, Value,
};
use crate::utils::collections::IndexMap;

use crate::graphql::printer::print;
use crate::utils::js::{parse_float, JsError, JsResult};
use crate::utils::link::link_url::whatwg_url;

/// `LinkImport`: one entry of `@link(import:)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LinkImport {
    pub kind: LinkImportKind,
    /// `@`-prefixed for directives.
    pub name: String,
    /// `as`, absent when not aliased.
    pub alias: Option<String>,
}

/// `"directive" | "type"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkImportKind {
    Directive,
    Type,
}

impl LinkImportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkImportKind::Directive => "directive",
            LinkImportKind::Type => "type",
        }
    }
}

/// `Link = ReturnType<typeof parseLink>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub name: Option<String>,
    /// NOTE(port): `mergeLinks` produces `""` (not `null`) when no version was seen; that is
    /// `Some("")` here. Both are falsy where the TS tests them.
    pub version: Option<String>,
    pub identity: String,
    pub imports: Vec<LinkImport>,
}

/// The result of `parseLinkUrl`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkUrl {
    pub name: Option<String>,
    pub version: Option<String>,
    pub identity: String,
}

fn string_value(value: &str) -> Value {
    Value::String(StringValue {
        value: value.to_string(),
        block: false,
    })
}

/// `printLink(link)`: the `@link(url:, import:)` directive printed with the library printer.
pub fn print_link(link: &Link) -> String {
    // `[link.identity, link.version].filter(Boolean).join("/")`
    let url: Vec<&str> = [Some(link.identity.as_str()), link.version.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect();
    let imports: Vec<Value> = link
        .imports
        .iter()
        .map(|im| match im.alias.as_deref() {
            Some(alias) if !alias.is_empty() => Value::Object(ObjectValue {
                fields: vec![
                    ObjectField {
                        name: Name::new("name"),
                        value: string_value(&im.name),
                    },
                    ObjectField {
                        name: Name::new("as"),
                        value: string_value(alias),
                    },
                ],
            }),
            _ => string_value(&im.name),
        })
        .collect();
    print(&Directive {
        name: Name::new("link"),
        arguments: vec![
            Argument {
                name: Name::new("url"),
                value: string_value(&url.join("/")),
            },
            Argument {
                name: Name::new("import"),
                value: Value::List(ListValue { values: imports }),
            },
        ],
    })
}

/// `/^v\d+/i.test(s)`.
fn starts_with_version(s: &str) -> bool {
    let bytes = s.as_bytes();
    matches!(bytes.first(), Some(b'v' | b'V')) && bytes.get(1).is_some_and(u8::is_ascii_digit)
}

/*

  https://example.com/exampleSchemaname/v1.0/?key=val&k2=v2#frag
                                            |------------------| ignored
                                       |----| version
                      |----------------| name
  |------------------------------------| identity

  If `name` is present, that `namespace` prefix will automatically be linked to the URL.
  If a `name` is not present, then elements of the foreign schema must be imported in order to be referenced.
*/
/// `parseLinkUrl(urlString)`. A string `new URL` rejects is an opaque identity.
pub fn parse_link_url(url_string: &str) -> LinkUrl {
    let url = match whatwg_url::Url::parse(url_string) {
        Ok(url) => url,
        Err(_) => {
            // if url: is not a valid RFC 3986 url, then it MUST be treated as an opaque identifier for the foreign schema
            return LinkUrl {
                name: None,
                version: None,
                identity: url_string.to_string(),
            };
        }
    };

    let parts: Vec<&str> = url.pathname.split('/').filter(|p| !p.is_empty()).collect();
    let len = parts.len();

    if len == 0 {
        return LinkUrl {
            name: None,
            version: None,
            identity: url.origin,
        };
    }

    let last = parts[len - 1];
    let second_last = if len >= 2 { Some(parts[len - 2]) } else { None };

    // https://spec.example.com/mySchema/v0.1
    // https://spec.example.com/mySchema/not-a-version
    let potentially_name_and_version = second_last.is_some();
    let last_is_version = starts_with_version(last);

    if let (Some(second_last), true) = (second_last, potentially_name_and_version && last_is_version) {
        let mut identity_parts: Vec<&str> = parts[..len - 2].to_vec();
        identity_parts.push(second_last);
        return LinkUrl {
            name: Some(second_last.to_string()),
            version: Some(last.to_string()),
            identity: format!("{}/{}", url.origin, identity_parts.join("/")),
        };
    }

    if last_is_version {
        return LinkUrl {
            name: None,
            version: Some(last.to_string()),
            identity: url.origin,
        };
    }

    // node returns file protocol origins as null.
    let origin = if url.protocol == "file:" {
        "file://".to_string()
    } else {
        url.origin
    };
    let mut identity_parts: Vec<&str> = parts[..len - 1].to_vec();
    identity_parts.push(last);
    LinkUrl {
        name: Some(last.to_string()),
        version: None,
        identity: format!("{}/{}", origin, identity_parts.join("/")),
    }
}

/// `parseLinkImport(importString)`: parses the printed `import:` value with JSON5.
/// Throws `Invalid import binding: ${importString}: ${String(error)}`.
pub fn parse_link_import(import_string: &str) -> JsResult<Vec<LinkImport>> {
    parse_link_import_bindings(import_string).map_err(|error| {
        JsError::error(format!("Invalid import binding: {}: {}", import_string, error.to_js_string()))
    })
}

/// The strings of `text` when it is exactly `[]` or `["a", "b", ...]` as graphql-js prints a
/// list of strings, and no string contains `"`, `\`, a line feed or a carriage return (the
/// characters a JSON5 double-quoted string treats specially). `None` for anything else.
fn plain_string_list(text: &str) -> Option<impl Iterator<Item = &str>> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    if inner.is_empty() {
        return Some(None.into_iter().flatten());
    }
    let inner = inner.strip_prefix('"')?.strip_suffix('"')?;
    if inner.split("\", \"").any(|s| s.contains(['"', '\\', '\n', '\r'])) {
        return None;
    }
    Some(Some(inner.split("\", \"")).into_iter().flatten())
}

/// The body of the `try` in `parseLinkImport`.
fn parse_link_import_bindings(import_string: &str) -> JsResult<Vec<LinkImport>> {
    use json5::Json5Value;

    // PERF: the common `["@key", "@shareable"]` read without the JSON5 parser. JSON5 reads such
    // a list as exactly these strings; the bindings below turn each string into the same import.
    if let Some(names) = plain_string_list(import_string) {
        return Ok(names
            .map(|name| LinkImport {
                kind: if name.starts_with('@') {
                    LinkImportKind::Directive
                } else {
                    LinkImportKind::Type
                },
                name: name.to_string(),
                alias: None,
            })
            .collect());
    }

    let bindings = json5::parse(import_string)?;

    let Json5Value::Array(bindings) = bindings else {
        return Err(JsError::error("Expected an array"));
    };

    // TODO: validate the name of the "name" and "alias" properties (if it's correct name for a type or directive)

    let kind_of = |name: &str| {
        if name.starts_with('@') {
            LinkImportKind::Directive
        } else {
            LinkImportKind::Type
        }
    };

    let mut result = Vec::with_capacity(bindings.len());
    for binding in bindings {
        if let Json5Value::String(binding) = &binding {
            result.push(LinkImport {
                kind: kind_of(binding),
                name: binding.clone(),
                alias: None,
            });
            continue;
        }

        // `typeof binding === "object" && binding.name`
        let name = match &binding {
            Json5Value::Null => {
                return Err(JsError::type_error("Cannot read properties of null (reading 'name')"));
            }
            Json5Value::Object(object) => object.get("name").filter(|v| v.is_truthy()),
            // Arrays have no `name` property.
            Json5Value::Array(_) => None,
            _ => None,
        };
        let is_object = matches!(binding, Json5Value::Object(_) | Json5Value::Array(_));

        if let (true, Some(name)) = (is_object, name) {
            let Json5Value::String(name) = name else {
                return Err(JsError::type_error("binding.name.startsWith is not a function"));
            };
            let name_kind = kind_of(name);

            let as_ = match &binding {
                Json5Value::Object(object) => object.get("as").filter(|v| v.is_truthy()),
                _ => None,
            };
            let Some(as_) = as_ else {
                result.push(LinkImport {
                    kind: name_kind,
                    name: name.clone(),
                    alias: None,
                });
                continue;
            };

            let Json5Value::String(as_) = as_ else {
                return Err(JsError::type_error("binding.as.startsWith is not a function"));
            };
            let alias_kind = kind_of(as_);

            if name_kind != alias_kind {
                return Err(JsError::error(format!("{name} and {as_} must be of the same kind")));
            }

            result.push(LinkImport {
                kind: name_kind,
                name: name.clone(),
                alias: Some(as_.clone()),
            });
            continue;
        }

        return Err(JsError::error("Syntax"));
    }
    Ok(result)
}

/// `mergeLinks(links)`: groups links by identity, keeping the highest version and the union of
/// directive imports. Accepts the supergraph's `links` (`Link & { graph }`) through `&Link`.
pub fn merge_links<'l>(links: impl IntoIterator<Item = &'l Link>) -> Vec<Link> {
    struct Group {
        name: Option<String>,
        highest_version: String,
        imports: Vec<LinkImport>,
    }

    // group by identity
    let mut group_by_identity: IndexMap<String, Group> = IndexMap::default();

    for link in links {
        match group_by_identity.get_mut(&link.identity) {
            None => {
                let imported_directives: Vec<&LinkImport> =
                    link.imports.iter().filter(|im| im.kind == LinkImportKind::Directive).collect();

                if imported_directives.is_empty() {
                    continue;
                }

                group_by_identity.insert(
                    link.identity.clone(),
                    Group {
                        name: link.name.clone(),
                        highest_version: link.version.clone().unwrap_or_default(),
                        imports: link
                            .imports
                            .iter()
                            .filter(|im| im.kind == LinkImportKind::Directive)
                            .cloned()
                            .collect(),
                    },
                );
            }
            Some(existing) => {
                // select highest version
                if let Some(version) = link.version.as_deref() {
                    // NOTE(port): `parseFloat` comparison; `NaN` compares false.
                    if !version.is_empty()
                        && parse_float(&version.replacen('v', "", 1))
                            > parse_float(&existing.highest_version.replacen('v', "", 1))
                    {
                        existing.highest_version = version.to_string();
                    }
                }

                // merge imports
                for im in &link.imports {
                    // Federation v2 ignores type imports in Supergraph SDL
                    if im.kind == LinkImportKind::Type {
                        continue;
                    }

                    let has_import = existing
                        .imports
                        .iter()
                        .any(|existing_im| existing_im.kind == im.kind && existing_im.name == im.name);

                    if !has_import {
                        existing.imports.push(im.clone());
                    }
                }

                if let Some(name) = link.name.as_deref() {
                    if !name.is_empty() {
                        existing.name = Some(name.to_string());
                    }
                }
            }
        }
    }

    group_by_identity
        .into_iter()
        .map(|(identity, link)| Link {
            identity,
            version: Some(link.highest_version),
            imports: link
                .imports
                .into_iter()
                .map(|link| LinkImport {
                    kind: link.kind,
                    name: link.name,
                    alias: link.alias,
                })
                .collect(),
            name: link.name,
        })
        .collect()
}

/// `parseLink(urlString, importString)`.
pub fn parse_link(url_string: &str, import_string: &str) -> JsResult<Link> {
    let spec = parse_link_url(url_string);

    Ok(Link {
        name: spec.name,
        version: spec.version,
        identity: spec.identity,
        imports: parse_link_import(import_string)?,
    })
}

/// `parseLinkDirective(directive)`: `None` when there is no string `url` argument.
pub fn parse_link_directive(directive: &ConstDirective) -> JsResult<Option<Link>> {
    let Some(url_arg) = directive.arguments.iter().find(|arg| is_url_argument(arg)) else {
        return Ok(None);
    };

    let import_arg = directive.arguments.iter().find(|arg| is_import_argument(arg));

    let url = match &url_arg.value {
        Value::String(s) => s.value.as_str(),
        _ => unreachable!("is_url_argument checked the kind"),
    };
    let spec = parse_link_url(url);

    let import_string = match import_arg {
        Some(import_arg) => print(&import_arg.value),
        None => "[]".to_string(),
    };

    Ok(Some(Link {
        name: spec.name,
        version: spec.version,
        identity: spec.identity,
        imports: parse_link_import(&import_string)?,
    }))
}

fn is_url_argument(arg: &ConstArgument) -> bool {
    arg.name.value == "url" && matches!(arg.value, graphql_js::Value::String(_))
}

fn is_import_argument(arg: &ConstArgument) -> bool {
    arg.name.value == "import" && matches!(arg.value, graphql_js::Value::List(_))
}

pub const SDL: &str = r#"
  directive @link(
    url: String
    as: String
    for: link__Purpose
    import: [link__Import]
  ) repeatable on SCHEMA

  scalar link__Import

  enum link__Purpose {
    """
    `SECURITY` features provide metadata necessary to securely resolve fields.
    """
    SECURITY

    """
    `EXECUTION` features provide metadata necessary for operation execution.
    """
    EXECUTION
  }
"#;

/// Port of `JSON5.parse` (json5 2.2.3, `lib/parse.js`), which `parseLinkImport` uses. Same
/// state machines, values and `SyntaxError` messages (`JSON5: invalid character 'x' at 1:2`,
/// with line and column counted as json5 does, in UTF-16 code units).
///
/// NOTE(port): `ID_Start`/`ID_Continue`/`Space_Separator` are json5's Unicode tables. Space
/// separators are exact; for identifiers, non-ASCII characters use Rust's `is_alphabetic`
/// (start) and `is_alphanumeric` (continue), an approximation. The inputs are printed GraphQL
/// values, whose object keys are ASCII GraphQL names. A `\u` escape of a lone surrogate becomes
/// U+FFFD (Rust strings cannot hold it).
mod json5 {
    use crate::utils::collections::IndexMap;

    use crate::utils::js::{parse_float, JsError, JsResult};

    #[derive(Clone, Debug, PartialEq)]
    pub enum Json5Value {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<Json5Value>),
        Object(IndexMap<String, Json5Value>),
    }

    impl Json5Value {
        /// JS truthiness.
        pub fn is_truthy(&self) -> bool {
            match self {
                Json5Value::Null => false,
                Json5Value::Bool(b) => *b,
                Json5Value::Number(n) => *n != 0.0 && !n.is_nan(),
                Json5Value::String(s) => !s.is_empty(),
                Json5Value::Array(_) | Json5Value::Object(_) => true,
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum State {
        Start,
        BeforePropertyName,
        AfterPropertyName,
        BeforePropertyValue,
        BeforeArrayValue,
        AfterPropertyValue,
        AfterArrayValue,
        End,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum LexState {
        Default,
        Comment,
        MultiLineComment,
        MultiLineCommentAsterisk,
        SingleLineComment,
        Value,
        IdentifierNameStartEscape,
        IdentifierName,
        IdentifierNameEscape,
        Sign,
        Zero,
        DecimalInteger,
        DecimalPointLeading,
        DecimalPoint,
        DecimalFraction,
        DecimalExponent,
        DecimalExponentSign,
        DecimalExponentInteger,
        Hexadecimal,
        HexadecimalInteger,
        String,
        Parse(State),
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Token {
        Eof,
        Punctuator(char),
        Null,
        Boolean(bool),
        Numeric(f64),
        String(String),
        Identifier(String),
    }

    struct Parser {
        source: Vec<char>,
        pos: usize,
        line: usize,
        column: usize,
        parse_state: State,
        /// Open containers; the last one is being filled.
        stack: Vec<(Json5Value, Option<String>)>,
        key: Option<String>,
        root: Option<Json5Value>,
        // Lexer state.
        lex_state: LexState,
        buffer: String,
        double_quote: bool,
        sign: f64,
        c: Option<char>,
    }

    pub fn parse(text: &str) -> JsResult<Json5Value> {
        let mut p = Parser {
            source: text.chars().collect(),
            pos: 0,
            line: 1,
            column: 0,
            parse_state: State::Start,
            stack: Vec::new(),
            key: None,
            root: None,
            lex_state: LexState::Default,
            buffer: String::new(),
            double_quote: false,
            sign: 1.0,
            c: None,
        };
        loop {
            let token = p.lex()?;
            let eof = token == Token::Eof;
            p.parse_token(token)?;
            if eof {
                break;
            }
        }
        // `root` is undefined only for empty input, which throws above.
        Ok(p.root.unwrap_or(Json5Value::Null))
    }

    fn syntax_error(message: String) -> JsError {
        JsError::Error {
            name: "SyntaxError",
            message,
        }
    }

    fn is_space_separator(c: char) -> bool {
        matches!(c, ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
    }

    fn is_id_start_char(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '$' || c == '_' || (!c.is_ascii() && c.is_alphabetic())
    }

    fn is_id_continue_char(c: char) -> bool {
        c.is_ascii_alphanumeric()
            || c == '$'
            || c == '_'
            || c == '\u{200c}'
            || c == '\u{200d}'
            || (!c.is_ascii() && c.is_alphanumeric())
    }

    fn is_digit(c: Option<char>) -> bool {
        c.is_some_and(|c| c.is_ascii_digit())
    }

    fn is_hex_digit(c: Option<char>) -> bool {
        c.is_some_and(|c| c.is_ascii_hexdigit())
    }

    fn format_char(c: char) -> String {
        match c {
            '\'' => "\\'".to_string(),
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\u{8}' => "\\b".to_string(),
            '\u{c}' => "\\f".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            '\u{b}' => "\\v".to_string(),
            '\0' => "\\0".to_string(),
            '\u{2028}' => "\\u2028".to_string(),
            '\u{2029}' => "\\u2029".to_string(),
            c if c < ' ' => format!("\\x{:02x}", c as u32),
            c => c.to_string(),
        }
    }

    /// `Number(buffer)` for the numeric literals the lexer accepts.
    fn number(buffer: &str) -> f64 {
        if let Some(hex) = buffer.strip_prefix("0x").or_else(|| buffer.strip_prefix("0X")) {
            return hex
                .chars()
                .fold(0.0, |acc, d| acc * 16.0 + d.to_digit(16).unwrap_or(0) as f64);
        }
        parse_float(buffer)
    }

    impl Parser {
        fn peek(&self) -> Option<char> {
            self.source.get(self.pos).copied()
        }

        fn read(&mut self) -> Option<char> {
            let c = self.peek();
            match c {
                Some('\n') => {
                    self.line += 1;
                    self.column = 0;
                }
                Some(c) => self.column += c.len_utf16(),
                None => self.column += 1,
            }
            if c.is_some() {
                self.pos += 1;
            }
            c
        }

        fn invalid_char(&self, c: Option<char>) -> JsError {
            match c {
                None => syntax_error(format!("JSON5: invalid end of input at {}:{}", self.line, self.column)),
                Some(c) => syntax_error(format!(
                    "JSON5: invalid character '{}' at {}:{}",
                    format_char(c),
                    self.line,
                    self.column
                )),
            }
        }

        fn invalid_eof(&self) -> JsError {
            syntax_error(format!("JSON5: invalid end of input at {}:{}", self.line, self.column))
        }

        fn invalid_identifier(&mut self) -> JsError {
            self.column = self.column.saturating_sub(5);
            syntax_error(format!("JSON5: invalid identifier character at {}:{}", self.line, self.column))
        }

        fn read_invalid(&mut self) -> JsError {
            let c = self.read();
            self.invalid_char(c)
        }

        fn literal(&mut self, s: &str) -> JsResult<()> {
            for c in s.chars() {
                if self.peek() != Some(c) {
                    return Err(self.read_invalid());
                }
                self.read();
            }
            Ok(())
        }

        fn lex(&mut self) -> JsResult<Token> {
            self.lex_state = LexState::Default;
            self.buffer = String::new();
            self.double_quote = false;
            self.sign = 1.0;

            loop {
                self.c = self.peek();
                let state = self.lex_state;
                if let Some(token) = self.lex_step(state)? {
                    return Ok(token);
                }
            }
        }

        fn push_read(&mut self) {
            if let Some(c) = self.read() {
                self.buffer.push(c);
            }
        }

        fn set_buffer_read(&mut self) {
            self.buffer = self.read().map(String::from).unwrap_or_default();
        }

        fn lex_step(&mut self, state: LexState) -> JsResult<Option<Token>> {
            let c = self.c;
            match state {
                LexState::Default => {
                    match c {
                        Some(
                            '\t' | '\u{b}' | '\u{c}' | ' ' | '\u{a0}' | '\u{feff}' | '\n' | '\r' | '\u{2028}'
                            | '\u{2029}',
                        ) => {
                            self.read();
                            return Ok(None);
                        }
                        Some('/') => {
                            self.read();
                            self.lex_state = LexState::Comment;
                            return Ok(None);
                        }
                        None => {
                            self.read();
                            return Ok(Some(Token::Eof));
                        }
                        Some(c) if is_space_separator(c) => {
                            self.read();
                            return Ok(None);
                        }
                        _ => {}
                    }
                    self.lex_step(LexState::Parse(self.parse_state))
                }
                LexState::Comment => match c {
                    Some('*') => {
                        self.read();
                        self.lex_state = LexState::MultiLineComment;
                        Ok(None)
                    }
                    Some('/') => {
                        self.read();
                        self.lex_state = LexState::SingleLineComment;
                        Ok(None)
                    }
                    _ => Err(self.read_invalid()),
                },
                LexState::MultiLineComment => match c {
                    Some('*') => {
                        self.read();
                        self.lex_state = LexState::MultiLineCommentAsterisk;
                        Ok(None)
                    }
                    None => Err(self.read_invalid()),
                    _ => {
                        self.read();
                        Ok(None)
                    }
                },
                LexState::MultiLineCommentAsterisk => match c {
                    Some('*') => {
                        self.read();
                        Ok(None)
                    }
                    Some('/') => {
                        self.read();
                        self.lex_state = LexState::Default;
                        Ok(None)
                    }
                    None => Err(self.read_invalid()),
                    _ => {
                        self.read();
                        self.lex_state = LexState::MultiLineComment;
                        Ok(None)
                    }
                },
                LexState::SingleLineComment => match c {
                    Some('\n' | '\r' | '\u{2028}' | '\u{2029}') => {
                        self.read();
                        self.lex_state = LexState::Default;
                        Ok(None)
                    }
                    None => {
                        self.read();
                        Ok(Some(Token::Eof))
                    }
                    _ => {
                        self.read();
                        Ok(None)
                    }
                },
                LexState::Value => match c {
                    Some(p @ ('{' | '[')) => {
                        self.read();
                        Ok(Some(Token::Punctuator(p)))
                    }
                    Some('n') => {
                        self.read();
                        self.literal("ull")?;
                        Ok(Some(Token::Null))
                    }
                    Some('t') => {
                        self.read();
                        self.literal("rue")?;
                        Ok(Some(Token::Boolean(true)))
                    }
                    Some('f') => {
                        self.read();
                        self.literal("alse")?;
                        Ok(Some(Token::Boolean(false)))
                    }
                    Some('-' | '+') => {
                        if self.read() == Some('-') {
                            self.sign = -1.0;
                        }
                        self.lex_state = LexState::Sign;
                        Ok(None)
                    }
                    Some('.') => {
                        self.set_buffer_read();
                        self.lex_state = LexState::DecimalPointLeading;
                        Ok(None)
                    }
                    Some('0') => {
                        self.set_buffer_read();
                        self.lex_state = LexState::Zero;
                        Ok(None)
                    }
                    Some('1'..='9') => {
                        self.set_buffer_read();
                        self.lex_state = LexState::DecimalInteger;
                        Ok(None)
                    }
                    Some('I') => {
                        self.read();
                        self.literal("nfinity")?;
                        Ok(Some(Token::Numeric(f64::INFINITY)))
                    }
                    Some('N') => {
                        self.read();
                        self.literal("aN")?;
                        Ok(Some(Token::Numeric(f64::NAN)))
                    }
                    Some('"' | '\'') => {
                        self.double_quote = self.read() == Some('"');
                        self.buffer = String::new();
                        self.lex_state = LexState::String;
                        Ok(None)
                    }
                    _ => Err(self.read_invalid()),
                },
                LexState::IdentifierNameStartEscape => {
                    if c != Some('u') {
                        return Err(self.read_invalid());
                    }
                    self.read();
                    let u = self.unicode_escape()?;
                    match u {
                        '$' | '_' => {}
                        u => {
                            if !is_id_start_char(u) {
                                return Err(self.invalid_identifier());
                            }
                        }
                    }
                    self.buffer.push(u);
                    self.lex_state = LexState::IdentifierName;
                    Ok(None)
                }
                LexState::IdentifierName => match c {
                    Some('$' | '_' | '\u{200c}' | '\u{200d}') => {
                        self.push_read();
                        Ok(None)
                    }
                    Some('\\') => {
                        self.read();
                        self.lex_state = LexState::IdentifierNameEscape;
                        Ok(None)
                    }
                    Some(c) if is_id_continue_char(c) => {
                        self.push_read();
                        Ok(None)
                    }
                    _ => Ok(Some(Token::Identifier(self.buffer.clone()))),
                },
                LexState::IdentifierNameEscape => {
                    if c != Some('u') {
                        return Err(self.read_invalid());
                    }
                    self.read();
                    let u = self.unicode_escape()?;
                    match u {
                        '$' | '_' | '\u{200c}' | '\u{200d}' => {}
                        u => {
                            if !is_id_continue_char(u) {
                                return Err(self.invalid_identifier());
                            }
                        }
                    }
                    self.buffer.push(u);
                    self.lex_state = LexState::IdentifierName;
                    Ok(None)
                }
                LexState::Sign => match c {
                    Some('.') => {
                        self.set_buffer_read();
                        self.lex_state = LexState::DecimalPointLeading;
                        Ok(None)
                    }
                    Some('0') => {
                        self.set_buffer_read();
                        self.lex_state = LexState::Zero;
                        Ok(None)
                    }
                    Some('1'..='9') => {
                        self.set_buffer_read();
                        self.lex_state = LexState::DecimalInteger;
                        Ok(None)
                    }
                    Some('I') => {
                        self.read();
                        self.literal("nfinity")?;
                        Ok(Some(Token::Numeric(self.sign * f64::INFINITY)))
                    }
                    Some('N') => {
                        self.read();
                        self.literal("aN")?;
                        Ok(Some(Token::Numeric(f64::NAN)))
                    }
                    _ => Err(self.read_invalid()),
                },
                LexState::Zero => match c {
                    Some('.') => {
                        self.push_read();
                        self.lex_state = LexState::DecimalPoint;
                        Ok(None)
                    }
                    Some('e' | 'E') => {
                        self.push_read();
                        self.lex_state = LexState::DecimalExponent;
                        Ok(None)
                    }
                    Some('x' | 'X') => {
                        self.push_read();
                        self.lex_state = LexState::Hexadecimal;
                        Ok(None)
                    }
                    _ => Ok(Some(Token::Numeric(self.sign * 0.0))),
                },
                LexState::DecimalInteger => match c {
                    Some('.') => {
                        self.push_read();
                        self.lex_state = LexState::DecimalPoint;
                        Ok(None)
                    }
                    Some('e' | 'E') => {
                        self.push_read();
                        self.lex_state = LexState::DecimalExponent;
                        Ok(None)
                    }
                    _ if is_digit(c) => {
                        self.push_read();
                        Ok(None)
                    }
                    _ => Ok(Some(Token::Numeric(self.sign * number(&self.buffer)))),
                },
                LexState::DecimalPointLeading => {
                    if is_digit(c) {
                        self.push_read();
                        self.lex_state = LexState::DecimalFraction;
                        return Ok(None);
                    }
                    Err(self.read_invalid())
                }
                LexState::DecimalPoint => match c {
                    Some('e' | 'E') => {
                        self.push_read();
                        self.lex_state = LexState::DecimalExponent;
                        Ok(None)
                    }
                    _ if is_digit(c) => {
                        self.push_read();
                        self.lex_state = LexState::DecimalFraction;
                        Ok(None)
                    }
                    _ => Ok(Some(Token::Numeric(self.sign * number(&self.buffer)))),
                },
                LexState::DecimalFraction => match c {
                    Some('e' | 'E') => {
                        self.push_read();
                        self.lex_state = LexState::DecimalExponent;
                        Ok(None)
                    }
                    _ if is_digit(c) => {
                        self.push_read();
                        Ok(None)
                    }
                    _ => Ok(Some(Token::Numeric(self.sign * number(&self.buffer)))),
                },
                LexState::DecimalExponent => match c {
                    Some('+' | '-') => {
                        self.push_read();
                        self.lex_state = LexState::DecimalExponentSign;
                        Ok(None)
                    }
                    _ if is_digit(c) => {
                        self.push_read();
                        self.lex_state = LexState::DecimalExponentInteger;
                        Ok(None)
                    }
                    _ => Err(self.read_invalid()),
                },
                LexState::DecimalExponentSign => {
                    if is_digit(c) {
                        self.push_read();
                        self.lex_state = LexState::DecimalExponentInteger;
                        return Ok(None);
                    }
                    Err(self.read_invalid())
                }
                LexState::DecimalExponentInteger => {
                    if is_digit(c) {
                        self.push_read();
                        return Ok(None);
                    }
                    Ok(Some(Token::Numeric(self.sign * number(&self.buffer))))
                }
                LexState::Hexadecimal => {
                    if is_hex_digit(c) {
                        self.push_read();
                        self.lex_state = LexState::HexadecimalInteger;
                        return Ok(None);
                    }
                    Err(self.read_invalid())
                }
                LexState::HexadecimalInteger => {
                    if is_hex_digit(c) {
                        self.push_read();
                        return Ok(None);
                    }
                    Ok(Some(Token::Numeric(self.sign * number(&self.buffer))))
                }
                LexState::String => match c {
                    Some('\\') => {
                        self.read();
                        let escaped = self.escape()?;
                        self.buffer.push_str(&escaped);
                        Ok(None)
                    }
                    Some('"') => {
                        if self.double_quote {
                            self.read();
                            return Ok(Some(Token::String(self.buffer.clone())));
                        }
                        self.push_read();
                        Ok(None)
                    }
                    Some('\'') => {
                        if !self.double_quote {
                            self.read();
                            return Ok(Some(Token::String(self.buffer.clone())));
                        }
                        self.push_read();
                        Ok(None)
                    }
                    Some('\n' | '\r') => Err(self.read_invalid()),
                    None => Err(self.read_invalid()),
                    // `\u2028`/`\u2029` only warn (console.warn), then are kept.
                    _ => {
                        self.push_read();
                        Ok(None)
                    }
                },
                LexState::Parse(State::Start) => match c {
                    Some(p @ ('{' | '[')) => {
                        self.read();
                        Ok(Some(Token::Punctuator(p)))
                    }
                    _ => {
                        self.lex_state = LexState::Value;
                        Ok(None)
                    }
                },
                LexState::Parse(State::BeforePropertyName) => match c {
                    Some('$' | '_') => {
                        self.set_buffer_read();
                        self.lex_state = LexState::IdentifierName;
                        Ok(None)
                    }
                    Some('\\') => {
                        self.read();
                        self.lex_state = LexState::IdentifierNameStartEscape;
                        Ok(None)
                    }
                    Some('}') => {
                        self.read();
                        Ok(Some(Token::Punctuator('}')))
                    }
                    Some('"' | '\'') => {
                        self.double_quote = self.read() == Some('"');
                        self.lex_state = LexState::String;
                        Ok(None)
                    }
                    Some(ch) if is_id_start_char(ch) => {
                        self.push_read();
                        self.lex_state = LexState::IdentifierName;
                        Ok(None)
                    }
                    _ => Err(self.read_invalid()),
                },
                LexState::Parse(State::AfterPropertyName) => {
                    if c == Some(':') {
                        self.read();
                        return Ok(Some(Token::Punctuator(':')));
                    }
                    Err(self.read_invalid())
                }
                LexState::Parse(State::BeforePropertyValue) => {
                    self.lex_state = LexState::Value;
                    Ok(None)
                }
                LexState::Parse(State::AfterPropertyValue) => match c {
                    Some(p @ (',' | '}')) => {
                        self.read();
                        Ok(Some(Token::Punctuator(p)))
                    }
                    _ => Err(self.read_invalid()),
                },
                LexState::Parse(State::BeforeArrayValue) => {
                    if c == Some(']') {
                        self.read();
                        return Ok(Some(Token::Punctuator(']')));
                    }
                    self.lex_state = LexState::Value;
                    Ok(None)
                }
                LexState::Parse(State::AfterArrayValue) => match c {
                    Some(p @ (',' | ']')) => {
                        self.read();
                        Ok(Some(Token::Punctuator(p)))
                    }
                    _ => Err(self.read_invalid()),
                },
                LexState::Parse(State::End) => Err(self.read_invalid()),
            }
        }

        fn escape(&mut self) -> JsResult<String> {
            let c = self.peek();
            let simple = |s: &str| Ok(s.to_string());
            match c {
                Some('b') => {
                    self.read();
                    simple("\u{8}")
                }
                Some('f') => {
                    self.read();
                    simple("\u{c}")
                }
                Some('n') => {
                    self.read();
                    simple("\n")
                }
                Some('r') => {
                    self.read();
                    simple("\r")
                }
                Some('t') => {
                    self.read();
                    simple("\t")
                }
                Some('v') => {
                    self.read();
                    simple("\u{b}")
                }
                Some('0') => {
                    self.read();
                    if is_digit(self.peek()) {
                        return Err(self.read_invalid());
                    }
                    simple("\0")
                }
                Some('x') => {
                    self.read();
                    Ok(self.hex_escape()?.to_string())
                }
                Some('u') => {
                    self.read();
                    Ok(self.unicode_escape()?.to_string())
                }
                Some('\n' | '\u{2028}' | '\u{2029}') => {
                    self.read();
                    simple("")
                }
                Some('\r') => {
                    self.read();
                    if self.peek() == Some('\n') {
                        self.read();
                    }
                    simple("")
                }
                Some('1'..='9') | None => Err(self.read_invalid()),
                Some(_) => Ok(self.read().map(String::from).unwrap_or_default()),
            }
        }

        fn hex_escape(&mut self) -> JsResult<char> {
            let mut buffer = String::new();
            for _ in 0..2 {
                if !is_hex_digit(self.peek()) {
                    return Err(self.read_invalid());
                }
                if let Some(c) = self.read() {
                    buffer.push(c);
                }
            }
            let code = u32::from_str_radix(&buffer, 16).unwrap_or(0);
            Ok(char::from_u32(code).unwrap_or('\u{fffd}'))
        }

        fn unicode_escape(&mut self) -> JsResult<char> {
            let mut buffer = String::new();
            for _ in 0..4 {
                if !is_hex_digit(self.peek()) {
                    return Err(self.read_invalid());
                }
                if let Some(c) = self.read() {
                    buffer.push(c);
                }
            }
            let code = u32::from_str_radix(&buffer, 16).unwrap_or(0);
            Ok(char::from_u32(code).unwrap_or('\u{fffd}'))
        }

        fn parse_token(&mut self, token: Token) -> JsResult<()> {
            match self.parse_state {
                State::Start => {
                    if token == Token::Eof {
                        return Err(self.invalid_eof());
                    }
                    self.push(token);
                }
                State::BeforePropertyName => match token {
                    Token::Identifier(value) | Token::String(value) => {
                        self.key = Some(value);
                        self.parse_state = State::AfterPropertyName;
                    }
                    Token::Punctuator(_) => self.pop(),
                    Token::Eof => return Err(self.invalid_eof()),
                    _ => {}
                },
                State::AfterPropertyName => {
                    if token == Token::Eof {
                        return Err(self.invalid_eof());
                    }
                    self.parse_state = State::BeforePropertyValue;
                }
                State::BeforePropertyValue => {
                    if token == Token::Eof {
                        return Err(self.invalid_eof());
                    }
                    self.push(token);
                }
                State::BeforeArrayValue => {
                    if token == Token::Eof {
                        return Err(self.invalid_eof());
                    }
                    if token == Token::Punctuator(']') {
                        self.pop();
                        return Ok(());
                    }
                    self.push(token);
                }
                State::AfterPropertyValue => {
                    if token == Token::Eof {
                        return Err(self.invalid_eof());
                    }
                    match token {
                        Token::Punctuator(',') => self.parse_state = State::BeforePropertyName,
                        Token::Punctuator('}') => self.pop(),
                        _ => {}
                    }
                }
                State::AfterArrayValue => {
                    if token == Token::Eof {
                        return Err(self.invalid_eof());
                    }
                    match token {
                        Token::Punctuator(',') => self.parse_state = State::BeforeArrayValue,
                        Token::Punctuator(']') => self.pop(),
                        _ => {}
                    }
                }
                State::End => {}
            }
            Ok(())
        }

        /// Adds a finished value to the current container (or makes it the root).
        fn attach(&mut self, value: Json5Value, key: Option<String>) {
            match self.stack.last_mut() {
                None => self.root = Some(value),
                Some((Json5Value::Array(items), _)) => items.push(value),
                Some((Json5Value::Object(fields), _)) => {
                    fields.insert(key.unwrap_or_default(), value);
                }
                Some(_) => {}
            }
        }

        fn after_value_state(&self) -> State {
            match self.stack.last() {
                None => State::End,
                Some((Json5Value::Array(_), _)) => State::AfterArrayValue,
                Some(_) => State::AfterPropertyValue,
            }
        }

        fn push(&mut self, token: Token) {
            let value = match token {
                Token::Punctuator('{') => Json5Value::Object(IndexMap::default()),
                Token::Punctuator('[') => Json5Value::Array(Vec::new()),
                Token::Null => Json5Value::Null,
                Token::Boolean(b) => Json5Value::Bool(b),
                Token::Numeric(n) => Json5Value::Number(n),
                Token::String(s) => Json5Value::String(s),
                // Unreachable in json5 too.
                _ => Json5Value::Null,
            };

            // The key under which this value goes into the current object.
            let key = self.key.take();
            match value {
                Json5Value::Object(_) | Json5Value::Array(_) => {
                    // Containers are attached when they are closed (`pop`); the TS attaches
                    // them first and fills them by reference, which gives the same result.
                    let is_array = matches!(value, Json5Value::Array(_));
                    self.stack.push((value, key));
                    self.parse_state = if is_array {
                        State::BeforeArrayValue
                    } else {
                        State::BeforePropertyName
                    };
                }
                value => {
                    self.attach(value, key);
                    self.parse_state = self.after_value_state();
                }
            }
        }

        fn pop(&mut self) {
            if let Some((value, key)) = self.stack.pop() {
                self.attach(value, key);
            }
            self.parse_state = self.after_value_state();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    /// `src/specifications/link.spec.ts`, `describe("parseLinkUrl")`: one test per row of the
    /// `test.each` table, named after the row's `$url` title.
    mod parse_link_url {
        use super::*;

        /// `expect(parseLinkUrl(url)).toEqual({ name, version, identity })`.
        fn check(url: &str, name: Option<&str>, version: Option<&str>, identity: &str) {
            assert_eq!(
                parse_link_url(url),
                LinkUrl {
                    name: name.map(str::to_string),
                    version: version.map(str::to_string),
                    identity: identity.to_string()
                },
                "{url}"
            );
        }

        #[test]
        fn https_spec_example_com_a_b_my_schema_v1_0() {
            check("https://spec.example.com/a/b/mySchema/v1.0/", Some("mySchema"), Some("v1.0"), "https://spec.example.com/a/b/mySchema");
        }

        #[test]
        fn https_spec_example_com_a_b_my_schema_v1_0_2() {
            check("https://spec.example.com/a/b/mySchema/v1.0", Some("mySchema"), Some("v1.0"), "https://spec.example.com/a/b/mySchema");
        }

        #[test]
        fn https_spec_example_com() {
            check("https://spec.example.com", None, None, "https://spec.example.com");
        }

        #[test]
        fn https_spec_example_com_my_schema_v0_1_q_v_frag() {
            check("https://spec.example.com/mySchema/v0.1?q=v#frag", Some("mySchema"), Some("v0.1"), "https://spec.example.com/mySchema");
        }

        #[test]
        fn https_spec_example_com_my_schema_not_a_version() {
            check("https://spec.example.com/mySchema/not-a-version", Some("not-a-version"), None, "https://spec.example.com/mySchema/not-a-version");
        }

        #[test]
        fn https_spec_example_com_v1_0() {
            check("https://spec.example.com/v1.0", None, Some("v1.0"), "https://spec.example.com");
        }

        #[test]
        fn https_spec_example_com_v_x() {
            check("https://spec.example.com/vX", Some("vX"), None, "https://spec.example.com/vX");
        }

        #[test]
        fn file_extensions_permissions() {
            check("file:///extensions/permissions", Some("permissions"), None, "file:///extensions/permissions");
        }

        #[test]
        fn https_specs_apollo_dev_federation_not_a_version() {
            check("https://specs.apollo.dev/federation/not-a-version", Some("not-a-version"), None, "https://specs.apollo.dev/federation/not-a-version");
        }

        #[test]
        fn https_specs_apollo_dev() {
            check("https://specs.apollo.dev", None, None, "https://specs.apollo.dev");
        }

        #[test]
        fn specs_apollo_dev() {
            check("specs.apollo.dev", None, None, "specs.apollo.dev");
        }

        #[test]
        fn not_valid_apollo_dev_v1() {
            check("not-valid.apollo.dev/v1", None, None, "not-valid.apollo.dev/v1");
        }
    }

    fn import(kind: LinkImportKind, name: &str, alias: Option<&str>) -> LinkImport {
        LinkImport {
            kind,
            name: name.to_string(),
            alias: alias.map(str::to_string),
        }
    }

    /// `src/specifications/link.spec.ts`, `describe("parseLinkImport")`.
    mod parse_link_import {
        use super::*;
        use LinkImportKind::*;

        /// `test('["Type"]')`.
        #[test]
        fn r#type() {
            assert_eq!(parse_link_import("[\"Type\"]").unwrap(), [import(Type, "Type", None)]);
        }

        /// `test('["Foo", "Bar"]')`.
        #[test]
        fn foo_bar() {
            assert_eq!(
                parse_link_import("[\"Foo\", \"Bar\"]").unwrap(),
                [import(Type, "Foo", None), import(Type, "Bar", None)]
            );
        }

        /// `test('["@foo", "@bar"]')`.
        #[test]
        fn foo_bar_2() {
            assert_eq!(
                parse_link_import("[\"@foo\", \"@bar\"]").unwrap(),
                [import(Directive, "@foo", None), import(Directive, "@bar", None)]
            );
        }

        /// `test('[{ name: "@foo", as: "@bar" }]')`.
        #[test]
        fn name_foo_as_bar() {
            assert_eq!(
                parse_link_import("[{ name: \"@foo\", as: \"@bar\" }]").unwrap(),
                [import(Directive, "@foo", Some("@bar"))]
            );
        }

        /// `test('[{ name: "@foo"]')`.
        #[test]
        fn name_foo() {
            assert_eq!(parse_link_import("[{ name: \"@foo\" }]").unwrap(), [import(Directive, "@foo", None)]);
        }

        /// `test("[]")`.
        #[test]
        fn untitled() {
            assert_eq!(parse_link_import("[]").unwrap(), []);
        }
    }

    /// Error messages generated with Node (`json5` 2.2.3).
    #[test]
    fn parse_link_import_errors() {
        let err = |s: &str| parse_link_import(s).unwrap_err().message().to_string();
        let cases = [
            ("[FOO]", "SyntaxError: JSON5: invalid character 'F' at 1:2"),
            ("[\"\"\"\nabc\n\"\"\"]", "SyntaxError: JSON5: invalid character '\\\"' at 1:4"),
            ("[nope]", "SyntaxError: JSON5: invalid character 'o' at 1:3"),
            ("", "SyntaxError: JSON5: invalid end of input at 1:1"),
            ("[", "SyntaxError: JSON5: invalid end of input at 1:2"),
            ("{a: 1", "SyntaxError: JSON5: invalid end of input at 1:6"),
            ("[1, 2", "SyntaxError: JSON5: invalid end of input at 1:6"),
            ("[\"é\", x]", "SyntaxError: JSON5: invalid character 'x' at 1:7"),
            ("[\"😀\", x]", "SyntaxError: JSON5: invalid character 'x' at 1:8"),
            ("[\"\\01\"]", "SyntaxError: JSON5: invalid character '1' at 1:5"),
            ("[1]]", "SyntaxError: JSON5: invalid character ']' at 1:4"),
            ("[\"a\nb\"]", "SyntaxError: JSON5: invalid character '\\n' at 2:0"),
            ("[\u{1}]", "SyntaxError: JSON5: invalid character '\\x01' at 1:2"),
            ("\"@key\"", "Error: Expected an array"),
            ("[1]", "Error: Syntax"),
            ("[[1]]", "Error: Syntax"),
            ("[{as: \"@a\"}]", "Error: Syntax"),
            ("[null]", "TypeError: Cannot read properties of null (reading 'name')"),
            ("[{name: 1}]", "TypeError: binding.name.startsWith is not a function"),
            ("[{name: \"@a\", as: 2}]", "TypeError: binding.as.startsWith is not a function"),
            ("[{name: \"@a\", as: \"b\"}]", "Error: @a and b must be of the same kind"),
        ];
        for (input, inner) in cases {
            assert_eq!(err(input), format!("Invalid import binding: {input}: {inner}"), "{input:?}");
        }
        assert_eq!(
            parse_link_import("[{name: \"@a\", as: \"\"}, // c\n \"b\" /* x */]").unwrap(),
            [import(LinkImportKind::Directive, "@a", None), import(LinkImportKind::Type, "b", None)]
        );
        assert_eq!(
            json5::parse("[0x1F, .5, 5., +Infinity, 1e3, \"\\x41\\u0042\"]").unwrap(),
            json5::Json5Value::Array(vec![
                json5::Json5Value::Number(31.0),
                json5::Json5Value::Number(0.5),
                json5::Json5Value::Number(5.0),
                json5::Json5Value::Number(f64::INFINITY),
                json5::Json5Value::Number(1000.0),
                json5::Json5Value::String("AB".to_string()),
            ])
        );
    }

    #[test]
    fn print_and_merge_links() {
        let directive = graphql_js::parse(
            r#"extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key", {name: "@tag", as: "@mytag"}, "FieldSet"])"#,
        )
        .unwrap();
        let graphql_js::Definition::SchemaExtension(ext) = &directive.definitions[0] else {
            panic!()
        };
        let link = parse_link_directive(&ext.directives[0]).unwrap().unwrap();
        assert_eq!(link.identity, "https://specs.apollo.dev/federation");
        assert_eq!(link.version, s("v2.3"));
        assert_eq!(
            print_link(&link),
            "@link(\n  url: \"https://specs.apollo.dev/federation/v2.3\"\n  import: [\"@key\", {name: \"@tag\", as: \"@mytag\"}, \"FieldSet\"]\n)"
        );
        let other = parse_link("https://specs.apollo.dev/federation/v2.5", r#"["@key", "@shareable"]"#).unwrap();
        let no_directives = parse_link("https://specs.example.dev/x/v1.0", r#"["T"]"#).unwrap();
        let merged = merge_links([&no_directives, &link, &other]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].version, s("v2.5"));
        assert_eq!(
            merged[0].imports,
            [
                import(LinkImportKind::Directive, "@key", None),
                import(LinkImportKind::Directive, "@tag", Some("@mytag")),
                import(LinkImportKind::Directive, "@shareable", None),
            ]
        );
    }
}
