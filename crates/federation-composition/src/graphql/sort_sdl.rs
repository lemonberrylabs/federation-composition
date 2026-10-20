//! Port of `src/graphql/sort-sdl.ts`: normalizes the AST of supergraph SDLs so they can be
//! compared without worrying about ordering.

use std::cmp::Ordering;

use graphql_core::{
    parse, strip_ignored_characters, visit_edit, AsNodeRef, Definition, Document, EditAction, EditResult, EditVisitor,
    Kind, Node, NodeRef, SelectionSet, Value, VisitInfo,
};

use super::contains_supergraph_spec::js_trim;
use super::printer::print;
use crate::utils::js::{JsError, JsResult};

/// The visitor object passed to `visit`. A `throw` inside it (from `parseFields`) is kept in
/// `error` and stops the traversal, as the exception does in TS.
struct SortVisitor {
    error: Option<JsError>,
}

impl SortVisitor {
    /// The `Directive` visitor, which can throw.
    fn directive(&mut self, node: &graphql_core::Directive) -> JsResult<graphql_core::Directive> {
        // NOTE(port): the TS assigns the normalized string to the argument node in place, which
        // also changes the caller's document; the port changes a copy.
        let mut arguments = node.arguments.clone();
        for arg in &mut arguments {
            if ["requires", "provides"].contains(&arg.name.value.as_str()) {
                if let Value::String(value) = &mut arg.value {
                    let parsed_fields = parse_fields(&value.value)?;

                    if let Some(parsed_fields) = parsed_fields {
                        let printed = strip_ignored_characters(&print(&parsed_fields))?;

                        let printed = printed.strip_prefix('{').unwrap_or(&printed);
                        let printed = printed.strip_suffix('}').unwrap_or(printed);
                        value.value = printed.to_string();
                    }
                }
            }
        }

        Ok(graphql_core::Directive {
            arguments: sort_nodes(&arguments),
            ..node.clone()
        })
    }
}

impl EditVisitor for SortVisitor {
    fn enter(&mut self, node: NodeRef<'_>, _info: &VisitInfo<'_, '_>) -> EditAction {
        let replacement: Node = match node {
            NodeRef::Document(node) => Document {
                definitions: sort_nodes(&node.definitions),
            }
            .into(),
            NodeRef::SchemaDefinition(node) => graphql_core::SchemaDefinition {
                directives: sort_nodes(&node.directives),
                ..node.clone()
            }
            .into(),
            NodeRef::ScalarTypeDefinition(node) => graphql_core::ScalarTypeDefinition {
                directives: sort_nodes(&node.directives),
                ..node.clone()
            }
            .into(),
            NodeRef::ObjectTypeDefinition(node) => graphql_core::ObjectTypeDefinition {
                directives: sort_nodes(&node.directives),
                fields: sort_nodes(&node.fields),
                interfaces: sort_nodes(&node.interfaces),
                ..node.clone()
            }
            .into(),
            NodeRef::InterfaceTypeDefinition(node) => graphql_core::InterfaceTypeDefinition {
                directives: sort_nodes(&node.directives),
                fields: sort_nodes(&node.fields),
                ..node.clone()
            }
            .into(),
            NodeRef::InputObjectTypeDefinition(node) => graphql_core::InputObjectTypeDefinition {
                directives: sort_nodes(&node.directives),
                fields: sort_nodes(&node.fields),
                ..node.clone()
            }
            .into(),
            NodeRef::EnumTypeDefinition(node) => graphql_core::EnumTypeDefinition {
                directives: sort_nodes(&node.directives),
                values: sort_nodes(&node.values),
                ..node.clone()
            }
            .into(),
            NodeRef::EnumValueDefinition(node) => graphql_core::EnumValueDefinition {
                directives: sort_nodes(&node.directives),
                ..node.clone()
            }
            .into(),
            NodeRef::UnionTypeDefinition(node) => graphql_core::UnionTypeDefinition {
                types: sort_nodes(&node.types),
                directives: sort_nodes(&node.directives),
                ..node.clone()
            }
            .into(),
            NodeRef::FieldDefinition(node) => graphql_core::FieldDefinition {
                directives: sort_nodes(&node.directives),
                // arguments: sortNodes(node.arguments),
                ..node.clone()
            }
            .into(),
            NodeRef::DirectiveDefinition(node) => graphql_core::DirectiveDefinition {
                locations: sort_nodes(&node.locations),
                ..node.clone()
            }
            .into(),
            NodeRef::Directive(node) => match self.directive(node) {
                Ok(directive) => directive.into(),
                Err(error) => {
                    self.error = Some(error);
                    return EditAction::Break;
                }
            },
            NodeRef::StringValue(node) => graphql_core::StringValue {
                value: js_trim(&node.value).to_string(),
                ..node.clone()
            }
            .into(),
            _ => return EditAction::Continue,
        };
        EditAction::Replace(replacement)
    }
}

/// `sortSDL(doc)`.
/// NOTE(port): the TS `catch` logs `"Failed to parse"` (with the document's source name) and
/// rethrows; the log is dropped. The errors are the `parse`/`stripIgnoredCharacters` syntax
/// errors of `@requires`/`@provides` field sets.
pub fn sort_sdl(doc: &Document) -> JsResult<Document> {
    let mut visitor = SortVisitor { error: None };
    let result = visit_edit(doc, &mut visitor);
    if let Some(error) = visitor.error {
        return Err(error);
    }
    match result {
        EditResult::Node(Node::Document(document)) => Ok(document),
        // The `Document` visitor always replaces the root with a document.
        other => unreachable!("sortSDL returns a document: {other:?}"),
    }
}

/// `parseFields(fields)`.
fn parse_fields(fields: &str) -> JsResult<Option<SelectionSet>> {
    let source = if js_trim(fields).starts_with('{') {
        format!("query {fields}")
    } else {
        format!("query {{ {fields} }}")
    };
    let parsed = parse(&source)?.definitions.into_iter().find_map(|d| match d {
        Definition::OperationDefinition(op) => Some(op),
        _ => None,
    });

    Ok(parsed.map(|op| op.selection_set))
}

/// `valueNodeToString(node)`.
fn value_node_to_string(node: &Value) -> String {
    match node {
        // `"name" in node`
        Value::Variable(v) => v.name.value.clone(),
        // `"value" in node`
        Value::Int(v) => v.value.clone(),
        Value::Float(v) => v.value.clone(),
        Value::String(v) => v.value.clone(),
        Value::Boolean(v) => v.value.to_string(),
        Value::Enum(v) => v.value.clone(),
        Value::List(v) => v.values.iter().map(value_node_to_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "OBJECT".to_string(),
        Value::Null(_) => "NULL".to_string(),
    }
}

/// JS `<`/`>` on strings: UTF-16 code unit order.
fn js_string_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// lodash `compareAscending` for string-or-undefined keys: `undefined` sorts last.
fn compare_ascending(a: &Option<String>, b: &Option<String>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => js_string_cmp(a, b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// `lodash.sortby(nodes, ...iteratees)`: a stable sort by the keys in order.
fn sort_by_keys<T: Clone>(nodes: &[T], keys: impl Fn(&T) -> Vec<Option<String>>) -> Vec<T> {
    let mut keyed: Vec<(Vec<Option<String>>, &T)> = nodes.iter().map(|n| (keys(n), n)).collect();
    keyed.sort_by(|(a, _), (b, _)| {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| compare_ascending(x, y))
            .find(|o| *o != Ordering::Equal)
            .unwrap_or(Ordering::Equal)
    });
    keyed.into_iter().map(|(_, n)| n.clone()).collect()
}

/// `get(node, "name.value")`.
fn name_value(node: NodeRef<'_>) -> Option<String> {
    node.name().map(|n| n.value.clone())
}

/// `sortNodes(nodes)`: one generic function for the TS overloads; the list kind is decided by
/// the kind of the first node, as in TS. `lodash.sortby` is a stable sort by the key(s).
fn sort_nodes<T: AsNodeRef + Clone>(nodes: &[T]) -> Vec<T> {
    if nodes.is_empty() {
        return Vec::new();
    }

    if is_of_kind_list(nodes, &[Kind::NamedType]) {
        return sort_by_keys(nodes, |n| vec![name_value(n.as_node_ref())]);
    }

    if is_of_kind_list(nodes, &[Kind::Directive]) {
        return sort_by_keys(nodes, |n| {
            let NodeRef::Directive(n) = n.as_node_ref() else {
                return vec![name_value(n.as_node_ref())];
            };
            let mut args: Vec<String> = n
                .arguments
                .iter()
                .map(|a| format!("{}{}", a.name.value, value_node_to_string(&a.value)))
                .collect();
            // `.sort()`: UTF-16 code unit order.
            args.sort_by(|a, b| js_string_cmp(a, b));
            vec![Some(format!("{}{}", n.name.value, args.join(";")))]
        });
    }

    if is_of_kind_list(nodes, &[Kind::VariableDefinition]) {
        return sort_by_keys(nodes, |n| match n.as_node_ref() {
            NodeRef::VariableDefinition(v) => vec![Some(v.variable.name.value.clone())],
            other => vec![name_value(other)],
        });
    }

    if is_of_kind_list(nodes, &[Kind::Argument]) {
        return sort_by_keys(nodes, |n| vec![name_value(n.as_node_ref())]);
    }

    if is_of_kind_list(nodes, &[Kind::EnumValueDefinition]) {
        return sort_by_keys(nodes, |n| vec![name_value(n.as_node_ref())]);
    }

    if is_of_kind_list(nodes, &[Kind::InputValueDefinition]) {
        return sort_by_keys(nodes, |n| vec![name_value(n.as_node_ref())]);
    }

    if is_of_kind_list(nodes, &[Kind::Field, Kind::FragmentSpread, Kind::InlineFragment]) {
        return sort_by_keys(nodes, |n| {
            let n = n.as_node_ref();
            vec![Some(n.kind().as_str().to_string()), name_value(n)]
        });
    }

    if is_of_kind_list(nodes, &[Kind::Name]) {
        return sort_by_keys(nodes, |n| match n.as_node_ref() {
            NodeRef::Name(name) => vec![Some(name.value.clone())],
            // `get(node, "value")` of a non-Name node in a list that starts with a Name.
            _ => vec![None],
        });
    }

    sort_by_keys(nodes, |n| {
        let n = n.as_node_ref();
        vec![Some(n.kind().as_str().to_string()), name_value(n)]
    })
}

/// `isOfKindList(nodes, kind)`: checks the first node's kind.
fn is_of_kind_list<T: AsNodeRef>(nodes: &[T], kinds: &[graphql_core::Kind]) -> bool {
    nodes.first().is_some_and(|n| kinds.contains(&n.as_node_ref().kind()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(sdl: &str) -> String {
        graphql_core::print(&sort_sdl(&parse(sdl).unwrap()).unwrap())
    }

    /// Expected output generated with the TS `sortSDL` and graphql-js `print`.
    #[test]
    fn sorts() {
        assert_eq!(
            sorted(
                r#"
type B implements Y & X @z @a(b: 2, a: [1, "x"]) @a(a: 1) {
  z: Int @join__field(requires: "  b { c   d }  ", provides: "{ x }")
  a(b: Int, a: Int): Int
}
"  desc  "
scalar A
enum E { Z A }
union U = Z | A
directive @d on OBJECT | FIELD
schema @b @a { query: B }
extend type Q { a: Int }
input I { b: Int a: Int }
interface Itf implements Z & A { b: Int a: Int }
"#
            ),
            "directive @d on FIELD | OBJECT\n\nenum E {\n  A\n  Z\n}\n\ninput I {\n  a: Int\n  b: Int\n}\n\ninterface Itf implements Z & A {\n  a: Int\n  b: Int\n}\n\ntype B implements X & Y @a(a: 1) @a(a: [1, \"x\"], b: 2) @z {\n  a(b: Int, a: Int): Int\n  z: Int @join__field(provides: \"x\", requires: \"b{c d}\")\n}\n\nextend type Q {\n  a: Int\n}\n\n\"desc\"\nscalar A\n\nschema @a @b {\n  query: B\n}\n\nunion U = A | Z"
        );
    }

    #[test]
    fn field_set_syntax_error() {
        let err = sort_sdl(&parse(r#"type A { a: Int @join__field(requires: "{") }"#).unwrap()).unwrap_err();
        assert_eq!(err.message(), "Syntax Error: Expected Name, found <EOF>.");
    }
}
