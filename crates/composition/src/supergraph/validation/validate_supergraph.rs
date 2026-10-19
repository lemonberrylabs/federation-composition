//! Port of `src/supergraph/validation/validate-supergraph.ts`.

use graphql_js::GraphQLError;

use super::rules::link_import_name_mismatch_rule::link_import_name_mismatch_rule;
use super::rules::required_query_rule::required_query_rule;
use super::rules::types_of_the_same_kind_rule::types_of_the_same_kind_rule;
use super::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::subgraph::validation::validate_subgraph::InternalOptions;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::{SubgraphStates, SupergraphState, SupergraphStateBuilder};
use crate::utils::js::JsResult;

/// A pre-supergraph rule: `(context) => void`.
pub type PreSupergraphRule = fn(&mut SupergraphValidationContext);

/// `preSupergraphRules`, with the TS function names used by `disableValidationRules`.
pub const PRE_SUPERGRAPH_RULES: [(&str, PreSupergraphRule); 3] = [
    ("RequiredQueryRule", required_query_rule),
    ("TypesOfTheSameKindRule", types_of_the_same_kind_rule),
    ("LinkImportNameMismatchRule", link_import_name_mismatch_rule),
];

/// `validateSupergraph(subgraphStates, state, __internal?)`.
/// The post-supergraph rules (`postSupergraphRules`, then `selectionDependentRules` if no
/// error was reported) are `SupergraphRule` structs, built in list order with
/// [`supergraph_rule`] and visited with `visit_supergraph_state`.
pub fn validate_supergraph(
    subgraph_states: SubgraphStates,
    state: &mut SupergraphStateBuilder,
    internal: Option<&InternalOptions>,
) -> JsResult<Vec<GraphQLError>> {
    todo!()
}

/// `rulesToSkip.includes(rule.name) ? {} : rule(context, supergraph)`.
fn supergraph_rule<'s, R: SupergraphRule<'s>>(
    rules_to_skip: &[String],
    context: &mut SupergraphValidationContext,
    supergraph: &'s SupergraphState,
) -> JsResult<Box<dyn SupergraphVisitor<'s> + 's>> {
    todo!()
}

/// An empty visitor (`{}`), used for skipped rules.
struct EmptySupergraphVisitor;

impl SupergraphVisitor<'_> for EmptySupergraphVisitor {}
