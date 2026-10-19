//! Port of `src/utils/link/link-import.ts`.

use graphql_js::ConstValue;

use crate::utils::js::JsResult;

/// `FederatedLinkImport`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FederatedLinkImport {
    pub name: String,
    /// TS `as`.
    pub as_: Option<String>,
}

impl FederatedLinkImport {
    pub fn new(name: String, as_: Option<String>) -> Self {
        FederatedLinkImport { name, as_ }
    }

    /// `toString()`: `{ name: "N", as: "A" }` or `"N"`.
    /// NOTE(port): `this.as ? ..` is falsy for `""`.
    pub fn to_string(&self) -> String {
        match &self.as_ {
            Some(as_) if !as_.is_empty() => format!("{{ name: \"{}\", as: \"{}\" }}", self.name, as_),
            _ => format!("\"{}\"", self.name),
        }
    }

    /// `FederatedLinkImport.fromTypedefs(node)`. Throws on unexpected value kinds.
    pub fn from_typedefs(node: &ConstValue) -> JsResult<Vec<FederatedLinkImport>> {
        todo!()
    }
}
