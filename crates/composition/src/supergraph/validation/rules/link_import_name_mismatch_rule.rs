//! Port of `src/supergraph/validation/rules/link-import-name-mismatch-rule.ts`.

use graphql_js::GraphQLError;
use crate::utils::collections::IndexMap;

use crate::supergraph::validation::validation_context::SupergraphValidationContext;
use crate::utils::format::and_list;

/// `LinkImportNameMismatchRule(context)`: a pre-supergraph rule (runs before the subgraphs are merged).
pub fn link_import_name_mismatch_rule(context: &mut SupergraphValidationContext) {
    // "<originalName>": { "<name>": ["<subgraphName>"] }
    let mut names_to_subgraphs: IndexMap<String, IndexMap<String, Vec<String>>> = IndexMap::default();

    let names_to_check = ["@cost", "@listSize"];

    for (subgraph_id, subgraph_state) in &context.subgraph_states {
        for imp in &subgraph_state.federation.imports {
            if !names_to_check.contains(&imp.name.as_str()) {
                continue;
            }

            let existing = names_to_subgraphs.entry(imp.name.clone()).or_default();

            let name = imp.alias.clone().unwrap_or_else(|| imp.name.clone());
            let existing_name = existing.entry(name).or_default();

            existing_name.push(subgraph_id.clone());
        }
    }

    let mut errors = Vec::new();
    for (original_name, aliases) in &names_to_subgraphs {
        if aliases.len() <= 1 {
            continue;
        }

        let items: Vec<String> = aliases
            .iter()
            .map(|(name, subgraph_ids)| {
                // `subgraphIds.map(context.graphIdToName)`: the ids are keys of
                // `subgraphStates`, so the lookup cannot throw.
                let names: Vec<String> = subgraph_ids
                    .iter()
                    .map(|id| context.subgraph_states[id].graph.name.clone())
                    .collect();
                format!(
                    " \"{name}\" in subgraph{} {}",
                    if subgraph_ids.len() > 1 { "s" } else { "" },
                    and_list(&names, false, Some("\""))
                )
            })
            .collect();

        errors.push(GraphQLError::with_code(
            format!(
                "The import name \"{original_name}\" is imported with mismatched name between subgraphs: it is imported as {}",
                and_list(&items, true, None)
            ),
            "LINK_IMPORT_NAME_MISMATCH",
        ));
    }

    for error in errors {
        context.report_error(error);
    }
}
