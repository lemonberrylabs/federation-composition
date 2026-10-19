//! Port of `src/supergraph/validation/rules/link-import-name-mismatch-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::validation::validation_context::SupergraphValidationContext;

/// `LinkImportNameMismatchRule(context)`: a pre-supergraph rule (runs before the subgraphs are merged).
pub fn link_import_name_mismatch_rule(context: &mut SupergraphValidationContext) {
    todo!()
}
