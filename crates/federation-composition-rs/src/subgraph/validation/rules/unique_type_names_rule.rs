//! Port of `src/subgraph/validation/rules/unique-type-names-rule.ts`.

use crate::utils::collections::IndexSet;
use graphql_rs::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueTypeNamesRule(context)`.
/// Visitor callbacks: `ScalarTypeDefinition`/`ObjectTypeDefinition`/`InterfaceTypeDefinition`/`UnionTypeDefinition`/`EnumTypeDefinition`/`InputObjectTypeDefinition: checkTypeName`.
/// Fields: the locals of the TS factory.
pub struct UniqueTypeNamesRule;

/// The visitor [`UniqueTypeNamesRule`] returns.
pub struct UniqueTypeNamesVisitor<'a> {
    known_type_names: IndexSet<&'a str>,
}

impl<'a> SimpleRule<'a> for UniqueTypeNamesRule {
    const NAME: &'static str = "UniqueTypeNamesRule";
    type Visitor = UniqueTypeNamesVisitor<'a>;

    fn new(_context: &mut SimpleValidationContext<'a>) -> UniqueTypeNamesVisitor<'a> {
        UniqueTypeNamesVisitor {
            known_type_names: IndexSet::default(),
        }
    }
}

impl<'a> UniqueTypeNamesVisitor<'a> {
    /// `checkTypeName(node)` (its `name`).
    fn check_type_name(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &'a str) {
        if self.known_type_names.contains(type_name) {
            context.report_error(GraphQLError::with_code(
                format!("There can be only one type named \"{type_name}\"."),
                "INVALID_GRAPHQL",
            ));
        } else {
            self.known_type_names.insert(type_name);
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueTypeNamesVisitor<'a> {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[
            Kind::ScalarTypeDefinition,
            Kind::ObjectTypeDefinition,
            Kind::InterfaceTypeDefinition,
            Kind::UnionTypeDefinition,
            Kind::EnumTypeDefinition,
            Kind::InputObjectTypeDefinition,
        ])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut SimpleValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        if graphql_rs::is_type_definition_node(node) {
            if let Some(name) = node.name() {
                self.check_type_name(cx, &name.value);
            }
        }
        VisitAction::Continue
    }
}
