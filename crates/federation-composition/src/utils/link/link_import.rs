//! Port of `src/utils/link/link-import.ts`.

use std::fmt;

use graphql_core::ConstValue;

use crate::utils::js::JsError;
use crate::Error;

/// One element of a `@link`'s `import` list: upstream's `FederatedLinkImport`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FederatedLinkImport {
    /// The imported name, e.g. `@key` or `FieldSet`.
    pub name: String,
    /// The local name it is imported as (TS `as`), e.g. `@myKey`.
    pub as_: Option<String>,
}

impl FederatedLinkImport {
    /// `new FederatedLinkImport(name, as)`.
    pub fn new(name: String, as_: Option<String>) -> Self {
        FederatedLinkImport { name, as_ }
    }

    /// `FederatedLinkImport.fromTypedefs(node)`: the imports of a `@link`'s `import` argument.
    /// `Err` (upstream throws) on unexpected value kinds.
    pub fn from_typedefs(node: &ConstValue) -> Result<Vec<FederatedLinkImport>, Error> {
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
                                    return Err(Error::from(JsError::error(format!(
                                        "Expected string value for @link \"name\" field but got \"{}\"",
                                        other.kind().as_str()
                                    ))))
                                }
                            }
                        } else if f.name.value == "as" {
                            match &f.value {
                                ConstValue::String(s) => as_ = Some(s.value.clone()),
                                other => {
                                    return Err(Error::from(JsError::error(format!(
                                        "Expected string value for @link \"as\" field but got \"{}\"",
                                        other.kind().as_str()
                                    ))))
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
                ))
                .into());
            }
            return Ok(imports);
        }
        Err(JsError::error(format!(
            "Expected a list of @link imports but got \"{}\"",
            node.kind().as_str()
        ))
        .into())
    }
}

impl fmt::Display for FederatedLinkImport {
    /// `toString()`: `{ name: "N", as: "A" }` or `"N"`.
    /// NOTE(port): `this.as ? ..` is falsy for `""`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.as_ {
            Some(as_) if !as_.is_empty() => write!(f, "{{ name: \"{}\", as: \"{}\" }}", self.name, as_),
            _ => write!(f, "\"{}\"", self.name),
        }
    }
}
