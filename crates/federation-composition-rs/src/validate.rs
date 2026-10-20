//! Port of `src/validate.ts`.

use std::rc::Rc;

use crate::utils::collections::{IndexMap, IndexSet};
use graphql_rs::{Definition, Document, GraphQLError};

use crate::graphql::helpers::move_schema_and_directive_definitions_to_top;
use crate::specifications::federation::{detect_federation_version, DetectedFederation, FederationVersion};
use crate::specifications::link::Link;
use crate::subgraph::state::{
    clean_subgraph_state_from_federation_spec, clean_subgraph_state_from_link_spec, create_subgraph_state_builder,
    GraphInput, SubgraphStateBuilder,
};
use crate::subgraph::validation::validate_subgraph::{
    validate_subgraph as internal_validate_subgraph, validate_subgraph_core, ExtractedLinks, InternalOptions,
    SubgraphCoreInput, SubgraphInput,
};
use crate::supergraph::state::{create_supergraph_state_builder, SubgraphStates, SupergraphSpecs};
use crate::supergraph::validation::validate_supergraph::validate_supergraph;
use crate::utils::js::{locale_compare, parse_float, JsError, JsResult};

/// The `subgraph` input of `validateSubgraph`/`validate`: `{ name, url?, typeDefs }`.
pub use crate::types::ServiceDefinition as SubgraphInputDefinition;

/// `numberAtStartRegex = /^\d/`.
fn starts_with_number(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

/// An entry of `buildGraphList`'s `graphs`: `{ id, name, typeDefs, url? }`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphListEntry {
    pub id: String,
    pub name: String,
    pub type_defs: Document,
    pub url: Option<String>,
}

/// The result of `buildGraphList`: `{ success: true, graphs }` or `{ success: false, errors }`.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphList {
    Success { graphs: Vec<GraphListEntry> },
    Failure { errors: Vec<GraphQLError> },
}

/// `buildGraphList(subgraphs)`: assigns ids and sorts by id (`localeCompare`). Throws
/// `A subgraph named X already exists`.
fn build_graph_list(subgraphs: &[SubgraphInputDefinition]) -> JsResult<GraphList> {
    let errors: Vec<GraphQLError> = Vec::new();

    let mut graphs: Vec<GraphListEntry> = Vec::new();

    // Set of visited names
    let mut names: IndexSet<String> = IndexSet::default();
    // A map of created IDs and their count
    let mut id_counter: IndexMap<String, u64> = IndexMap::default();

    for subgraph in subgraphs {
        let name = subgraph.name.clone();
        let name_starts_with_number = starts_with_number(&name);

        if names.contains(&name) {
            return Err(JsError::error(format!("A subgraph named {name} already exists")));
        }

        let url = subgraph.url.clone();
        let type_defs = &subgraph.type_defs;

        names.insert(name.clone());

        // We do kind of a workaround here to make sure constantCase does not ignore special characters
        // We replace all non-alphanumeric characters with `_` and then we strip everything that is not `_` or alphanumeric
        let mut proposed_id = constant_case(&replace_non_alphanumeric(&name));

        if name_starts_with_number {
            proposed_id = format!("_{proposed_id}_");
        }

        // Check if ID was already created
        if let Some(&count) = id_counter.get(&proposed_id) {
            // It was created only once so far
            if count == 1 {
                // Let's add a `_1` suffix
                let graph = graphs
                    .iter_mut()
                    .find(|g| g.id == proposed_id)
                    .expect("a graph with the counted id exists");
                graph.id.push_str("_1");
            }

            // Push a new graph with `_N` suffix
            graphs.push(GraphListEntry {
                name,
                id: format!("{}_{}", proposed_id, count + 1),
                url,
                type_defs: move_schema_and_directive_definitions_to_top(type_defs),
            });

            // Increase the counter
            id_counter.insert(proposed_id, count + 1);
        } else {
            // It's the first time the ID was used
            id_counter.insert(proposed_id.clone(), 1);
            graphs.push(GraphListEntry {
                name,
                id: proposed_id,
                url,
                type_defs: move_schema_and_directive_definitions_to_top(type_defs),
            });
        }
    }

    if !errors.is_empty() {
        return Ok(GraphList::Failure { errors });
    }

    // @apollo/composition sorts services by name A->Z
    // We will sort them by ID to make sure the order is deterministic.
    // The sorting part is critical to some of the composition logic (e.g. descriptions on type definitions - first one wins)

    graphs.sort_by(|a, b| locale_compare(&a.id, &b.id));

    Ok(GraphList::Success { graphs })
}

/// `name.replace(/[^A-Z0-9]/gi, "_")`: every UTF-16 code unit that is not an ASCII letter or
/// digit becomes `_` (a character outside the BMP is two code units, so two `_`).
fn replace_non_alphanumeric(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
        } else {
            for _ in 0..c.len_utf16() {
                result.push('_');
            }
        }
    }
    result
}

/// The `constant-case` package's `constantCase(input, { stripRegexp: /[^A-Z0-9_]+/gi })`, as
/// `buildGraphList` calls it (after replacing `/[^A-Z0-9]/gi` with `_`).
/// NOTE(port): `no-case` 3.0.4: split with `/([a-z0-9])([A-Z])/g` then `/([A-Z])([A-Z][a-z])/g`
/// (`"$1\0$2"`), replace runs of `/[^A-Z0-9_]+/gi` with `\0`, trim `\0`, split on `\0`,
/// upper-case each part and join with `_`. All patterns are ASCII-only (no `u` flag).
fn constant_case(input: &str) -> String {
    let is_lower = |c: char| c.is_ascii_lowercase();
    let is_upper = |c: char| c.is_ascii_uppercase();

    // `/([a-z0-9])([A-Z])/g`
    let chars: Vec<char> = input.chars().collect();
    let mut step1: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && (is_lower(chars[i]) || chars[i].is_ascii_digit()) && is_upper(chars[i + 1]) {
            step1.push(chars[i]);
            step1.push('\0');
            step1.push(chars[i + 1]);
            i += 2;
        } else {
            step1.push(chars[i]);
            i += 1;
        }
    }

    // `/([A-Z])([A-Z][a-z])/g`
    let mut step2: Vec<char> = Vec::with_capacity(step1.len());
    let mut i = 0;
    while i < step1.len() {
        if i + 2 < step1.len() && is_upper(step1[i]) && is_upper(step1[i + 1]) && is_lower(step1[i + 2]) {
            step2.push(step1[i]);
            step2.push('\0');
            step2.push(step1[i + 1]);
            step2.push(step1[i + 2]);
            i += 3;
        } else {
            step2.push(step1[i]);
            i += 1;
        }
    }

    // `stripRegexp` `/[^A-Z0-9_]+/gi` -> `\0`
    let mut result = String::with_capacity(step2.len());
    let mut in_stripped_run = false;
    for c in step2 {
        if c.is_ascii_alphanumeric() || c == '_' {
            result.push(c);
            in_stripped_run = false;
        } else if !in_stripped_run {
            result.push('\0');
            in_stripped_run = true;
        }
    }

    // Trim the delimiter from around the output string, then transform each token.
    result
        .trim_matches('\0')
        .split('\0')
        .map(|token| token.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// The state both `validateSubgraph` and `validate` reach once every subgraph is validated.
struct ValidatedSubgraphs {
    subgraph_errors: Vec<GraphQLError>,
    detected_federation_spec: IndexMap<String, DetectedFederation>,
    subgraph_state_builders: IndexMap<String, SubgraphStateBuilder>,
}

/// The part shared by `validateSubgraph` and `validate`: build the graph list, validate the
/// cores, then each subgraph. `Ok(Err(errors))` is an early `errors` return.
fn validate_subgraphs(
    subgraphs: &[SubgraphInputDefinition],
    internal: Option<&InternalOptions>,
) -> JsResult<Result<ValidatedSubgraphs, Vec<GraphQLError>>> {
    let graph_list = build_graph_list(subgraphs)?;

    let mut graphs = match graph_list {
        GraphList::Failure { errors } => return Ok(Err(errors)),
        GraphList::Success { graphs } => graphs,
    };

    let core_per_subgraph: Vec<ExtractedLinks> = graphs
        .iter()
        .map(|subgraph| {
            validate_subgraph_core(SubgraphCoreInput {
                name: &subgraph.name,
                type_defs: &subgraph.type_defs,
            })
        })
        .collect();
    // Validate the basics of basics before checking subgraphs.
    let core_errors: Vec<GraphQLError> = core_per_subgraph
        .iter()
        .flat_map(|core| core.errors.clone().unwrap_or_default())
        .collect();

    // If core-level errors are detected, we skip validation of subgraphs.
    // We do it because if a core is invalid the subgraph is going to be invalid anyway.
    // By core, we mean the validation of `@link` and other fundamental logic of Federation v2
    if !core_errors.is_empty() {
        return Ok(Err(core_errors));
    }

    let mut detected_federation_spec: IndexMap<String, DetectedFederation> = IndexMap::default();
    let detected: Vec<DetectedFederation> = graphs
        .iter()
        .map(|graph| detect_federation_version(&graph.type_defs))
        .collect::<JsResult<_>>()?;
    for (graph, detected) in graphs.iter().zip(detected) {
        detected_federation_spec.insert(graph.id.clone(), detected);
    }

    // NOTE(port): `new Map(entries)`: a repeated id keeps its first position and the last value.
    let mut subgraph_state_builders: IndexMap<String, SubgraphStateBuilder> = IndexMap::default();
    for (i, graph) in graphs.iter().enumerate() {
        let builder = create_subgraph_state_builder(
            &GraphInput {
                id: graph.id.clone(),
                name: graph.name.clone(),
                url: graph.url.clone(),
            },
            &graph.type_defs,
            detected_federation_spec[&graph.id].version,
            core_per_subgraph[i].links.as_deref().unwrap_or(&[]),
        );
        subgraph_state_builders.insert(graph.id.clone(), builder);
    }

    // Validate each subgraph
    let mut subgraph_errors: Vec<GraphQLError> = Vec::new();
    for graph in graphs.iter_mut() {
        let GraphListEntry {
            id, name, type_defs, ..
        } = graph;
        let builder = subgraph_state_builders
            .get_mut(id.as_str())
            .expect("a state builder exists for every graph");
        let federation = &detected_federation_spec[id.as_str()];
        subgraph_errors.extend(internal_validate_subgraph(
            SubgraphInput { name, id, type_defs },
            builder,
            federation,
            internal,
        )?);
    }

    Ok(Ok(ValidatedSubgraphs {
        subgraph_errors,
        detected_federation_spec,
        subgraph_state_builders,
    }))
}

/// `validateSubgraph(subgraph)` (the public one in `validate.ts`).
pub fn validate_subgraph(subgraph: &SubgraphInputDefinition) -> JsResult<Vec<GraphQLError>> {
    let subgraphs = std::slice::from_ref(subgraph);
    match validate_subgraphs(subgraphs, None)? {
        Err(errors) => Ok(errors),
        Ok(validated) => Ok(validated.subgraph_errors),
    }
}

/// The success value of `validate`.
#[derive(Clone, Debug)]
pub struct ValidationSuccess {
    pub supergraph: Vec<Definition>,
    pub links: Vec<Link>,
    pub specs: SupergraphSpecs,
    pub federation_version: FederationVersion,
}

/// The result of `validate`: `{ success: true, ... }` or `{ success: false, errors }`.
#[derive(Clone, Debug)]
pub enum ValidationResult {
    Success(ValidationSuccess),
    Failure { errors: Vec<GraphQLError> },
}

/// `validate(subgraphs, __internal?)`.
pub fn validate(
    subgraphs: &[SubgraphInputDefinition],
    internal: Option<&InternalOptions>,
) -> JsResult<ValidationResult> {
    let ValidatedSubgraphs {
        subgraph_errors,
        detected_federation_spec,
        subgraph_state_builders,
    } = match validate_subgraphs(subgraphs, internal)? {
        Err(errors) => return Ok(ValidationResult::Failure { errors }),
        Ok(validated) => validated,
    };

    // If subgraph-level errors are detected, we skip validation of a supergraph.
    // We do it because if a subgraph is invalid the supergraph is going to be invalid anyway.
    if !subgraph_errors.is_empty() {
        return Ok(ValidationResult::Failure {
            errors: subgraph_errors,
        });
    }

    // We build the state of the supergraph as we validate the supergraph.
    // We do it for better performance (less iterations and shit like that).
    let mut state = create_supergraph_state_builder();
    let subgraph_states: SubgraphStates = subgraph_state_builders
        .into_iter()
        .map(|(id, builder)| {
            (
                id,
                Rc::new(clean_subgraph_state_from_federation_spec(
                    clean_subgraph_state_from_link_spec(builder.into_state()),
                )),
            )
        })
        .collect();
    let supergraph_errors = validate_supergraph(subgraph_states, &mut state, internal)?;

    // If supergraph-level errors are detected, we skip the generation of a supergraph.
    // It's corrupted anyway...
    if !supergraph_errors.is_empty() {
        return Ok(ValidationResult::Failure {
            errors: supergraph_errors,
        });
    }

    let nodes = state.build()?;

    let mut max_federation_version = FederationVersion::V1_0;

    for detected in detected_federation_spec.values() {
        if is_version_higher(detected.version, max_federation_version) {
            max_federation_version = detected.version;
        }
    }

    Ok(ValidationResult::Success(ValidationSuccess {
        supergraph: nodes,
        links: state.links(),
        specs: state.get_supergraph_state().specs.clone(),
        federation_version: max_federation_version,
    }))
}

/// `isVersionHigher(sourceVersion, targetVersion)`: `Number("2.1") > Number("2.0")`.
fn is_version_higher(source_version: FederationVersion, target_version: FederationVersion) -> bool {
    // `Number(version.replace("v", ""))`; for these strings `Number` and `parseFloat` agree.
    let source_version_parts = parse_float(&source_version.as_str().replacen('v', "", 1));
    let target_version_parts = parse_float(&target_version.as_str().replacen('v', "", 1));

    source_version_parts > target_version_parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subgraph(name: &str) -> SubgraphInputDefinition {
        SubgraphInputDefinition {
            type_defs: graphql_rs::parse("type Query { a: String }").unwrap(),
            name: name.to_string(),
            url: None,
        }
    }

    fn graph_ids(names: &[&str]) -> Vec<(String, String)> {
        let subgraphs: Vec<_> = names.iter().map(|n| subgraph(n)).collect();
        match build_graph_list(&subgraphs).unwrap() {
            GraphList::Success { graphs } => graphs.into_iter().map(|g| (g.name, g.id)).collect(),
            GraphList::Failure { .. } => panic!("unexpected failure"),
        }
    }

    // Expected values generated with `constant-case` 3.0.4 in Node, as `buildGraphList` calls it.
    #[test]
    fn constant_case_matches_constant_case_package() {
        let cases = [
            ("my-service", "MY_SERVICE"),
            ("myService", "MY_SERVICE"),
            ("Users2", "USERS2"),
            ("ABCDef", "ABC_DEF"),
            ("a__b", "A__B"),
            ("_leading", "_LEADING"),
            ("trailing_", "TRAILING_"),
            ("foo.bar baz", "FOO_BAR_BAZ"),
            ("XMLHttpRequest", "XML_HTTP_REQUEST"),
            ("v2Api", "V2_API"),
            ("123abc", "123ABC"),
            ("a1B2c3D", "A1_B2C3_D"),
            ("AaAaA", "AA_AA_A"),
            ("ÄpfelBaum", "_PFEL_BAUM"),
            ("emoji😀name", "EMOJI__NAME"),
            ("", ""),
            ("--", "__"),
            ("Product-Service_v1", "PRODUCT_SERVICE_V1"),
            ("fooBAR", "FOO_BAR"),
            ("FOOBar", "FOO_BAR"),
        ];
        for (name, expected) in cases {
            assert_eq!(constant_case(&replace_non_alphanumeric(name)), expected, "{name}");
        }
        // Without the pre-replacement, runs of other characters become one delimiter.
        assert_eq!(constant_case("foo bar"), "FOO_BAR");
        assert_eq!(constant_case("foo..Bar"), "FOO_BAR");
        assert_eq!(constant_case("\0x"), "X");
        assert_eq!(constant_case("a-B"), "A_B");
    }

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    // Expected values generated in Node with the TS `buildGraphList` logic.
    #[test]
    fn graph_list_ids_and_order() {
        assert_eq!(
            graph_ids(&["users", "1products", "Users-", "users_", "Users"]),
            pairs(&[
                ("1products", "_1PRODUCTS_"),
                ("Users-", "USERS__1"),
                ("users_", "USERS__2"),
                ("users", "USERS_1"),
                ("Users", "USERS_2"),
            ])
        );
        assert_eq!(
            graph_ids(&["a", "A", "a-", "b_c", "bC"]),
            pairs(&[
                ("a-", "A_"),
                ("a", "A_1"),
                ("A", "A_2"),
                ("b_c", "B_C_1"),
                ("bC", "B_C_2")
            ])
        );
    }

    #[test]
    fn duplicate_subgraph_name_throws() {
        let subgraphs = vec![subgraph("a"), subgraph("a")];
        assert_eq!(
            build_graph_list(&subgraphs).unwrap_err().to_js_string(),
            "Error: A subgraph named a already exists"
        );
    }

    #[test]
    fn version_comparison() {
        assert!(is_version_higher(FederationVersion::V2_1, FederationVersion::V2_0));
        assert!(!is_version_higher(FederationVersion::V2_0, FederationVersion::V2_0));
        assert!(is_version_higher(FederationVersion::V2_0, FederationVersion::V1_0));
        assert!(is_version_higher(FederationVersion::V2_9, FederationVersion::V2_8));
        assert!(!is_version_higher(FederationVersion::V1_0, FederationVersion::V2_9));
    }
}
