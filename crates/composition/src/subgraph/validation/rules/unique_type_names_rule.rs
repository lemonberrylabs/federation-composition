//! Port of `src/subgraph/validation/rules/unique-type-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::IndexSet;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueTypeNamesRule(context)`.
/// Visitor callbacks: `ScalarTypeDefinition`/`ObjectTypeDefinition`/`InterfaceTypeDefinition`/`UnionTypeDefinition`/`EnumTypeDefinition`/`InputObjectTypeDefinition: checkTypeName`.
/// Fields: the locals of the TS factory.
pub struct UniqueTypeNamesRule {
    known_type_names: IndexSet<String>,
}

impl<'a> SimpleRule<'a> for UniqueTypeNamesRule {
    const NAME: &'static str = "UniqueTypeNamesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueTypeNamesRule {
            known_type_names: IndexSet::new(),
        }
    }
}

impl UniqueTypeNamesRule {
    /// `checkTypeName(node)` (its `name`).
    fn check_type_name(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str) {
        if self.known_type_names.contains(type_name) {
            context.report_error(GraphQLError::with_code(
                format!("There can be only one type named \"{type_name}\"."),
                "INVALID_GRAPHQL",
            ));
        } else {
            self.known_type_names.insert(type_name.to_string());
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueTypeNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        if graphql_js::is_type_definition_node(node) {
            if let Some(name) = node.name() {
                self.check_type_name(cx, &name.value);
            }
        }
        VisitAction::Continue
    }
}
