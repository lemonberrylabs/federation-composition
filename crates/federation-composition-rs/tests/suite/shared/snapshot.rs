//! `toMatchInlineSnapshot` with vitest 4.1.10 semantics.
//!
//! vitest strips the indentation of the inline snapshot (`stripSnapshotIndentation`),
//! serializes the received value with pretty-format (`escapeString: false`,
//! `printBasicPrototype: false`, indent 2), and passes when both are equal after `trim()`.
//! Copy the TS snapshot text verbatim (cooked: `` \` `` is a backtick, `\\` is `\`).

use federation_composition_rs::sort_sdl;
use federation_composition_rs::utils::js::number_to_string;
use graphql_rs::GraphQLError;
use serde_json::Value;

use super::utils::{is_js_whitespace, js_trim};

/// A value `toMatchInlineSnapshot` can serialize.
pub trait SnapshotValue {
    /// pretty-format's output for the value.
    fn serialize_snapshot(&self) -> String;
}

impl SnapshotValue for str {
    /// A string is printed in double quotes, unescaped (`escapeString: false`).
    fn serialize_snapshot(&self) -> String {
        format!("\"{self}\"")
    }
}

impl SnapshotValue for String {
    fn serialize_snapshot(&self) -> String {
        self.as_str().serialize_snapshot()
    }
}

impl<T: SnapshotValue + ?Sized> SnapshotValue for &T {
    fn serialize_snapshot(&self) -> String {
        (**self).serialize_snapshot()
    }
}

/// `undefined` for `None`.
impl<T: SnapshotValue> SnapshotValue for Option<T> {
    fn serialize_snapshot(&self) -> String {
        match self {
            Some(value) => value.serialize_snapshot(),
            None => "undefined".to_string(),
        }
    }
}

impl SnapshotValue for GraphQLError {
    /// An `Error` is printed as `[${name}: ${message}]`.
    fn serialize_snapshot(&self) -> String {
        format!("[GraphQLError: {}]", self.message)
    }
}

impl SnapshotValue for [GraphQLError] {
    fn serialize_snapshot(&self) -> String {
        pretty_array(self.iter().map(|e| e.serialize_snapshot()).collect(), "")
    }
}

impl SnapshotValue for Vec<GraphQLError> {
    fn serialize_snapshot(&self) -> String {
        self.as_slice().serialize_snapshot()
    }
}

/// A plain JS value (objects, arrays, strings, numbers, booleans, `null`). Object keys are
/// printed sorted, as pretty-format does by default.
impl SnapshotValue for Value {
    fn serialize_snapshot(&self) -> String {
        pretty_value(self, "")
    }
}

/// The snapshot serializer some spec files install with `expect.addSnapshotSerializer`:
/// a string that contains `specs.apollo.dev` is printed as `print(sortSDL(parse(value)))`
/// (without quotes); any other string is printed as usual.
pub struct SortSdlSerializer<'a>(pub &'a str);

impl SnapshotValue for SortSdlSerializer<'_> {
    fn serialize_snapshot(&self) -> String {
        if !self.0.contains("specs.apollo.dev") {
            return self.0.serialize_snapshot();
        }
        let document = match graphql_rs::parse(self.0) {
            Ok(document) => document,
            Err(error) => panic!("snapshot serializer: parse threw: {}", error.message),
        };
        match sort_sdl(&document) {
            Ok(sorted) => graphql_rs::print(&sorted),
            Err(error) => panic!("snapshot serializer: sortSDL threw: {}", error.to_js_string()),
        }
    }
}

fn pretty_array(items: Vec<String>, indentation: &str) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let mut out = String::from("[\n");
    for item in items {
        out.push_str(indentation);
        out.push_str("  ");
        out.push_str(&item);
        out.push_str(",\n");
    }
    out.push_str(indentation);
    out.push(']');
    out
}

fn pretty_value(value: &Value, indentation: &str) -> String {
    let inner = format!("{indentation}  ");
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n.as_f64().expect("a JSON number")),
        Value::String(s) => s.serialize_snapshot(),
        Value::Array(items) => pretty_array(items.iter().map(|v| pretty_value(v, &inner)).collect(), indentation),
        Value::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let mut keys: Vec<&String> = map.keys().collect();
            // `Object.keys(val).sort()`: UTF-16 code unit order.
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            let mut out = String::from("{\n");
            for key in keys {
                out.push_str(&inner);
                out.push_str(&key.serialize_snapshot());
                out.push_str(": ");
                out.push_str(&pretty_value(&map[key], &inner));
                out.push_str(",\n");
            }
            out.push_str(indentation);
            out.push('}');
            out
        }
    }
}

/// vitest's `stripSnapshotIndentation(inlineSnapshot)`.
pub fn strip_snapshot_indentation(inline_snapshot: &str) -> String {
    // `/^([^\S\n]*)\S/m`: the leading whitespace (other than `\n`) of the first line, as the `/m`
    // regex sees lines, that has a non-whitespace character.
    let indentation = inline_snapshot
        .split(['\n', '\r', '\u{2028}', '\u{2029}'])
        .find_map(|line| {
            let first = line.find(|c: char| !is_js_whitespace(c))?;
            Some(&line[..first])
        });
    let Some(indentation) = indentation.filter(|i| !i.is_empty()) else {
        return inline_snapshot.to_string();
    };
    let mut lines: Vec<String> = inline_snapshot.split('\n').map(str::to_string).collect();
    if lines.len() <= 2 {
        return inline_snapshot.to_string();
    }
    if !js_trim(&lines[0]).is_empty() || !js_trim(lines.last().unwrap()).is_empty() {
        return inline_snapshot.to_string();
    }
    let last = lines.len() - 1;
    for line in &mut lines[1..last] {
        if !line.is_empty() {
            if !line.starts_with(indentation) {
                return inline_snapshot.to_string();
            }
            *line = line[indentation.len()..].to_string();
        }
    }
    lines[last] = String::new();
    lines.join("\n")
}

/// `expect(received).toMatchInlineSnapshot(snapshot)`.
#[track_caller]
pub fn to_match_inline_snapshot(received: impl SnapshotValue, snapshot: &str) {
    let expected = strip_snapshot_indentation(snapshot);
    let received = received.serialize_snapshot();
    let (expected, received) = (js_trim(&expected), js_trim(&received));
    if expected != received {
        panic!("Snapshot mismatched\n--- received ---\n{received}\n--- expected ---\n{expected}\n");
    }
}
