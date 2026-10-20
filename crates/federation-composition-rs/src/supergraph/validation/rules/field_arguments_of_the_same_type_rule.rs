//! Port of `src/supergraph/validation/rules/field-arguments-of-the-same-type-rule.ts`.

use graphql_rs::GraphQLError;
use crate::utils::collections::{IndexMap, IndexSet};

use crate::supergraph::composition::directive::{DirectiveArgState, DirectiveState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldArgState, ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;

/// `FieldArgumentsOfTheSameTypeRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeFieldArg`, `DirectiveFieldArg`.
pub struct FieldArgumentsOfTheSameTypeRule;

impl<'s> SupergraphRule<'s> for FieldArgumentsOfTheSameTypeRule {
    const NAME: &'static str = "FieldArgumentsOfTheSameTypeRule";

    fn new(_context: &mut SupergraphValidationContext, _supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(FieldArgumentsOfTheSameTypeRule)
    }
}

impl<'s> SupergraphVisitor<'s> for FieldArgumentsOfTheSameTypeRule {
    fn object_type_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
        arg_state: &'s ObjectTypeFieldArgState,
    ) -> JsResult<()> {
        // PERF: when every graph has the supergraph type, the normalized type below is that type
        // for every graph, `typeToGraphs` has one entry and nothing is reported.
        if arg_state.by_graph.values().all(|arg| arg.r#type == arg_state.r#type) {
            return Ok(());
        }

        let mut type_to_graphs: IndexMap<String, Vec<String>> = IndexMap::default();

        for (graph_name, arg) in &arg_state.by_graph {
            // We normalize the type to remove the non-null modifier
            // TODO: This needs to show `!` and `[]` modifiers as well
            // TODO: This needs a good comparison logic (e.g. `String!` and `String` are the same in context of fields and arguments)
            let is_non_nullable = arg.r#type.ends_with('!');
            let is_non_nullable_in_supergraph = arg_state.r#type.ends_with('!');
            let is_matching_non_nullable_part = strip_trailing_bang(&arg_state.r#type) == strip_trailing_bang(&arg.r#type);

            // Turn User into User! (if super type is non-nullable)
            // Supergraph type | sign | Local type
            // -----------------------------------
            // `User!`           ===    `User`
            // `String!`         !==    `User`
            let normalized_output_type = if is_matching_non_nullable_part {
                if is_non_nullable_in_supergraph {
                    if is_non_nullable {
                        arg.r#type.clone()
                    } else {
                        format!("{}!", arg.r#type)
                    }
                } else {
                    arg.r#type.clone()
                }
            } else {
                arg.r#type.clone()
            };

            type_to_graphs
                .entry(normalized_output_type)
                .or_default()
                .push(graph_name.clone());
        }

        if type_to_graphs.len() > 1 {
            let mut groups = Vec::new();
            for (output_type, graphs) in &type_to_graphs {
                let plural = if graphs.len() > 1 { "s" } else { "" };
                groups.push(format!(
                    "type \"{output_type}\" in subgraph{plural} \"{}\"",
                    cx.graph_ids_to_names(graphs)?.join("\", \"")
                ));
            }
            let rest = &groups[2..];
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Type of argument \"{}.{}({}:)\" is incompatible across subgraphs: it has {} but {}{}",
                    object_type_state.name,
                    field_state.name,
                    arg_state.name,
                    groups[0],
                    groups[1],
                    if !rest.is_empty() {
                        format!(" and {}", rest.join(" and "))
                    } else {
                        String::new()
                    }
                ),
                "FIELD_ARGUMENT_TYPE_MISMATCH",
            ));
        }

        Ok(())
    }

    fn directive_field_arg(
        &mut self,
        cx: &mut SupergraphValidationContext,
        directive_state: &'s DirectiveState,
        arg_state: &'s DirectiveArgState,
    ) -> JsResult<()> {
        if !directive_state.is_executable {
            // We only want to do this validation on executable directives
            return Ok(());
        }

        if arg_state.by_graph.len() != cx.subgraph_states.len() {
            // If the directive is not defined in all subgraphs it will be omitted from the supergraph
            return Ok(());
        }

        let mut types_per_subgraph: IndexMap<String, IndexSet<String>> = IndexMap::default();

        for (graph_name, arg) in &arg_state.by_graph {
            // `collectSubgraphType(arg.type, graphName)`
            types_per_subgraph
                .entry(arg.r#type.clone())
                .or_default()
                .insert(graph_name.clone());
        }

        if types_per_subgraph.len() == 1 {
            return Ok(());
        }

        let mut groups = Vec::new();
        for (output_type, graphs) in &types_per_subgraph {
            let plural = if graphs.len() > 1 { "s" } else { "" };
            groups.push(format!(
                "type \"{output_type}\" in subgraph{plural} {}",
                join_items(&cx.graph_ids_to_names(graphs)?)
            ));
        }
        let first = &groups[0];
        let rest = &groups[1..];
        cx.report_error(GraphQLError::with_code(
            format!(
                "Type of argument \"@{}({}:)\" is incompatible across subgraphs: it has {first} but {}",
                directive_state.name,
                arg_state.name,
                rest.join(" and ")
            ),
            "FIELD_ARGUMENT_TYPE_MISMATCH",
        ));

        Ok(())
    }

    fn has_directive_field_arg(&self) -> bool {
        true
    }
}

/// `type.replace(/!$/, "")`.
fn strip_trailing_bang(ty: &str) -> &str {
    ty.strip_suffix('!').unwrap_or(ty)
}

fn join_items(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => format!("\"{}\"", items[0]),
        2 => format!("\"{}\" and \"{}\"", items[0], items[1]),
        n => format!("\"{}\", and \"{}\"", items[..n - 1].join("\", \""), items[n - 1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn join_items_formats_like_the_ts() {
        assert_eq!(join_items(&s(&[])), "");
        assert_eq!(join_items(&s(&["a"])), "\"a\"");
        assert_eq!(join_items(&s(&["a", "b"])), "\"a\" and \"b\"");
        assert_eq!(join_items(&s(&["a", "b", "c"])), "\"a\", \"b\", and \"c\"");
    }

    #[test]
    fn strip_trailing_bang_removes_one() {
        assert_eq!(strip_trailing_bang("[A!]!"), "[A!]");
        assert_eq!(strip_trailing_bang("A"), "A");
    }
}
