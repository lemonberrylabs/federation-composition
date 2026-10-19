//! Port of `src/utils/link/link-url.ts`.

use crate::utils::js::JsResult;

/// `VERSION_MATCH = /v(\d{1,3})\.(\d{1,4})/i` (unanchored).
pub const VERSION_MATCH: &str = r"(?i)v(\d{1,3})\.(\d{1,4})";

/// `parseVersion(version)`: `[major, minor]`, or `[-1, -1]` when there is no match.
fn parse_version(version: Option<&str>) -> (i64, i64) {
    todo!()
}

/// The argument of the overloaded `supports(...)`.
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

/// `FederatedLinkUrl`: the parsed `@link(url:)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FederatedLinkUrl {
    pub identity: String,
    pub name: Option<String>,
    pub version: Option<String>,
    /// -1 if no version is set.
    major: i64,
    minor: i64,
}

impl FederatedLinkUrl {
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

    /// `toString()`.
    pub fn to_string(&self) -> String {
        match &self.version {
            Some(v) if !v.is_empty() => format!("{}/{}", self.identity, v),
            _ => self.identity.clone(),
        }
    }

    /// `FederatedLinkUrl.fromUrl(urlSource)`. Throws `TypeError: Invalid URL` (WHATWG URL
    /// parsing, as `new URL`) for inputs such as `"spec.example.com"`.
    /// NOTE(port): needs the subset of WHATWG URL parsing that produces `protocol`, `origin`
    /// and `pathname` for the inputs `src/` sees.
    pub fn from_url(url_source: &str) -> JsResult<FederatedLinkUrl> {
        todo!()
    }

    /// `supports(...)`: whether this version supports another version.
    pub fn supports(&self, version: SupportsVersion<'_>) -> bool {
        todo!()
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
