//! Port of `src/subgraph/validation/rules/unique-enum-value-names-rule.ts`.

use crate::utils::collections::{IndexMap, IndexSet};
use graphql_core::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_core::EnumValueDefinition;

/// `UniqueEnumValueNamesRule(context)`.
/// Visitor callbacks: `EnumTypeDefinition`/`EnumTypeExtension: checkValueUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueEnumValueNamesRule;

/// The visitor [`UniqueEnumValueNamesRule`] returns.
pub struct UniqueEnumValueNamesVisitor<'a> {
    known_value_names: IndexMap<&'a str, IndexSet<&'a str>>,
}

impl<'a> SimpleRule<'a> for UniqueEnumValueNamesRule {
    const NAME: &'static str = "UniqueEnumValueNamesRule";
    type Visitor = UniqueEnumValueNamesVisitor<'a>;

    fn new(_context: &mut SimpleValidationContext<'a>) -> UniqueEnumValueNamesVisitor<'a> {
        UniqueEnumValueNamesVisitor {
            known_value_names: IndexMap::default(),
        }
    }
}

impl<'a> UniqueEnumValueNamesVisitor<'a> {
    /// `checkValueUniqueness(node)` for an enum definition or extension (its `name` and `values`).
    fn check_value_uniqueness(
        &mut self,
        context: &mut SimpleValidationContext<'_>,
        type_name: &'a str,
        values: &'a [EnumValueDefinition],
    ) {
        let value_names = self.known_value_names.entry(type_name).or_default();

        for value_def in values {
            let value_name = &value_def.name.value;

            if value_names.contains(value_name.as_str()) {
                context.report_error(GraphQLError::with_code(
                    format!("Enum value \"{type_name}.{value_name}\" can only be defined once."),
                    "INVALID_GRAPHQL",
                ));
            } else {
                value_names.insert(value_name);
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueEnumValueNamesVisitor<'a> {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_core::VisitKinds {
        use graphql_core::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::EnumTypeDefinition, Kind::EnumTypeExtension])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut SimpleValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        match node {
            NodeRef::EnumTypeDefinition(n) => self.check_value_uniqueness(cx, &n.name.value, &n.values),
            NodeRef::EnumTypeExtension(n) => self.check_value_uniqueness(cx, &n.name.value, &n.values),
            _ => {}
        }
        VisitAction::Continue
    }
}
