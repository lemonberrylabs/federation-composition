//! Port of `src/subgraph/validation/rules/unique-field-definition-names-rule.ts`.

use graphql_rs::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::utils::collections::{IndexMap, IndexSet};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_rs::Name;

/// `UniqueFieldDefinitionNamesRule(context)`.
/// Visitor callbacks: `InputObjectTypeDefinition`/`InputObjectTypeExtension`/`InterfaceTypeDefinition`/`InterfaceTypeExtension`/`ObjectTypeDefinition`/`ObjectTypeExtension: checkFieldUniqueness`.
/// Fields: the locals of the TS factory.
pub struct UniqueFieldDefinitionNamesRule;

/// The visitor [`UniqueFieldDefinitionNamesRule`] returns.
pub struct UniqueFieldDefinitionNamesVisitor<'a> {
    known_field_names: IndexMap<&'a str, IndexSet<&'a str>>,
}

impl<'a> SimpleRule<'a> for UniqueFieldDefinitionNamesRule {
    const NAME: &'static str = "UniqueFieldDefinitionNamesRule";
    type Visitor = UniqueFieldDefinitionNamesVisitor<'a>;

    fn new(_context: &mut SimpleValidationContext<'a>) -> UniqueFieldDefinitionNamesVisitor<'a> {
        UniqueFieldDefinitionNamesVisitor {
            known_field_names: IndexMap::default(),
        }
    }
}

impl<'a> UniqueFieldDefinitionNamesVisitor<'a> {
    /// `checkFieldUniqueness(node)`: `node.name` and the names of `node.fields` (field or input
    /// value definitions).
    fn check_field_uniqueness(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &'a str, field_names: &[&'a Name]) {
        let known = self.known_field_names.entry(type_name).or_default();

        for field_name in field_names {
            let field_name = &field_name.value;

            if known.contains(field_name.as_str()) {
                context.report_error(GraphQLError::with_code(
                    format!("Field \"{type_name}.{field_name}\" can only be defined once."),
                    "INVALID_GRAPHQL",
                ));
            } else {
                known.insert(field_name);
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueFieldDefinitionNamesVisitor<'a> {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::InputObjectTypeDefinition, Kind::InputObjectTypeExtension, Kind::InterfaceTypeDefinition, Kind::InterfaceTypeExtension, Kind::ObjectTypeDefinition, Kind::ObjectTypeExtension])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let (name, field_names): (&'a Name, Vec<&'a Name>) = match node {
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
