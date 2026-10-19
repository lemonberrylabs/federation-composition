//! Port of `src/subgraph/validation/rules/unique-field-definition-names-rule.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::{IndexMap, IndexSet};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::Name;

/// `UniqueFieldDefinitionNamesRule(context)`.
/// Visitor callbacks: `InputObjectTypeDefinition`/`InputObjectTypeExtension`/`InterfaceTypeDefinition`/`InterfaceTypeExtension`/`ObjectTypeDefinition`/`ObjectTypeExtension: checkFieldUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueFieldDefinitionNamesRule {
    known_field_names: IndexMap<String, IndexSet<String>>,
}

impl<'a> SimpleRule<'a> for UniqueFieldDefinitionNamesRule {
    const NAME: &'static str = "UniqueFieldDefinitionNamesRule";

    fn new(_context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueFieldDefinitionNamesRule {
            known_field_names: IndexMap::new(),
        }
    }
}

impl UniqueFieldDefinitionNamesRule {
    /// `checkFieldUniqueness(node)`: `node.name` and the names of `node.fields` (field or input
    /// value definitions).
    fn check_field_uniqueness(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str, field_names: &[&Name]) {
        let known = self.known_field_names.entry(type_name.to_string()).or_default();

        for field_name in field_names {
            let field_name = &field_name.value;

            if known.contains(field_name) {
                context.report_error(GraphQLError::with_code(
                    format!("Field \"{type_name}.{field_name}\" can only be defined once."),
                    "INVALID_GRAPHQL",
                ));
            } else {
                known.insert(field_name.clone());
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueFieldDefinitionNamesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let (name, field_names): (&Name, Vec<&Name>) = match node {
            NodeRef::InputObjectTypeDefinition(n) => (&n.name, n.fields.iter().map(|f| &f.name).collect()),
            NodeRef::InputObjectTypeExtension(n) => (&n.name, n.fields.iter().map(|f| &f.name).collect()),
            NodeRef::InterfaceTypeDefinition(n) => (&n.name, n.fields.iter().map(|f| &f.name).collect()),
            NodeRef::InterfaceTypeExtension(n) => (&n.name, n.fields.iter().map(|f| &f.name).collect()),
            NodeRef::ObjectTypeDefinition(n) => (&n.name, n.fields.iter().map(|f| &f.name).collect()),
            NodeRef::ObjectTypeExtension(n) => (&n.name, n.fields.iter().map(|f| &f.name).collect()),
            _ => return VisitAction::Continue,
        };
        self.check_field_uniqueness(cx, &name.value, &field_names);
        VisitAction::Continue
    }
}
