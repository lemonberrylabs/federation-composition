//! Port of `src/supergraph/validation/rules/directive-composition-rule.ts`.

use crate::utils::collections::{IndexMap, IndexSet};
use graphql_core::GraphQLError;

use crate::specifications::link::LinkImportKind;
use crate::supergraph::composition::directive::DirectiveState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::{parse_int, JsResult};

/// `DirectiveCompositionRule(context, supergraph)`. The factory's locals
/// (`linkIdentityToMajorVersion`, `appliedDirectiveToGraphs`,
/// `appliedDirectiveToLinkIdentities`) are only used in `new`.
/// Visitor callbacks: `Directive`.
pub struct DirectiveCompositionRule;

/// A JS number as JSON: integers as integers, `NaN`/`Infinity` as `null` (`JSON.stringify`).
fn js_number_to_json(n: f64) -> serde_json::Value {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        serde_json::Value::from(n as i64)
    } else {
        serde_json::Number::from_f64(n).map_or(serde_json::Value::Null, serde_json::Value::Number)
    }
}

impl<'s> SupergraphRule<'s> for DirectiveCompositionRule {
    const NAME: &'static str = "DirectiveCompositionRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        let mut link_identity_to_major_version: IndexMap<String, f64> = IndexMap::default();
        // Map<originalDirectiveName, Map<appliedName, Set<graph>>>
        let mut applied_directive_to_graphs: IndexMap<String, IndexMap<String, IndexSet<String>>> = IndexMap::default();
        // Map<appliedName, Set<linkIdentity>>
        let mut applied_directive_to_link_identities: IndexMap<String, IndexSet<String>> = IndexMap::default();

        for link_with_graph in &supergraph.links {
            let link = &link_with_graph.link;
            if let Some(version) = link.version.as_deref().filter(|v| !v.is_empty()) {
                // `parseInt(link.version.replace("v", "").split(".")[0])`: `replace` with a
                // string pattern replaces the first occurrence only.
                let replaced = version.replacen('v', "", 1);
                let major = parse_int(replaced.split('.').next().unwrap_or(""));

                let existing = link_identity_to_major_version.get(&link.identity).copied();
                // NOTE(port): `existing !== major` is true for `NaN`, as `!=` is on `f64`.
                if let Some(existing) = existing.filter(|e| *e != major) {
                    let mut extensions = IndexMap::default();
                    extensions.insert(
                        "code".to_string(),
                        serde_json::Value::String("DIRECTIVE_COMPOSITION_ERROR".to_string()),
                    );
                    extensions.insert(
                        "versions".to_string(),
                        serde_json::Value::Array(vec![js_number_to_json(existing), js_number_to_json(major)]),
                    );
                    context.report_error(GraphQLError::with_extensions(
                        format!(
                            "Core feature \"{}\" requested to be merged has major version mismatch across subgraphs",
                            link.identity
                        ),
                        extensions,
                    ));
                } else {
                    link_identity_to_major_version.insert(link.identity.clone(), major);
                }
            }

            for im in &link.imports {
                if im.kind == LinkImportKind::Directive {
                    let applied_name = im.alias.clone().unwrap_or_else(|| im.name.clone());
                    let original_name = im.name.clone();

                    // Look for directives, their aliases and graphs.
                    applied_directive_to_graphs
                        .entry(original_name)
                        .or_default()
                        .entry(applied_name.clone())
                        .or_default()
                        .insert(link_with_graph.graph.clone());

                    // Look for directives linked from different specs
                    applied_directive_to_link_identities
                        .entry(applied_name)
                        .or_default()
                        .insert(link.identity.clone());
                }
            }
        }

        for (original_directive_name, applied_names) in &applied_directive_to_graphs {
            if applied_names.len() > 1 {
                let mut groups = Vec::new();
                for (applied_name, graphs) in applied_names {
                    let plural = if graphs.len() > 1 { "s" } else { "" };
                    groups.push(format!(
                        "name \"{applied_name}\" in subgraph{plural} \"{}\"",
                        context.graph_ids_to_names(graphs)?.join("\", \"")
                    ));
                }
                let rest = &groups[2..];
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Composed directive \"{original_directive_name}\" has incompatible name across subgraphs: it has {} but {}{}. Composed directive must have the same name across all subgraphs.",
                        groups[0],
                        groups[1],
                        if !rest.is_empty() {
                            format!(" and {}", rest.join(" and "))
                        } else {
                            String::new()
                        }
                    ),
                    "DIRECTIVE_COMPOSITION_ERROR",
                ));
            }
        }

        for (directive_name, link_identities) in &applied_directive_to_link_identities {
            if link_identities.len() > 1 {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Composed directive \"{directive_name}\" is not linked by the same core feature in every subgraph"
                    ),
                    "DIRECTIVE_COMPOSITION_ERROR",
                ));
            }
        }

        Ok(DirectiveCompositionRule)
    }
}

impl<'s> SupergraphVisitor<'s> for DirectiveCompositionRule {
    fn directive(&mut self, cx: &mut SupergraphValidationContext, directive_state: &'s DirectiveState) -> JsResult<()> {
        if directive_state.by_graph.len() == 1 || (directive_state.is_executable && !directive_state.composed) {
            return Ok(());
        }

        let graphs_size = directive_state.by_graph.len();

        let has_location_defined_in_all_graphs = directive_state.locations.iter().any(|location| {
            let mut count = 0;
            for graph_state in directive_state.by_graph.values() {
                if graph_state.locations.contains(location) {
                    count += 1;
                }
            }

            count == graphs_size
        });

        if !has_location_defined_in_all_graphs {
            cx.report_error(GraphQLError::new(format!(
                "Directive \"@{}\" has no shared locations between subgraphs",
                directive_state.name
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_number_to_json_matches_json_stringify() {
        assert_eq!(js_number_to_json(2.0).to_string(), "2");
        assert_eq!(js_number_to_json(f64::NAN).to_string(), "null");
        assert_eq!(js_number_to_json(1.5).to_string(), "1.5");
    }
}
