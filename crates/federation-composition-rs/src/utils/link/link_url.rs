//! Port of `src/utils/link/link-url.ts`.

use std::fmt;

use crate::Error;

/// `VERSION_MATCH = /v(\d{1,3})\.(\d{1,4})/i` (unanchored).
// TS: the regex; the port matches it by hand (`version_match`).
#[allow(dead_code)]
pub const VERSION_MATCH: &str = r"(?i)v(\d{1,3})\.(\d{1,4})";

/// `VERSION_MATCH.exec(text)`: the major and minor digit groups of the leftmost match.
/// NOTE(port): a hand-written matcher for the regex above (the crate has no regex
/// dependency). `\d{1,3}` followed by `\.` only matches a run of one to three digits that is
/// followed by a dot (backtracking cannot help, the next character is a digit); `\d{1,4}`
/// takes up to four digits greedily.
fn version_match(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    for start in 0..bytes.len() {
        if bytes[start] != b'v' && bytes[start] != b'V' {
            continue;
        }
        let major_start = start + 1;
        let major_len = bytes[major_start..].iter().take_while(|b| b.is_ascii_digit()).count();
        if major_len == 0 || major_len > 3 {
            continue;
        }
        let dot = major_start + major_len;
        if bytes.get(dot) != Some(&b'.') {
            continue;
        }
        let minor_start = dot + 1;
        let minor_len = bytes[minor_start..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
            .min(4);
        if minor_len == 0 {
            continue;
        }
        return Some((&text[major_start..dot], &text[minor_start..minor_start + minor_len]));
    }
    None
}

/// `parseVersion(version)`: `[major, minor]`, or `[-1, -1]` when there is no match.
fn parse_version(version: Option<&str>) -> (i64, i64) {
    if let Some((major, minor)) = version.and_then(version_match) {
        // `Number(major)`: one to four decimal digits.
        return (major.parse().unwrap_or(-1), minor.parse().unwrap_or(-1));
    }
    (-1, -1)
}

/// The argument of the overloaded `supports(...)` of [`FederatedLinkUrl::supports`] and
/// [`FederatedLink::supports`](crate::FederatedLink::supports).
#[derive(Clone, Copy, Debug)]
pub enum SupportsVersion<'a> {
    /// `supports(version: string)`.
    Str(&'a str),
    /// `supports(major: number, minor: number)`.
    MajorMinor(i64, i64),
    /// `supports(version: FederatedLinkUrl)`.
    Url(&'a FederatedLinkUrl),
    /// `supports(null)`.
    Null,
}

/// The parsed `url` of a `@link`: upstream's `FederatedLinkUrl`.
///
/// For `https://specs.apollo.dev/federation/v2.3` the identity is
/// `https://specs.apollo.dev/federation`, the name `federation` and the version `v2.3`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FederatedLinkUrl {
    pub(crate) identity: String,
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
    /// -1 if no version is set.
    major: i64,
    minor: i64,
}

impl FederatedLinkUrl {
    /// `new FederatedLinkUrl(identity, name, version)`.
    pub fn new(identity: String, name: Option<String>, version: Option<String>) -> Self {
        let (major, minor) = parse_version(version.as_deref());
        FederatedLinkUrl {
            identity,
            name,
            version,
            major,
            minor,
        }
    }

    /// The spec's identity (`identity`): the URL without the version.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The spec's name (`name`), the last path segment before the version.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The version (`version`), e.g. `v2.3`.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// `FederatedLinkUrl.fromUrl(urlSource)`. `Err` (upstream throws `TypeError: Invalid URL`,
    /// from WHATWG URL parsing as `new URL` does) for inputs such as `"spec.example.com"`.
    pub fn from_url(url_source: &str) -> Result<FederatedLinkUrl, Error> {
        let url = whatwg_url::Url::parse(url_source)?;
        let parts: Vec<&str> = url.pathname.split('/').filter(|p| !p.is_empty()).collect();
        let version_or_name = parts.last().copied();
        let origin = if url.protocol == "file:" {
            "file://".to_string()
        } else {
            url.origin.clone()
        };
        if let Some(version_or_name) = version_or_name {
            if version_match(version_or_name).is_some() {
                let maybe_name = if parts.len() >= 2 {
                    Some(parts[parts.len() - 2])
                } else {
                    None
                };
                let identity = match maybe_name {
                    Some(_) => format!("{}/{}", origin, parts[..parts.len() - 1].join("/")),
                    None => origin,
                };
                return Ok(FederatedLinkUrl::new(
                    identity,
                    maybe_name.map(str::to_string),
                    Some(version_or_name.to_string()),
                ));
            }
            return Ok(FederatedLinkUrl::new(
                format!("{}/{}", origin, parts.join("/")),
                Some(version_or_name.to_string()),
                None,
            ));
        }
        Ok(FederatedLinkUrl::new(origin, None, None))
    }

    /// `supports(...)`: whether this version supports another version.
    pub fn supports(&self, version: SupportsVersion<'_>) -> bool {
        let (major, minor) = match version {
            SupportsVersion::Str(version) => parse_version(Some(version)),
            SupportsVersion::MajorMinor(major, minor) => (major, minor),
            SupportsVersion::Url(other) => {
                // check that it is the same spec
                if other.identity != self.identity {
                    return false;
                }
                (other.major, other.minor)
            }
            // handles null case
            SupportsVersion::Null => return self.version.is_none(),
        };
        self.is_compatible_version(major, minor)
    }

    fn is_compatible_version(&self, major: i64, minor: i64) -> bool {
        if self.major == major {
            if self.major == 0 {
                return self.minor == minor;
            }
            return self.minor >= minor;
        }
        false
    }
}

impl fmt::Display for FederatedLinkUrl {
    /// `toString()`: the identity, followed by `/` and the version when there is one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) if !v.is_empty() => write!(f, "{}/{}", self.identity, v),
            _ => f.write_str(&self.identity),
        }
    }
}

/// The part of the WHATWG URL parser (`new URL(input)` without a base) that `fromUrl` needs:
/// `protocol`, `origin` and `pathname`, and the same failures (`TypeError: Invalid URL`).
///
/// NOTE(port): JS gets this from the engine. Ported: scheme parsing, special and non-special
/// authorities (credentials, ports, default ports), host parsing (percent-decoding, forbidden
/// code points, IPv4 numbers in decimal/octal/hex, IPv6 with compression), path segments with
/// dot-segment removal and the path percent-encode set, opaque paths, query and fragment
/// stripping, and input trimming. Approximated: non-ASCII domains are lowercased and Punycode
/// encoded per label, without the rest of UTS #46 (mapping, normalization, `xn--` label
/// validation); Windows drive letters in `file:` URLs are not special-cased; the `blob:` origin
/// is `"null"` like other non-special schemes.
pub(crate) mod whatwg_url {
    use crate::utils::js::{JsError, JsResult};

    pub struct Url {
        /// `url.protocol`, e.g. `"https:"`.
        pub protocol: String,
        /// `url.origin`.
        pub origin: String,
        /// `url.pathname`.
        pub pathname: String,
    }

    fn invalid() -> JsError {
        JsError::type_error("Invalid URL")
    }

    fn default_port(scheme: &str) -> Option<u32> {
        match scheme {
            "http" | "ws" => Some(80),
            "https" | "wss" => Some(443),
            "ftp" => Some(21),
            _ => None,
        }
    }

    fn is_special(scheme: &str) -> bool {
        matches!(scheme, "http" | "https" | "ws" | "wss" | "ftp" | "file")
    }

    impl Url {
        pub fn parse(input: &str) -> JsResult<Url> {
            // Strip leading and trailing C0 control or space, remove ASCII tab or newline.
            let trimmed = input.trim_matches(|c: char| c <= '\u{20}');
            let input: Vec<char> = trimmed.chars().filter(|c| !matches!(c, '\t' | '\n' | '\r')).collect();

            // Scheme.
            if !input.first().is_some_and(|c| c.is_ascii_alphabetic()) {
                return Err(invalid());
            }
            let mut i = 0;
            while i < input.len() && (input[i].is_ascii_alphanumeric() || matches!(input[i], '+' | '-' | '.')) {
                i += 1;
            }
            if input.get(i) != Some(&':') {
                // "no scheme" state without a base URL.
                return Err(invalid());
            }
            let scheme: String = input[..i].iter().collect::<String>().to_ascii_lowercase();
            let rest = &input[i + 1..];
            let special = is_special(&scheme);
            let is_slash = |c: char| c == '/' || (special && c == '\\');

            let (host, port, path_input): (Option<String>, Option<u32>, &[char]) = if scheme == "file" {
                if rest.len() >= 2 && is_slash(rest[0]) && is_slash(rest[1]) {
                    let after = &rest[2..];
                    let end = after
                        .iter()
                        .position(|c| matches!(c, '/' | '\\' | '?' | '#'))
                        .unwrap_or(after.len());
                    let buffer: String = after[..end].iter().collect();
                    let host = if buffer.is_empty() {
                        String::new()
                    } else {
                        let host = parse_host(&buffer, true)?;
                        if host == "localhost" {
                            String::new()
                        } else {
                            host
                        }
                    };
                    (Some(host), None, &after[end..])
                } else {
                    (Some(String::new()), None, rest)
                }
            } else if special {
                // Special authority ignore slashes.
                let start = rest.iter().position(|c| *c != '/' && *c != '\\').unwrap_or(rest.len());
                let after = &rest[start..];
                let end = after
                    .iter()
                    .position(|c| matches!(c, '/' | '\\' | '?' | '#'))
                    .unwrap_or(after.len());
                let (host, port) = parse_authority(&after[..end], &scheme, true)?;
                (Some(host), port, &after[end..])
            } else if rest.len() >= 2 && rest[0] == '/' && rest[1] == '/' {
                let after = &rest[2..];
                let end = after
                    .iter()
                    .position(|c| matches!(c, '/' | '?' | '#'))
                    .unwrap_or(after.len());
                let (host, port) = parse_authority(&after[..end], &scheme, false)?;
                (Some(host), port, &after[end..])
            } else {
                (None, None, rest)
            };

            let path_end = path_input
                .iter()
                .position(|c| matches!(c, '?' | '#'))
                .unwrap_or(path_input.len());
            let path_input = &path_input[..path_end];

            let pathname = if host.is_none() && path_input.first() != Some(&'/') {
                // Opaque path.
                let mut out = String::new();
                for c in path_input {
                    percent_encode_char(*c, |c| (c as u32) < 0x20 || (c as u32) > 0x7e, &mut out);
                }
                out
            } else {
                let mut segments: Vec<String> = Vec::new();
                let mut input = path_input;
                if input.first().is_some_and(|c| is_slash(*c)) {
                    input = &input[1..];
                }
                if !(input.is_empty() && !special && host.is_some() && path_input.is_empty()) {
                    let mut buffer = String::new();
                    let mut idx = 0;
                    loop {
                        let c = input.get(idx).copied();
                        let at_separator = c.is_none_or(is_slash);
                        if at_separator {
                            if is_double_dot(&buffer) {
                                segments.pop();
                                if c.is_none() {
                                    segments.push(String::new());
                                }
                            } else if is_single_dot(&buffer) {
                                if c.is_none() {
                                    segments.push(String::new());
                                }
                            } else {
                                segments.push(std::mem::take(&mut buffer));
                            }
                            buffer.clear();
                            if c.is_none() {
                                break;
                            }
                        } else if let Some(c) = c {
                            percent_encode_char(c, in_path_percent_encode_set, &mut buffer);
                        }
                        idx += 1;
                    }
                }
                segments.iter().map(|s| format!("/{s}")).collect()
            };

            let origin = match (&host, special && scheme != "file") {
                (Some(host), true) => match port {
                    Some(port) => format!("{scheme}://{host}:{port}"),
                    None => format!("{scheme}://{host}"),
                },
                _ => "null".to_string(),
            };

            Ok(Url {
                protocol: format!("{scheme}:"),
                origin,
                pathname,
            })
        }
    }

    fn is_single_dot(s: &str) -> bool {
        s == "." || s.eq_ignore_ascii_case("%2e")
    }

    fn is_double_dot(s: &str) -> bool {
        let s = s.to_ascii_lowercase();
        matches!(s.as_str(), ".." | ".%2e" | "%2e." | "%2e%2e")
    }

    fn in_path_percent_encode_set(c: char) -> bool {
        let n = c as u32;
        !(0x20..=0x7e).contains(&n) || matches!(c, ' ' | '"' | '#' | '<' | '>' | '?' | '^' | '`' | '{' | '}')
    }

    fn percent_encode_char(c: char, in_set: impl Fn(char) -> bool, out: &mut String) {
        if in_set(c) {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        } else {
            out.push(c);
        }
    }

    /// The authority state: credentials, host and port.
    fn parse_authority(authority: &[char], scheme: &str, special: bool) -> JsResult<(String, Option<u32>)> {
        let host_port: &[char] = match authority.iter().rposition(|c| *c == '@') {
            Some(at) => {
                let host_port = &authority[at + 1..];
                if host_port.is_empty() {
                    return Err(invalid());
                }
                host_port
            }
            None => authority,
        };
        // The port starts at the first `:` outside brackets.
        let mut in_brackets = false;
        let mut colon = None;
        for (i, c) in host_port.iter().enumerate() {
            match c {
                '[' => in_brackets = true,
                ']' => in_brackets = false,
                ':' if !in_brackets => {
                    colon = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let (host_buffer, port_buffer) = match colon {
            Some(i) => (&host_port[..i], Some(&host_port[i + 1..])),
            None => (host_port, None),
        };
        if host_buffer.is_empty() {
            if special || port_buffer.is_some() {
                return Err(invalid());
            }
            return Ok((String::new(), None));
        }
        let host = parse_host(&host_buffer.iter().collect::<String>(), special)?;
        let mut port = None;
        if let Some(port_buffer) = port_buffer {
            if !port_buffer.iter().all(|c| c.is_ascii_digit()) {
                return Err(invalid());
            }
            if !port_buffer.is_empty() {
                let digits = port_buffer.iter().collect::<String>();
                let digits = digits.trim_start_matches('0');
                if digits.len() > 5 {
                    return Err(invalid());
                }
                let value: u32 = if digits.is_empty() {
                    0
                } else {
                    digits.parse().map_err(|_| invalid())?
                };
                if value > 65535 {
                    return Err(invalid());
                }
                if default_port(scheme) != Some(value) {
                    port = Some(value);
                }
            }
        }
        Ok((host, port))
    }

    /// The host parser. `special` is "not opaque".
    fn parse_host(input: &str, special: bool) -> JsResult<String> {
        if let Some(inner) = input.strip_prefix('[') {
            let inner = inner.strip_suffix(']').ok_or_else(invalid)?;
            let address = parse_ipv6(inner).ok_or_else(invalid)?;
            return Ok(format!("[{}]", serialize_ipv6(&address)));
        }
        if !special {
            // Opaque host.
            if input.chars().any(|c| {
                matches!(
                    c,
                    '\0' | '\t'
                        | '\n'
                        | '\r'
                        | ' '
                        | '#'
                        | '/'
                        | ':'
                        | '<'
                        | '>'
                        | '?'
                        | '@'
                        | '['
                        | '\\'
                        | ']'
                        | '^'
                        | '|'
                )
            }) {
                return Err(invalid());
            }
            let mut out = String::new();
            for c in input.chars() {
                percent_encode_char(c, |c| (c as u32) < 0x20 || (c as u32) > 0x7e, &mut out);
            }
            return Ok(out);
        }
        let domain = String::from_utf8_lossy(&percent_decode(input)).into_owned();
        let ascii_domain = domain_to_ascii(&domain);
        if ascii_domain.is_empty() {
            return Err(invalid());
        }
        if ascii_domain.chars().any(|c| {
            (c as u32) <= 0x1f
                || c == '\u{7f}'
                || matches!(
                    c,
                    ' ' | '#' | '%' | '/' | ':' | '<' | '>' | '?' | '@' | '[' | '\\' | ']' | '^' | '|'
                )
        }) {
            return Err(invalid());
        }
        if ends_in_a_number(&ascii_domain) {
            let address = parse_ipv4(&ascii_domain).ok_or_else(invalid)?;
            return Ok(format!(
                "{}.{}.{}.{}",
                address >> 24,
                (address >> 16) & 0xff,
                (address >> 8) & 0xff,
                address & 0xff
            ));
        }
        Ok(ascii_domain)
    }

    fn percent_decode(input: &str) -> Vec<u8> {
        let bytes = input.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%'
                && i + 2 < bytes.len()
                && bytes[i + 1].is_ascii_hexdigit()
                && bytes[i + 2].is_ascii_hexdigit()
            {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00");
                out.push(u8::from_str_radix(hex, 16).unwrap_or(0));
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        out
    }

    /// Lowercases and Punycode-encodes non-ASCII labels (see the module NOTE).
    fn domain_to_ascii(domain: &str) -> String {
        domain
            .to_lowercase()
            .split('.')
            .map(|label| {
                if label.is_ascii() {
                    label.to_string()
                } else {
                    format!("xn--{}", punycode_encode(label))
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    /// RFC 3492 Punycode encoding of one label.
    fn punycode_encode(label: &str) -> String {
        const BASE: u32 = 36;
        const T_MIN: u32 = 1;
        const T_MAX: u32 = 26;
        const SKEW: u32 = 38;
        const DAMP: u32 = 700;
        fn adapt(mut delta: u32, num_points: u32, first: bool) -> u32 {
            delta /= if first { DAMP } else { 2 };
            delta += delta / num_points;
            let mut k = 0;
            while delta > ((BASE - T_MIN) * T_MAX) / 2 {
                delta /= BASE - T_MIN;
                k += BASE;
            }
            k + (BASE - T_MIN + 1) * delta / (delta + SKEW)
        }
        fn digit(d: u32) -> char {
            (if d < 26 { b'a' + d as u8 } else { b'0' + (d - 26) as u8 }) as char
        }
        let input: Vec<u32> = label.chars().map(|c| c as u32).collect();
        let mut output: String = label.chars().filter(char::is_ascii).collect();
        let basic = output.len() as u32;
        let mut handled = basic;
        if basic > 0 {
            output.push('-');
        }
        let mut n: u32 = 128;
        let mut delta: u32 = 0;
        let mut bias: u32 = 72;
        while (handled as usize) < input.len() {
            let m = input.iter().copied().filter(|c| *c >= n).min().unwrap_or(n);
            delta = delta.saturating_add((m - n).saturating_mul(handled + 1));
            n = m;
            for &c in &input {
                if c < n {
                    delta = delta.saturating_add(1);
                }
                if c == n {
                    let mut q = delta;
                    let mut k = BASE;
                    loop {
                        let t = if k <= bias {
                            T_MIN
                        } else if k >= bias + T_MAX {
                            T_MAX
                        } else {
                            k - bias
                        };
                        if q < t {
                            break;
                        }
                        output.push(digit(t + (q - t) % (BASE - t)));
                        q = (q - t) / (BASE - t);
                        k += BASE;
                    }
                    output.push(digit(q));
                    bias = adapt(delta, handled + 1, handled == basic);
                    delta = 0;
                    handled += 1;
                }
            }
            delta += 1;
            n += 1;
        }
        output
    }

    fn ends_in_a_number(input: &str) -> bool {
        let mut parts: Vec<&str> = input.split('.').collect();
        if parts.last() == Some(&"") {
            if parts.len() == 1 {
                return false;
            }
            parts.pop();
        }
        let last = parts.last().copied().unwrap_or("");
        if !last.is_empty() && last.bytes().all(|b| b.is_ascii_digit()) {
            return true;
        }
        parse_ipv4_number(last).is_some()
    }

    fn parse_ipv4_number(input: &str) -> Option<u64> {
        if input.is_empty() {
            return None;
        }
        let (digits, radix) = if let Some(hex) = input.strip_prefix("0x").or_else(|| input.strip_prefix("0X")) {
            (hex, 16)
        } else if input.len() >= 2 && input.starts_with('0') {
            (&input[1..], 8)
        } else {
            (input, 10)
        };
        if digits.is_empty() {
            return Some(0);
        }
        if !digits.chars().all(|c| c.is_digit(radix)) {
            return None;
        }
        // Values that do not fit are larger than any valid address part.
        Some(u64::from_str_radix(digits, radix).unwrap_or(u64::MAX))
    }

    fn parse_ipv4(input: &str) -> Option<u32> {
        let mut parts: Vec<&str> = input.split('.').collect();
        if parts.last() == Some(&"") && parts.len() > 1 {
            parts.pop();
        }
        if parts.len() > 4 {
            return None;
        }
        let numbers: Vec<u64> = parts.iter().map(|p| parse_ipv4_number(p)).collect::<Option<_>>()?;
        let (last, init) = numbers.split_last()?;
        if init.iter().any(|n| *n > 255) {
            return None;
        }
        if *last >= 256u64.pow(5 - numbers.len() as u32) {
            return None;
        }
        let mut ipv4 = *last;
        for (i, n) in init.iter().enumerate() {
            ipv4 += n * 256u64.pow(3 - i as u32);
        }
        Some(ipv4 as u32)
    }

    fn parse_ipv6(input: &str) -> Option<[u16; 8]> {
        let input: Vec<char> = input.chars().collect();
        let mut address = [0u16; 8];
        let mut piece_index = 0usize;
        let mut compress: Option<usize> = None;
        let mut pointer = 0usize;
        let c = |p: usize| input.get(p).copied();
        if c(pointer) == Some(':') {
            if c(pointer + 1) != Some(':') {
                return None;
            }
            pointer += 2;
            piece_index += 1;
            compress = Some(piece_index);
        }
        while c(pointer).is_some() {
            if piece_index == 8 {
                return None;
            }
            if c(pointer) == Some(':') {
                if compress.is_some() {
                    return None;
                }
                pointer += 1;
                piece_index += 1;
                compress = Some(piece_index);
                continue;
            }
            let mut value: u32 = 0;
            let mut length = 0;
            while length < 4 && c(pointer).is_some_and(|ch| ch.is_ascii_hexdigit()) {
                value = value * 0x10 + c(pointer)?.to_digit(16)?;
                pointer += 1;
                length += 1;
            }
            if c(pointer) == Some('.') {
                if length == 0 {
                    return None;
                }
                pointer -= length;
                if piece_index > 6 {
                    return None;
                }
                let mut numbers_seen = 0;
                while c(pointer).is_some() {
                    let mut ipv4_piece: Option<u32> = None;
                    if numbers_seen > 0 {
                        if c(pointer) == Some('.') && numbers_seen < 4 {
                            pointer += 1;
                        } else {
                            return None;
                        }
                    }
                    if !c(pointer).is_some_and(|ch| ch.is_ascii_digit()) {
                        return None;
                    }
                    while let Some(d) = c(pointer).and_then(|ch| ch.to_digit(10)) {
                        ipv4_piece = match ipv4_piece {
                            None => Some(d),
                            Some(0) => return None,
                            Some(p) => Some(p * 10 + d),
                        };
                        if ipv4_piece? > 255 {
                            return None;
                        }
                        pointer += 1;
                    }
                    address[piece_index] = (address[piece_index] as u32 * 0x100 + ipv4_piece?) as u16;
                    numbers_seen += 1;
                    if numbers_seen == 2 || numbers_seen == 4 {
                        piece_index += 1;
                    }
                }
                if numbers_seen != 4 {
                    return None;
                }
                break;
            } else if c(pointer) == Some(':') {
                pointer += 1;
                c(pointer)?;
            } else if c(pointer).is_some() {
                return None;
            }
            address[piece_index] = value as u16;
            piece_index += 1;
        }
        if let Some(compress) = compress {
            let mut swaps = piece_index - compress;
            piece_index = 7;
            while piece_index != 0 && swaps > 0 {
                address.swap(piece_index, compress + swaps - 1);
                piece_index -= 1;
                swaps -= 1;
            }
        } else if piece_index != 8 {
            return None;
        }
        Some(address)
    }

    fn serialize_ipv6(address: &[u16; 8]) -> String {
        // The first longest run of two or more zero pieces is compressed.
        let mut compress: Option<(usize, usize)> = None;
        let mut i = 0;
        while i < 8 {
            if address[i] == 0 {
                let start = i;
                while i < 8 && address[i] == 0 {
                    i += 1;
                }
                let len = i - start;
                if len > 1 && compress.is_none_or(|(_, l)| len > l) {
                    compress = Some((start, len));
                }
            } else {
                i += 1;
            }
        }
        let mut out = String::new();
        let mut ignore0 = false;
        for (index, piece) in address.iter().enumerate() {
            if ignore0 && *piece == 0 {
                continue;
            }
            ignore0 = false;
            if compress.is_some_and(|(start, _)| start == index) {
                out.push_str(if index == 0 { "::" } else { ":" });
                ignore0 = true;
                continue;
            }
            out.push_str(&format!("{piece:x}"));
            if index != 7 {
                out.push(':');
            }
        }
        out
    }

    #[cfg(test)]
    mod tests {
        use super::Url;

        /// Expected values generated with Node 24 (`new URL(input)`).
        #[test]
        fn matches_node() {
            /// `(protocol, origin, pathname)`, or `None` when `new URL` throws.
            type Parsed<'a> = Option<(&'a str, &'a str, &'a str)>;
            let cases: &[(&str, Parsed)] = &[
                (
                    "https://a.com/a^b|c\"d<e>f`g{h}i j~k%41/é",
                    Some((
                        "https:",
                        "https://a.com",
                        "/a%5Eb|c%22d%3Ce%3Ef%60g%7Bh%7Di%20j~k%41/%C3%A9",
                    )),
                ),
                (
                    "https://a.com/x/./y/../z/%2e%2E/w/.",
                    Some(("https:", "https://a.com", "/x/w/")),
                ),
                ("https:a.com\\b\\c", Some(("https:", "https://a.com", "/b/c"))),
                ("https://A.CoM:443/x", Some(("https:", "https://a.com", "/x"))),
                ("https://a.com:0080/x", Some(("https:", "https://a.com:80", "/x"))),
                ("http://a.com:80/x", Some(("http:", "http://a.com", "/x"))),
                ("https://0x7f.1/", Some(("https:", "https://127.0.0.1", "/"))),
                ("https://[::1]:8080/", Some(("https:", "https://[::1]:8080", "/"))),
                (
                    "https://[1:0:0:2:0:0:0:3]/",
                    Some(("https:", "https://[1:0:0:2::3]", "/")),
                ),
                ("https://ex%41mple.com/", Some(("https:", "https://example.com", "/"))),
                ("foo://host/a/b", Some(("foo:", "null", "/a/b"))),
                ("urn:a/b/c", Some(("urn:", "null", "a/b/c"))),
                ("file://host/a/b", Some(("file:", "null", "/a/b"))),
                ("file:/a", Some(("file:", "null", "/a"))),
                ("file:a/b", Some(("file:", "null", "/a/b"))),
                (
                    "file:///extensions/permissions",
                    Some(("file:", "null", "/extensions/permissions")),
                ),
                ("https://u:p@h.com/", Some(("https:", "https://h.com", "/"))),
                ("https:///a.com/b", Some(("https:", "https://a.com", "/b"))),
                ("https://a.com.", Some(("https:", "https://a.com.", "/"))),
                ("https://1.2.3.4.", Some(("https:", "https://1.2.3.4", "/"))),
                ("https://999.1.1.1", None),
                ("https://a b.com", None),
                ("https://a.com:65536", None),
                ("https://1.2.3", Some(("https:", "https://1.2.0.3", "/"))),
                (
                    "https://[::ffff:1.2.3.4]/",
                    Some(("https:", "https://[::ffff:102:304]", "/")),
                ),
                ("HTTPS://x.com/V1.0", Some(("https:", "https://x.com", "/V1.0"))),
                ("  https://a.com/\t x  ", Some(("https:", "https://a.com", "/%20x"))),
                ("https://bücher.de/", Some(("https:", "https://xn--bcher-kva.de", "/"))),
                (
                    "https://spec.example.com/mySchema/v0.1?q=v#frag",
                    Some(("https:", "https://spec.example.com", "/mySchema/v0.1")),
                ),
                ("spec.example.com", None),
                ("https://", None),
                ("", None),
            ];
            for (input, expected) in cases {
                let actual = Url::parse(input).ok().map(|u| (u.protocol, u.origin, u.pathname));
                let expected = expected.map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()));
                assert_eq!(actual, expected, "{input:?}");
            }
            assert_eq!(Url::parse("x").err().unwrap().to_js_string(), "TypeError: Invalid URL");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(url: &str) -> (String, Option<String>, Option<String>) {
        let u = FederatedLinkUrl::from_url(url).unwrap();
        (u.identity, u.name, u.version)
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    /// `__tests__/utils/link-url.spec.ts`.
    #[test]
    fn from_url() {
        assert_eq!(
            parts("https://spec.example.com/a/b/mySchema/v1.0/"),
            ("https://spec.example.com/a/b/mySchema".into(), s("mySchema"), s("v1.0"))
        );
        assert_eq!(
            parts("https://spec.example.com"),
            ("https://spec.example.com".into(), None, None)
        );
        assert_eq!(
            parts("https://spec.example.com/mySchema/v0.1?q=v#frag"),
            ("https://spec.example.com/mySchema".into(), s("mySchema"), s("v0.1"))
        );
        assert_eq!(
            parts("https://spec.example.com/v1.0"),
            ("https://spec.example.com".into(), None, s("v1.0"))
        );
        assert_eq!(
            parts("https://spec.example.com/vX"),
            ("https://spec.example.com/vX".into(), s("vX"), None)
        );
        assert_eq!(
            parts("file:///extensions/permissions"),
            ("file:///extensions/permissions".into(), s("permissions"), None)
        );
        assert_eq!(
            FederatedLinkUrl::from_url("spec.example.com").unwrap_err().message(),
            "Invalid URL"
        );
    }

    #[test]
    fn supports() {
        let url = |u: &str| FederatedLinkUrl::from_url(u).unwrap();
        let yes = [
            (
                "https://spec.example.com/a/b/mySchema/v1.2/",
                "https://spec.example.com/a/b/mySchema/v1.0/",
            ),
            ("https://spec.example.com", "https://spec.example.com"),
            (
                "https://spec.example.com/mySchema/v0.1?q=v#frag",
                "https://spec.example.com/mySchema/v0.1",
            ),
            ("https://spec.example.com/v1.100", "https://spec.example.com/v1.0"),
            ("https://spec.example.com/vX", "https://spec.example.com/vX"),
        ];
        for (a, b) in yes {
            assert!(url(a).supports(SupportsVersion::Url(&url(b))), "{a} {b}");
        }
        let no = [
            (
                "https://spec.example.com/a/b/mySchema/v1.0/",
                "https://spec.example.com/a/b/mySchema/v1.2/",
            ),
            (
                "https://spec.example.com/mySchema/v0.1?q=v#frag",
                "https://spec.example.com/mySchema/v0.2",
            ),
            ("https://spec.example.com/v1.0", "https://spec.example.com/v1.100"),
        ];
        for (a, b) in no {
            assert!(!url(a).supports(SupportsVersion::Url(&url(b))), "{a} {b}");
        }
        let fed = url("https://specs.apollo.dev/federation/v2.3");
        assert!(fed.supports(SupportsVersion::Str("v2.0")));
        assert!(!fed.supports(SupportsVersion::Str("v2.4")));
        assert!(fed.supports(SupportsVersion::MajorMinor(2, 3)));
        assert!(!fed.supports(SupportsVersion::Null));
        assert!(url("https://spec.example.com/vX").supports(SupportsVersion::Null));
    }

    #[test]
    fn to_string() {
        let cases = [
            (
                "https://spec.example.com/a/b/mySchema/v1.0/",
                "https://spec.example.com/a/b/mySchema/v1.0",
            ),
            ("https://spec.example.com", "https://spec.example.com"),
            (
                "https://spec.example.com/mySchema/v0.1?q=v#frag",
                "https://spec.example.com/mySchema/v0.1",
            ),
            ("https://spec.example.com/v1.0", "https://spec.example.com/v1.0"),
            ("https://spec.example.com/vX", "https://spec.example.com/vX"),
        ];
        for (input, expected) in cases {
            assert_eq!(FederatedLinkUrl::from_url(input).unwrap().to_string(), expected);
        }
    }

    #[test]
    fn version_regex() {
        assert_eq!(version_match("v1.0"), Some(("1", "0")));
        assert_eq!(version_match("V12.34567"), Some(("12", "3456")));
        assert_eq!(version_match("v1234.5"), None);
        assert_eq!(version_match("xv1234.5v2.1"), Some(("2", "1")));
        assert_eq!(version_match("v.1"), None);
        assert_eq!(parse_version(Some("v007.010")), (7, 10));
        assert_eq!(parse_version(None), (-1, -1));
    }
}
