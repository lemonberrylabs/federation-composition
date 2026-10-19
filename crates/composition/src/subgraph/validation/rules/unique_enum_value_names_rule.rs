//! Port of `src/subgraph/validation/rules/unique-enum-value-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::utils::collections::{IndexMap, IndexSet};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::EnumValueDefinition;

/// `UniqueEnumValueNamesRule(context)`.
/// Visitor callbacks: `EnumTypeDefinition`/`EnumTypeExtension: checkValueUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueEnumValueNamesRule {
    known_value_names: IndexMap<String, IndexSet<String>>,
}

impl<'a> SimpleRule<'a> for UniqueEnumValueNamesRule {
    const NAME: &'static str = "UniqueEnumValueNamesRule";

    fn new(_context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueEnumValueNamesRule {
            known_value_names: IndexMap::default(),
        }
    }
}

impl UniqueEnumValueNamesRule {
    /// `checkValueUniqueness(node)` for an enum definition or extension (its `name` and `values`).
    fn check_value_uniqueness(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str, values: &[EnumValueDefinition]) {
        let value_names = self.known_value_names.entry(type_name.to_string()).or_default();

        for value_def in values {
            let value_name = &value_def.name.value;

            if value_names.contains(value_name) {
                context.report_error(GraphQLError::with_code(
                    format!("Enum value \"{type_name}.{value_name}\" can only be defined once."),
                    "INVALID_GRAPHQL",
                ));
            } else {
                value_names.insert(value_name.clone());
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueEnumValueNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::EnumTypeDefinition(n) => self.check_value_uniqueness(cx, &n.name.value, &n.values),
            NodeRef::EnumTypeExtension(n) => self.check_value_uniqueness(cx, &n.name.value, &n.values),
            _ => {}
        }
        VisitAction::Continue
    }
}
