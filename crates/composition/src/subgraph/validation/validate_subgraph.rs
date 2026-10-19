//! Port of `src/subgraph/validation/validate-subgraph.ts`.
//!
//! Visits: the subgraph document is visited once with
//! `visit_with_type_node_info(visit_in_parallel([state builder visitor, ...federation rules]))`
//! and the `SubgraphValidationContext` as `cx`; then the full type definitions
//! (federation definitions + link spec + subgraph) are visited with
//! `visit_in_parallel(graphql rules)` and the `SimpleValidationContext`. A skipped rule
//! (`__internal.disableValidationRules` contains its `NAME`) is an empty visitor (`{}`).
//! A visitor that throws records the error in the context (`cx.throw`); after the visit,
//! `validate_subgraph` returns it as `Err`.

use graphql_js::{Document, GraphQLError};
use indexmap::IndexMap;

use super::validation_context::{SimpleRule, SimpleValidationContext, SubgraphRule, SubgraphValidationContext};
use crate::specifications::federation::DetectedFederation;
use crate::specifications::link::Link;
use crate::subgraph::state::SubgraphStateBuilder;
use crate::utils::js::JsResult;

/// The `subgraph` of `validateSubgraphCore`: `{ name, typeDefs }`.
#[derive(Clone, Copy, Debug)]
pub struct SubgraphCoreInput<'a> {
    pub name: &'a str,
    pub type_defs: &'a Document,
}

/// The `subgraph` of `validateSubgraph`: `{ name, typeDefs, id }`. `type_defs` is mutable
/// because `cleanSubgraphTypeDefsFromSubgraphSpec` edits the document in place (and the
/// caller's graph sees the edit, as in TS).
#[derive(Debug)]
pub struct SubgraphInput<'a> {
    pub name: &'a str,
    pub id: &'a str,
    pub type_defs: &'a mut Document,
}

/// `__internal?: { disableValidationRules?: string[] }`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InternalOptions {
    pub disable_validation_rules: Option<Vec<String>>,
}

/// The result of `extractLinks`/`validateSubgraphCore`: `{ links }` or `{ errors }`.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtractedLinks {
    /// `links` (`None` when `errors` is set).
    pub links: Option<Vec<Link>>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// `assertUniqueSubgraphNames(subgraphs)`: throws `A subgraph named X already exists`.
pub fn assert_unique_subgraph_names<'n>(subgraphs: impl IntoIterator<Item = &'n str>) -> JsResult<()> {
    todo!()
}

/// `validateSubgraphCore(subgraph)`: `@link` validation, errors enriched with the subgraph name.
pub fn validate_subgraph_core(subgraph: SubgraphCoreInput<'_>) -> ExtractedLinks {
    todo!()
}

/// `validateSubgraph(subgraph, stateBuilder, federation, __internal?)`.
/// `Err` is a throw escaping the TS function (from a visitor or the state builder).
pub fn validate_subgraph(
    subgraph: SubgraphInput<'_>,
    state_builder: &mut SubgraphStateBuilder,
    federation: &DetectedFederation,
    internal: Option<&InternalOptions>,
) -> JsResult<Vec<GraphQLError>> {
    todo!()
}

/// The `linkSpecDefinitions` SDL parsed inside `validateSubgraph`.
const LINK_SPEC_DEFINITIONS_SDL: &str = r#"
    enum Purpose {
      EXECUTION
      SECURITY
    }

    directive @link(
      url: String
      as: String
      for: link__Purpose
      import: [link__Import]
    ) repeatable on SCHEMA

    scalar link__Import

    enum link__Purpose {
      """
      `SECURITY` features provide metadata necessary to securely resolve fields.
      """
      SECURITY

      """
      `EXECUTION` features provide metadata necessary for operation execution.
      """
      EXECUTION
    }
  "#;

/// Boxes a federation rule, or an empty visitor when the rule is disabled.
/// `federationRules.map((rule) => rulesToSkip.includes(rule.name) ? {} : rule(validationContext))`.
fn federation_rule<'a, R: SubgraphRule<'a>>(
    rules_to_skip: &[String],
    context: &mut SubgraphValidationContext<'a>,
) -> Box<dyn graphql_js::Visitor<'a, SubgraphValidationContext<'a>> + 'a> {
    todo!()
}

/// Same as [`federation_rule`] for the graphql rules.
fn graphql_rule<'a, R: SimpleRule<'a>>(
    rules_to_skip: &[String],
    context: &mut SimpleValidationContext<'a>,
) -> Box<dyn graphql_js::Visitor<'a, SimpleValidationContext<'a>> + 'a> {
    todo!()
}

/// `enrichErrorWithSubgraphName(error, subgraphName)`: prefixes `[name] ` and sets
/// `extensions.subgraphName`, unless already set (truthy).
fn enrich_error_with_subgraph_name(error: GraphQLError, subgraph_name: &str) -> GraphQLError {
    todo!()
}

/// `availableFeatures`: known versions of non-federation Apollo specs.
fn available_features() -> IndexMap<&'static str, &'static [&'static str]> {
    IndexMap::from([
        ("link", &["v1.0"][..]),
        ("tag", &["v0.1", "v0.2"][..]),
        ("kotlin_labs", &["v0.1", "v0.2"][..]),
        ("join", &["v0.1", "v0.2", "v0.3", "v0.4", "v0.5"][..]),
        ("inaccessible", &["v0.1", "v0.2"][..]),
        ("core", &["v0.1", "v0.2"][..]),
    ])
}

/// `extractLinks(subgraph)`. The `try/catch` around each link turns a throw from
/// `parseLinkDirective`/`createSpecSchema` into an error (`error instanceof GraphQLError ?
/// error : new GraphQLError(String(error))`, i.e. `JsError::into_graphql_error`).
fn extract_links(subgraph: SubgraphCoreInput<'_>) -> ExtractedLinks {
    todo!()
}

fn is_schema_definition_or_extension_node(node: &graphql_js::Definition) -> bool {
    matches!(
        node,
        graphql_js::Definition::SchemaDefinition(_) | graphql_js::Definition::SchemaExtension(_)
    )
}

/// `onlyDocumentNode(item)`.
fn only_document_node(item: Option<&Document>) -> bool {
    item.is_some()
}

/// `cleanSubgraphTypeDefsFromSubgraphSpec(typeDefs)`: removes `_Any`, `_Entity`, `_Service`
/// and the `_service`/`_entities` query fields, in place.
fn clean_subgraph_type_defs_from_subgraph_spec(type_defs: &mut Document) {
    todo!()
}
