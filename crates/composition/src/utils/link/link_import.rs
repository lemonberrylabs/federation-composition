//! Port of `src/utils/link/link-import.ts`.

use graphql_js::ConstValue;

use crate::utils::js::{JsError, JsResult};

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
        if let ConstValue::List(list) = node {
            let mut imports = Vec::with_capacity(list.values.len());
            for v in &list.values {
                if let ConstValue::String(s) = v {
                    imports.push(FederatedLinkImport::new(s.value.clone(), None));
                    continue;
                }
                if let ConstValue::Object(object) = v {
                    let mut name = String::new();
                    let mut as_: Option<String> = None;

                    for f in &object.fields {
                        if f.name.value == "name" {
                            match &f.value {
                                ConstValue::String(s) => name = s.value.clone(),
                                other => {
                                    return Err(JsError::error(format!(
                                        "Expected string value for @link \"name\" field but got \"{}\"",
                                        other.kind().as_str()
                                    )))
                                }
                            }
                        } else if f.name.value == "as" {
                            match &f.value {
                                ConstValue::String(s) => as_ = Some(s.value.clone()),
                                other => {
                                    return Err(JsError::error(format!(
                                        "Expected string value for @link \"as\" field but got \"{}\"",
                                        other.kind().as_str()
                                    )))
                                }
                            }
                        }
                    }
                    imports.push(FederatedLinkImport::new(name, as_));
                    continue;
                }
                return Err(JsError::error(format!(
                    "Unexpected value kind \"{}\" in @link import declaration",
                    v.kind().as_str()
                )));
            }
            return Ok(imports);
        }
        Err(JsError::error(format!(
            "Expected a list of @link imports but got \"{}\"",
            node.kind().as_str()
        )))
    }
}
