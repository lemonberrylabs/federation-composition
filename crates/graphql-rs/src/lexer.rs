//! Port of graphql-js `language/lexer` (and `schemaCoordinateLexer`).
//!
//! graphql-js lexes UTF-16 code units; this lexer works on UTF-8 bytes. All
//! token delimiters are ASCII, so token boundaries are the same; token
//! offsets are byte offsets. Error messages are identical: where graphql-js
//! quotes a slice measured in code units, the slice is reproduced in code
//! units (see [`crate::js::utf16_slice`]).
//!
//! Errors are boxed to keep the results on the hot path small.

use std::borrow::Cow;

use crate::block_string::dedent_block_string_lines;
use crate::error::GraphQLError;
use crate::js::utf16_slice;

/// graphql-js `TokenKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Sof,
    Eof,
    Bang,
    Dollar,
    Amp,
    ParenL,
    ParenR,
    Dot,
    Spread,
    Colon,
    Equals,
    At,
    BracketL,
    BracketR,
    BraceL,
    Pipe,
    BraceR,
    Name,
    Int,
    Float,
    String,
    BlockString,
    Comment,
}

impl TokenKind {
    /// The graphql-js `TokenKind` string.
    pub fn as_str(self) -> &'static str {
        match self {
            TokenKind::Sof => "<SOF>",
            TokenKind::Eof => "<EOF>",
            TokenKind::Bang => "!",
            TokenKind::Dollar => "$",
            TokenKind::Amp => "&",
            TokenKind::ParenL => "(",
            TokenKind::ParenR => ")",
            TokenKind::Dot => ".",
            TokenKind::Spread => "...",
            TokenKind::Colon => ":",
            TokenKind::Equals => "=",
            TokenKind::At => "@",
            TokenKind::BracketL => "[",
            TokenKind::BracketR => "]",
            TokenKind::BraceL => "{",
            TokenKind::Pipe => "|",
            TokenKind::BraceR => "}",
            TokenKind::Name => "Name",
            TokenKind::Int => "Int",
            TokenKind::Float => "Float",
            TokenKind::String => "String",
            TokenKind::BlockString => "BlockString",
            TokenKind::Comment => "Comment",
        }
    }

    /// graphql-js `isPunctuatorTokenKind`.
    pub fn is_punctuator(self) -> bool {
        matches!(
            self,
            TokenKind::Bang
                | TokenKind::Dollar
                | TokenKind::Amp
                | TokenKind::ParenL
                | TokenKind::ParenR
                | TokenKind::Dot
                | TokenKind::Spread
                | TokenKind::Colon
                | TokenKind::Equals
                | TokenKind::At
                | TokenKind::BracketL
                | TokenKind::BracketR
                | TokenKind::BraceL
                | TokenKind::Pipe
                | TokenKind::BraceR
        )
    }
}

/// A lexed token. `start`/`end` are byte offsets into the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Token<'s> {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
    /// The token value; `None` for punctuators, `<SOF>` and `<EOF>`, exactly
    /// where graphql-js leaves `value` undefined.
    pub value: Option<Cow<'s, str>>,
}

impl<'s> Token<'s> {
    fn new(kind: TokenKind, start: usize, end: usize, value: Option<Cow<'s, str>>) -> Self {
        Token {
            kind,
            start,
            end,
            value,
        }
    }

    /// graphql-js parser `getTokenDesc`.
    pub fn desc(&self) -> String {
        token_desc(self.kind, self.value.as_deref())
    }
}

/// graphql-js parser `getTokenKindDesc`.
pub(crate) fn token_kind_desc(kind: TokenKind) -> String {
    if kind.is_punctuator() {
        format!("\"{}\"", kind.as_str())
    } else {
        kind.as_str().to_owned()
    }
}

/// graphql-js parser `getTokenDesc`.
pub(crate) fn token_desc(kind: TokenKind, value: Option<&str>) -> String {
    match value {
        Some(v) => format!("{} \"{}\"", token_kind_desc(kind), v),
        None => token_kind_desc(kind),
    }
}

/// graphql-js `Lexer`. With `coordinate` set it is graphql-js
/// `SchemaCoordinateLexer`.
pub struct Lexer<'s> {
    source: &'s str,
    /// The current token.
    pub token: Token<'s>,
    /// The token after `token`, when `lookahead` has read it.
    peeked: Option<Token<'s>>,
    coordinate: bool,
}

impl<'s> Lexer<'s> {
    pub fn new(source: &'s str) -> Self {
        Lexer {
            source,
            token: Token::new(TokenKind::Sof, 0, 0, None),
            peeked: None,
            coordinate: false,
        }
    }

    pub(crate) fn new_coordinate(source: &'s str) -> Self {
        Lexer {
            coordinate: true,
            ..Lexer::new(source)
        }
    }

    pub fn source(&self) -> &'s str {
        self.source
    }

    /// graphql-js `advance`: moves to the next non-comment token and returns
    /// the previous one.
    pub fn advance(&mut self) -> Result<Token<'s>, Box<GraphQLError>> {
        let next = match self.peeked.take() {
            Some(t) => t,
            None => self.next_after_current()?,
        };
        Ok(std::mem::replace(&mut self.token, next))
    }

    /// graphql-js `lookahead`: the next non-comment token, without consuming.
    pub fn lookahead(&mut self) -> Result<&Token<'s>, Box<GraphQLError>> {
        if self.token.kind == TokenKind::Eof {
            return Ok(&self.token);
        }
        if self.peeked.is_none() {
            self.peeked = Some(self.next_after_current()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }

    fn next_after_current(&self) -> Result<Token<'s>, Box<GraphQLError>> {
        if self.token.kind == TokenKind::Eof {
            return Ok(self.token.clone());
        }
        let mut pos = self.token.end;
        loop {
            let token = if self.coordinate {
                self.read_next_coordinate_token(pos)?
            } else {
                self.read_next_token(pos)?
            };
            if token.kind != TokenKind::Comment {
                return Ok(token);
            }
            pos = token.end;
        }
    }

    #[inline]
    fn at(&self, pos: usize) -> u8 {
        // Past the end graphql-js reads NaN; NUL compares the same way
        // against every character the lexer looks for.
        *self.source.as_bytes().get(pos).unwrap_or(&0)
    }

    fn punct(&self, kind: TokenKind, pos: usize, len: usize) -> Token<'s> {
        Token::new(kind, pos, pos + len, None)
    }

    fn read_next_token(&self, start: usize) -> Result<Token<'s>, Box<GraphQLError>> {
        let body = self.source.as_bytes();
        let body_length = body.len();
        let mut position = start;
        while position < body_length {
            let code = body[position];
            if CLASS[code as usize] == IGNORED {
                position += 1;
                continue;
            }
            match code {
                // U+FEFF (BOM), encoded EF BB BF.
                0xEF if body.get(position + 1) == Some(&0xBB)
                    && body.get(position + 2) == Some(&0xBF) =>
                {
                    position += 3;
                    continue;
                }
                b'\t' | b' ' | b',' | b'\n' => {
                    position += 1;
                    continue;
                }
                b'\r' => {
                    position += if self.at(position + 1) == b'\n' { 2 } else { 1 };
                    continue;
                }
                b'#' => return Ok(self.read_comment(position)),
                b'!' => return Ok(self.punct(TokenKind::Bang, position, 1)),
                b'$' => return Ok(self.punct(TokenKind::Dollar, position, 1)),
                b'&' => return Ok(self.punct(TokenKind::Amp, position, 1)),
                b'(' => return Ok(self.punct(TokenKind::ParenL, position, 1)),
                b')' => return Ok(self.punct(TokenKind::ParenR, position, 1)),
                b'.' => {
                    let next_code = self.at(position + 1);
                    if next_code == b'.' && self.at(position + 2) == b'.' {
                        return Ok(self.punct(TokenKind::Spread, position, 3));
                    }
                    if next_code == b'.' {
                        return Err(GraphQLError::syntax(
                            "Unexpected \"..\", did you mean \"...\"?",
                        ));
                    } else if next_code.is_ascii_digit() {
                        let end = self.read_digits(position + 1, next_code)?;
                        let digits = &self.source[position + 1..end];
                        return Err(GraphQLError::syntax(format!(
                            "Invalid number, expected digit before \".\", did you mean \"0.{digits}\"?"
                        )));
                    }
                }
                b':' => return Ok(self.punct(TokenKind::Colon, position, 1)),
                b'=' => return Ok(self.punct(TokenKind::Equals, position, 1)),
                b'@' => return Ok(self.punct(TokenKind::At, position, 1)),
                b'[' => return Ok(self.punct(TokenKind::BracketL, position, 1)),
                b']' => return Ok(self.punct(TokenKind::BracketR, position, 1)),
                b'{' => return Ok(self.punct(TokenKind::BraceL, position, 1)),
                b'|' => return Ok(self.punct(TokenKind::Pipe, position, 1)),
                b'}' => return Ok(self.punct(TokenKind::BraceR, position, 1)),
                b'"' => {
                    if self.at(position + 1) == b'"' && self.at(position + 2) == b'"' {
                        return self.read_block_string(position);
                    }
                    return self.read_string(position);
                }
                _ => {}
            }
            if code.is_ascii_digit() || code == b'-' {
                return self.read_number(position, code);
            }
            if is_name_start(code) {
                return Ok(self.read_name(position));
            }
            // Every char of a Rust string is a Unicode scalar value, so the
            // graphql-js `Invalid character` branch (lone surrogates) cannot
            // occur here.
            return Err(GraphQLError::syntax(if code == b'\'' {
                "Unexpected single quote character ('), did you mean to use a double quote (\")?"
                    .to_owned()
            } else {
                format!(
                    "Unexpected character: {}.",
                    self.print_code_point_at(position)
                )
            }));
        }
        Ok(Token::new(TokenKind::Eof, body_length, body_length, None))
    }

    fn read_next_coordinate_token(&self, position: usize) -> Result<Token<'s>, Box<GraphQLError>> {
        let body = self.source.as_bytes();
        if position < body.len() {
            let code = body[position];
            match code {
                b'.' => return Ok(self.punct(TokenKind::Dot, position, 1)),
                b'(' => return Ok(self.punct(TokenKind::ParenL, position, 1)),
                b')' => return Ok(self.punct(TokenKind::ParenR, position, 1)),
                b':' => return Ok(self.punct(TokenKind::Colon, position, 1)),
                b'@' => return Ok(self.punct(TokenKind::At, position, 1)),
                _ => {}
            }
            if is_name_start(code) {
                return Ok(self.read_name(position));
            }
            return Err(GraphQLError::syntax(format!(
                "Invalid character: {}.",
                self.print_code_point_at(position)
            )));
        }
        Ok(Token::new(TokenKind::Eof, body.len(), body.len(), None))
    }

    /// graphql-js `printCodePointAt`.
    fn print_code_point_at(&self, location: usize) -> String {
        let Some(c) = self.source.get(location..).and_then(|s| s.chars().next()) else {
            return TokenKind::Eof.as_str().to_owned();
        };
        let code = c as u32;
        if (0x0020..=0x007e).contains(&code) {
            if c == '"' {
                "'\"'".to_owned()
            } else {
                format!("\"{c}\"")
            }
        } else {
            format!("U+{code:04X}")
        }
    }

    fn read_comment(&self, start: usize) -> Token<'s> {
        let body = self.source.as_bytes();
        let mut position = start + 1;
        while position < body.len() {
            let code = body[position];
            if code == b'\n' || code == b'\r' {
                break;
            }
            position += 1;
        }
        Token::new(
            TokenKind::Comment,
            start,
            position,
            Some(Cow::Borrowed(&self.source[start + 1..position])),
        )
    }

    fn read_number(&self, start: usize, first_code: u8) -> Result<Token<'s>, Box<GraphQLError>> {
        let mut position = start;
        let mut code = first_code;
        let mut is_float = false;
        if code == b'-' {
            position += 1;
            code = self.at(position);
        }
        if code == b'0' {
            position += 1;
            code = self.at(position);
            if code.is_ascii_digit() {
                return Err(GraphQLError::syntax(format!(
                    "Invalid number, unexpected digit after 0: {}.",
                    self.print_code_point_at(position)
                )));
            }
        } else {
            position = self.read_digits(position, code)?;
            code = self.at(position);
        }
        if code == b'.' {
            is_float = true;
            position += 1;
            code = self.at(position);
            position = self.read_digits(position, code)?;
            code = self.at(position);
        }
        if code == b'E' || code == b'e' {
            is_float = true;
            position += 1;
            code = self.at(position);
            if code == b'+' || code == b'-' {
                position += 1;
                code = self.at(position);
            }
            position = self.read_digits(position, code)?;
            code = self.at(position);
        }
        if code == b'.' || is_name_start(code) {
            return Err(GraphQLError::syntax(format!(
                "Invalid number, expected digit but got: {}.",
                self.print_code_point_at(position)
            )));
        }
        Ok(Token::new(
            if is_float {
                TokenKind::Float
            } else {
                TokenKind::Int
            },
            start,
            position,
            Some(Cow::Borrowed(&self.source[start..position])),
        ))
    }

    fn read_digits(&self, start: usize, first_code: u8) -> Result<usize, Box<GraphQLError>> {
        if !first_code.is_ascii_digit() {
            return Err(GraphQLError::syntax(format!(
                "Invalid number, expected digit but got: {}.",
                self.print_code_point_at(start)
            )));
        }
        let mut position = start + 1;
        while self.at(position).is_ascii_digit() {
            position += 1;
        }
        Ok(position)
    }

    fn read_string(&self, start: usize) -> Result<Token<'s>, Box<GraphQLError>> {
        let body = self.source.as_bytes();
        let mut position = start + 1;
        let mut chunk_start = position;
        let mut value: Option<String> = None;
        while position < body.len() {
            let code = body[position];
            if code == b'"' {
                let value = match value {
                    None => Cow::Borrowed(&self.source[chunk_start..position]),
                    Some(mut v) => {
                        v.push_str(&self.source[chunk_start..position]);
                        Cow::Owned(v)
                    }
                };
                return Ok(Token::new(
                    TokenKind::String,
                    start,
                    position + 1,
                    Some(value),
                ));
            }
            if code == b'\\' {
                let v = value.get_or_insert_with(String::new);
                v.push_str(&self.source[chunk_start..position]);
                let (c, size) = if self.at(position + 1) == b'u' {
                    if self.at(position + 2) == b'{' {
                        self.read_escaped_unicode_variable_width(position)?
                    } else {
                        self.read_escaped_unicode_fixed_width(position)?
                    }
                } else {
                    self.read_escaped_character(position)?
                };
                v.push(c);
                // Escapes that succeed are ASCII, so `size` is in bytes too.
                position += size;
                chunk_start = position;
                continue;
            }
            if code == b'\n' || code == b'\r' {
                break;
            }
            position += 1;
        }
        Err(GraphQLError::syntax("Unterminated string."))
    }

    fn read_escaped_unicode_variable_width(
        &self,
        position: usize,
    ) -> Result<(char, usize), Box<GraphQLError>> {
        let mut point: i32 = 0;
        let mut size = 3;
        // Only ASCII is consumed before the loop ends (any other char is not a
        // hex digit), so byte offsets equal UTF-16 offsets here.
        while size < 12 {
            let code = self.at(position + size);
            size += 1;
            if code == b'}' {
                if size < 5 || !is_unicode_scalar_value(point) {
                    break;
                }
                return Ok((char::from_u32(point as u32).unwrap(), size));
            }
            point = point.wrapping_shl(4) | read_hex_digit(code);
            if point < 0 {
                break;
            }
        }
        Err(GraphQLError::syntax(format!(
            "Invalid Unicode escape sequence: \"{}\".",
            utf16_slice(self.source, position, size)
        )))
    }

    fn read_escaped_unicode_fixed_width(
        &self,
        position: usize,
    ) -> Result<(char, usize), Box<GraphQLError>> {
        let code = self.read_16_bit_hex_code(position + 2);
        if is_unicode_scalar_value(code) {
            return Ok((char::from_u32(code as u32).unwrap(), 6));
        }
        if is_leading_surrogate(code)
            && self.at(position + 6) == b'\\'
            && self.at(position + 7) == b'u'
        {
            let trailing_code = self.read_16_bit_hex_code(position + 8);
            if is_trailing_surrogate(trailing_code) {
                let point = 0x10000 + ((code - 0xd800) << 10) + (trailing_code - 0xdc00);
                return Ok((char::from_u32(point as u32).unwrap(), 12));
            }
        }
        Err(GraphQLError::syntax(format!(
            "Invalid Unicode escape sequence: \"{}\".",
            utf16_slice(self.source, position, 6)
        )))
    }

    fn read_16_bit_hex_code(&self, position: usize) -> i32 {
        // A non-ASCII byte is not a hex digit and makes the result negative,
        // so misaligned offsets after it never matter.
        (read_hex_digit(self.at(position)) << 12)
            | (read_hex_digit(self.at(position + 1)) << 8)
            | (read_hex_digit(self.at(position + 2)) << 4)
            | read_hex_digit(self.at(position + 3))
    }

    fn read_escaped_character(&self, position: usize) -> Result<(char, usize), Box<GraphQLError>> {
        let c = match self.at(position + 1) {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{0008}',
            b'f' => '\u{000c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            _ => {
                return Err(GraphQLError::syntax(format!(
                    "Invalid character escape sequence: \"{}\".",
                    utf16_slice(self.source, position, 2)
                )))
            }
        };
        Ok((c, 2))
    }

    fn read_block_string(&self, start: usize) -> Result<Token<'s>, Box<GraphQLError>> {
        let body = self.source.as_bytes();
        let mut position = start + 3;
        let mut chunk_start = position;
        let mut current_line = String::new();
        let mut block_lines: Vec<String> = Vec::new();
        while position < body.len() {
            let code = body[position];
            if code == b'"' && self.at(position + 1) == b'"' && self.at(position + 2) == b'"' {
                current_line.push_str(&self.source[chunk_start..position]);
                block_lines.push(current_line);
                let value = dedent_block_string_lines(&block_lines).join("\n");
                return Ok(Token::new(
                    TokenKind::BlockString,
                    start,
                    position + 3,
                    Some(Cow::Owned(value)),
                ));
            }
            if code == b'\\'
                && self.at(position + 1) == b'"'
                && self.at(position + 2) == b'"'
                && self.at(position + 3) == b'"'
            {
                current_line.push_str(&self.source[chunk_start..position]);
                chunk_start = position + 1;
                position += 4;
                continue;
            }
            if code == b'\n' || code == b'\r' {
                current_line.push_str(&self.source[chunk_start..position]);
                block_lines.push(std::mem::take(&mut current_line));
                if code == b'\r' && self.at(position + 1) == b'\n' {
                    position += 2;
                } else {
                    position += 1;
                }
                chunk_start = position;
                continue;
            }
            position += 1;
        }
        Err(GraphQLError::syntax("Unterminated string."))
    }

    fn read_name(&self, start: usize) -> Token<'s> {
        let body = self.source.as_bytes();
        let mut position = start + 1;
        while position < body.len() && CLASS[body[position] as usize] == NAME {
            position += 1;
        }
        Token::new(
            TokenKind::Name,
            start,
            position,
            Some(Cow::Borrowed(&self.source[start..position])),
        )
    }
}

fn is_unicode_scalar_value(code: i32) -> bool {
    (0x0000..=0xd7ff).contains(&code) || (0xe000..=0x10ffff).contains(&code)
}

fn is_leading_surrogate(code: i32) -> bool {
    (0xd800..=0xdbff).contains(&code)
}

fn is_trailing_surrogate(code: i32) -> bool {
    (0xdc00..=0xdfff).contains(&code)
}

fn read_hex_digit(code: u8) -> i32 {
    match code {
        b'0'..=b'9' => (code - b'0') as i32,
        b'A'..=b'F' => (code - b'A' + 10) as i32,
        b'a'..=b'f' => (code - b'a' + 10) as i32,
        _ => -1,
    }
}

const OTHER: u8 = 0;
const IGNORED: u8 = 1;
const NAME: u8 = 2;

/// Byte classes for the two hottest loops: `IGNORED` for tab, space, comma
/// and line feed (carriage returns and the BOM take the slow path), `NAME`
/// for graphql-js `isNameContinue`.
static CLASS: [u8; 256] = {
    let mut table = [OTHER; 256];
    table[b'\t' as usize] = IGNORED;
    table[b' ' as usize] = IGNORED;
    table[b',' as usize] = IGNORED;
    table[b'\n' as usize] = IGNORED;
    let mut c = 0;
    while c < 256 {
        let b = c as u8;
        if is_name_continue(b) {
            table[c] = NAME;
        }
        c += 1;
    }
    table
};

/// graphql-js `isNameStart`.
pub(crate) fn is_name_start(code: u8) -> bool {
    code.is_ascii_alphabetic() || code == b'_'
}

/// graphql-js `isNameContinue`.
pub(crate) const fn is_name_continue(code: u8) -> bool {
    code.is_ascii_alphanumeric() || code == b'_'
}
