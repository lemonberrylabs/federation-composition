//! Port of `src/subgraph/validation/rules/elements/override.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use graphql_js::Value;

/// `OverrideRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct OverrideRules {}

impl<'a> SubgraphRule<'a> for OverrideRules {
    const NAME: &'static str = "OverrideRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for OverrideRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}

/// `labelNonPercentRegex`.
const LABEL_NON_PERCENT_REGEX: &str = r"^[a-zA-Z][a-zA-Z0-9_\-:./]*$";
/// `labelPercentRegex`.
const LABEL_PERCENT_REGEX: &str = r"^percent\(((\d{1,2}(\.\d{1,8})?)|100(\.[0]{1,8})?)\)$";

fn invalid_label_value_error(value: &str, coordinate: &str, subgraph_name: &str) -> String {
    format!(
        "Invalid @override label {value} on field \"{coordinate}\" on subgraph \"{subgraph_name}\": labels must start with a letter and after that may contain alphanumerics, underscores, minuses, colons, periods, or slashes. Alternatively, labels may be of the form \"percent(x)\" where x is a float between 0-100 inclusive."
    )
}

/// `isValidLabel(valueNode)`. NOTE(port): JS `\d` is ASCII-only, like `regex`'s `[0-9]`;
/// use `[0-9]` or `(?-u:\d)` when porting the patterns.
fn is_valid_label(value_node: &Value) -> bool {
    todo!()
}
