//! Port of `src/supergraph/validation/rules/fields-of-the-same-type-rule.ts`.

use std::borrow::Cow;

use crate::utils::collections::{IndexMap, IndexSet};
use graphql_rs::GraphQLError;

use crate::subgraph::state::SubgraphType;
use crate::supergraph::composition::input_object_type::{InputObjectTypeFieldState, InputObjectTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `FieldsOfTheSameTypeRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`, `InputObjectTypeField`.
pub struct FieldsOfTheSameTypeRule;

impl<'s> SupergraphRule<'s> for FieldsOfTheSameTypeRule {
    const NAME: &'static str = "FieldsOfTheSameTypeRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(FieldsOfTheSameTypeRule)
    }
}

/// `type.replaceAll("!", "").replaceAll("[", "").replaceAll("]", "")`.
/// PERF: borrows when the modifiers are only leading and trailing (always the case for a
/// printed type) instead of allocating three strings; the result is the same.
pub(super) fn strip_all_modifiers(ty: &str) -> Cow<'_, str> {
    // The modifiers are ASCII, so byte offsets around them are char boundaries.
    let is_modifier = |b: &u8| matches!(b, b'!' | b'[' | b']');
    let bytes = ty.as_bytes();
    let start = bytes.iter().position(|b| !is_modifier(b)).unwrap_or(bytes.len());
    let end = bytes.iter().rposition(|b| !is_modifier(b)).map_or(start, |i| i + 1);
    let trimmed = &ty[start..end];
    if trimmed.as_bytes().iter().any(is_modifier) {
        Cow::Owned(ty.replace(['!', '[', ']'], ""))
    } else {
        Cow::Borrowed(trimmed)
    }
}

/// The `groups` of the error message: `type "<outputType>" in subgraph(s) "<a>", "<b>"`.
fn type_groups<K: AsRef<str>>(
    cx: &SupergraphValidationContext,
    type_to_graphs: &IndexMap<K, Vec<&String>>,
) -> JsResult<Vec<String>> {
    type_to_graphs
        .iter()
        .map(|(output_type, graphs)| {
            let plural = if graphs.len() > 1 { "s" } else { "" };
            Ok(format!(
                "type \"{}\" in subgraph{plural} \"{}\"",
                output_type.as_ref(),
                cx.graph_ids_to_names(graphs.iter().copied())?.join("\", \"")
            ))
        })
        .collect()
}

/// `${first} but ${second}${rest.length ? ` and ${rest.join(" and ")}` : ""}`.
fn first_but_second_and_rest(groups: &[String]) -> String {
    let rest = &groups[2..];
    format!(
        "{} but {}{}",
        groups[0],
        groups[1],
        if !rest.is_empty() {
            format!(" and {}", rest.join(" and "))
        } else {
            String::new()
        }
    )
}

/// The key of `typeToGraphs` in `ObjectTypeField`: the local type, or its normalized form when
/// it matches the normalized supergraph type.
/// PERF: when the local type is the supergraph type and not a list, `normalizeOutputTypeStrings`
/// returns it unchanged for both (the `!` is stripped from the local type only when the
/// supergraph type, the same string, has none), so nothing needs to be allocated.
fn object_field_output_type<'s>(super_type: &'s str, local_type: &'s str) -> Cow<'s, str> {
    if local_type == super_type && !local_type.starts_with('[') {
        return Cow::Borrowed(local_type);
    }
    let normalized_output_types = normalize_output_type_strings(OutputTypeStrings {
        super_type: super_type.to_string(),
        local_type: local_type.to_string(),
    });
    if normalized_output_types.super_type == normalized_output_types.local_type {
        Cow::Owned(normalized_output_types.local_type)
    } else {
        Cow::Borrowed(local_type)
    }
}

/// The key of `typeToGraphs` in `InputObjectTypeField`.
fn input_field_output_type<'s>(super_type: &'s str, local_type: &'s str) -> Cow<'s, str> {
    // We normalize the type to remove the non-null modifier
    let is_non_nullable = local_type.ends_with('!');
    let is_non_nullable_in_supergraph = super_type.ends_with('!');
    let is_matching_non_nullable_part = strip_non_null(super_type) == strip_non_null(local_type);

    // Turn User! into User (if super type is nullable)
    // Supergraph type | sign | Local type
    // -----------------------------------
    // `User!`           ===    `User`
    // `String!`         !==    `User`
    if is_matching_non_nullable_part {
        if is_non_nullable_in_supergraph {
            if is_non_nullable {
                Cow::Borrowed(local_type)
            } else {
                Cow::Owned(format!("{local_type}!"))
            }
        } else {
            Cow::Borrowed(local_type)
        }
    } else {
        Cow::Borrowed(local_type)
    }
}

/// Whether all the output types are the same (`typeToGraphs.size <= 1`).
fn has_one_output_type<'s>(mut output_types: impl Iterator<Item = Cow<'s, str>>) -> bool {
    match output_types.next() {
        Some(first) => output_types.all(|output_type| output_type == first),
        None => true,
    }
}

impl<'s> SupergraphVisitor<'s> for FieldsOfTheSameTypeRule {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        // PERF: when every graph has the same output type, `typeToGraphs` has one entry and
        // nothing is reported; the union and interface lookups only matter otherwise, so the
        // maps are only built when the output types differ.
        if has_one_output_type(
            field_state
                .by_graph
                .values()
                .map(|field| object_field_output_type(&field_state.r#type, &field.r#type)),
        ) {
            return Ok(());
        }

        // PERF: the keys and graph names borrow from the state instead of being cloned.
        let mut type_to_graphs: IndexMap<Cow<'s, str>, Vec<&'s String>> = IndexMap::default();
        let mut type_name_to_possible_type_names: IndexMap<Cow<'s, str>, IndexSet<String>> = IndexMap::default();

        for (graph_name, field) in &field_state.by_graph {
            let type_name = strip_all_modifiers(&field.r#type);
            let type_state = cx
                .subgraph_states
                .get(graph_name)
                .and_then(|s| s.types.get(type_name.as_ref()));

            match type_state {
                Some(SubgraphType::Union(union_type)) => {
                    let list = type_name_to_possible_type_names.entry(type_name.clone()).or_default();
                    for member in &union_type.members {
                        list.insert(member.clone());
                    }
                }
                Some(SubgraphType::Interface(interface_type)) => {
                    let list = type_name_to_possible_type_names.entry(type_name.clone()).or_default();
                    for member in &interface_type.implemented_by {
                        list.insert(member.clone());
                    }
                }
                _ => {}
            }

            let field_output_type = object_field_output_type(&field_state.r#type, &field.r#type);

            type_to_graphs.entry(field_output_type).or_default().push(graph_name);
        }

        if type_to_graphs.len() > 1 {
            if type_name_to_possible_type_names.len() == 1 {
                let mut possible_type_names: Vec<&str> = Vec::new();

                for (union_or_interface_name, list) in &type_name_to_possible_type_names {
                    possible_type_names.push(union_or_interface_name);
                    for type_name in list {
                        possible_type_names.push(type_name);
                    }
                }

                let output_type_names: Vec<Cow<'_, str>> =
                    type_to_graphs.keys().map(|t| strip_all_modifiers(t)).collect();

                if output_type_names
                    .iter()
                    .all(|t| possible_type_names.iter().any(|p| *p == t.as_ref()))
                {
                    return Ok(());
                }
            }

            let groups = type_groups(cx, &type_to_graphs)?;

            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type of field \"{}.{}\" is incompatible across subgraphs: it has {}",
                    object_type_state.name,
                    field_state.name,
                    first_but_second_and_rest(&groups)
                ),
                "FIELD_TYPE_MISMATCH",
            ));
        }

        Ok(())
    }

    fn input_object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        input_object_type_state: &'s InputObjectTypeState,
        field_state: &'s InputObjectTypeFieldState,
    ) -> JsResult<()> {
        // PERF: with a single output type nothing is reported (see `object_type_field`).
        if has_one_output_type(
            field_state
                .by_graph
                .values()
                .map(|field| input_field_output_type(&field_state.r#type, &field.r#type)),
        ) {
            return Ok(());
        }

        // PERF: the keys and graph names borrow from the state instead of being cloned.
        let mut type_to_graphs: IndexMap<Cow<'s, str>, Vec<&'s String>> = IndexMap::default();

        for (graph_name, field) in &field_state.by_graph {
            let normalized_output_type = input_field_output_type(&field_state.r#type, &field.r#type);

            type_to_graphs
                .entry(normalized_output_type)
                .or_default()
                .push(graph_name);
        }

        if type_to_graphs.len() > 1 {
            let groups = type_groups(cx, &type_to_graphs)?;
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type of field \"{}.{}\" is incompatible across subgraphs: it has {}",
                    input_object_type_state.name,
                    field_state.name,
                    first_but_second_and_rest(&groups)
                ),
                "FIELD_TYPE_MISMATCH",
            ));
        }

        Ok(())
    }
}

/// `stripNonNull(type)`: `type.replace(/!$/, "")`, removes one trailing `!`.
fn strip_non_null(ty: &str) -> &str {
    ty.strip_suffix('!').unwrap_or(ty)
}

/// JS `str.slice(start, end)` on an ASCII string (negative indexes count from the end).
fn js_slice(s: &str, start: isize, end: isize) -> String {
    let len = s.len() as isize;
    let normalize = |i: isize| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (from, to) = (normalize(start), normalize(end));
    if to <= from {
        return String::new();
    }
    s[from as usize..to as usize].to_string()
}

/// `stripList(type)`: `type.slice(indexOf("[") + 1, lastIndexOf("]"))`.
/// NOTE(port): JS `slice` with `-1` end (no `]`) counts from the end; ported exactly. Type
/// strings are ASCII, so byte indexes are UTF-16 indexes.
fn strip_list(ty: &str) -> String {
    let starts_at = ty.find('[').map_or(-1, |i| i as isize);
    let ends_at = ty.rfind(']').map_or(-1, |i| i as isize);

    js_slice(ty, starts_at + 1, ends_at)
}

/// `{ superType, localType }`, the argument and result of `normalizeOutputTypeStrings`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OutputTypeStrings {
    super_type: String,
    local_type: String,
}

/// `normalizeOutputTypeStrings({ superType, localType })`: removes the modifiers that make two
/// output types compatible (see the table in the TS).
fn normalize_output_type_strings(input: OutputTypeStrings) -> OutputTypeStrings {
    let mut super_type_normalized = input.super_type;
    let mut local_type_normalized = input.local_type;

    // If the super type is nullable, we remove the non-null modifier from the local type
    if !super_type_normalized.ends_with('!') {
        local_type_normalized = strip_non_null(&local_type_normalized).to_string();
    }

    // If the super type is a list and local type as well, we remove the list modifier
    if super_type_normalized.starts_with('[') && local_type_normalized.starts_with('[') {
        let inner_super = strip_list(&super_type_normalized);
        let mut inner_local = strip_list(&local_type_normalized);

        // If the inner type of super type is nullable, we remove the non-null modifier from the inner type of the local type
        if !inner_super.ends_with('!') {
            inner_local = strip_non_null(&inner_local).to_string();
        }

        let super_sign = if super_type_normalized.ends_with('!') { "!" } else { "" };
        let local_sign = if local_type_normalized.ends_with('!') { "!" } else { "" };
        super_type_normalized = format!("[{inner_super}]{super_sign}");
        local_type_normalized = format!("[{inner_local}]{local_sign}");
    }

    OutputTypeStrings {
        super_type: super_type_normalized,
        local_type: local_type_normalized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize(super_type: &str, local_type: &str) -> (String, String) {
        let r = normalize_output_type_strings(OutputTypeStrings {
            super_type: super_type.into(),
            local_type: local_type.into(),
        });
        (r.super_type, r.local_type)
    }

    #[test]
    fn strip_list_matches_js_slice() {
        assert_eq!(strip_list("[User!]!"), "User!");
        assert_eq!(strip_list("[[User]]"), "[User]");
        // No `]`: `slice(1, -1)` drops the last character.
        assert_eq!(strip_list("[User"), "Use");
        // No `[`: `slice(0, ..)`.
        assert_eq!(strip_list("User]"), "User");
        assert_eq!(strip_list("]["), "");
    }

    #[test]
    fn strip_non_null_removes_one_trailing_bang() {
        assert_eq!(strip_non_null("User!!"), "User!");
        assert_eq!(strip_non_null("User"), "User");
    }

    #[test]
    fn normalize_output_type_strings_table() {
        // Supergraph type | sign | Local type
        let same = |s: &str, l: &str| {
            let (a, b) = normalize(s, l);
            a == b
        };
        assert!(same("User", "User!"));
        assert!(!same("String", "User!"));
        assert!(same("[User]", "[User!]"));
        assert!(same("[User!]", "[User!]"));
        assert!(same("[User]!", "[User!]!"));
        assert!(same("[User]!", "[User]!"));
        assert!(same("[User]", "[User!]!"));
        assert!(!same("[User!]", "[User]"));
        assert!(!same("User!", "User"));
        assert_eq!(normalize("[User]", "[User!]!"), ("[User]".into(), "[User]".into()));
    }

    #[test]
    fn strip_all_modifiers_removes_every_modifier() {
        assert_eq!(strip_all_modifiers("[[User!]!]!"), "User");
    }

    #[test]
    fn first_but_second_and_rest_joins_groups() {
        let g = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(first_but_second_and_rest(&g(&["a", "b"])), "a but b");
        assert_eq!(
            first_but_second_and_rest(&g(&["a", "b", "c", "d"])),
            "a but b and c and d"
        );
    }
}
