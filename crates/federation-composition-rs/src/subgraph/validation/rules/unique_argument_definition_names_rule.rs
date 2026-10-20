//! Port of `src/subgraph/validation/rules/unique-argument-definition-names-rule.ts`.

use graphql_rs::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::utils::collections::IndexSet;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_rs::{FieldDefinition, InputValueDefinition};

/// `UniqueArgumentDefinitionNamesRule(context)`.
/// Visitor callbacks: `DirectiveDefinition`, `InterfaceTypeDefinition`/`InterfaceTypeExtension`/`ObjectTypeDefinition`/`ObjectTypeExtension: checkArgUniquenessPerField`.
/// Fields: the locals of the TS factory.
pub struct UniqueArgumentDefinitionNamesRule {}

impl<'a> SimpleRule<'a> for UniqueArgumentDefinitionNamesRule {
    const NAME: &'static str = "UniqueArgumentDefinitionNamesRule";
    type Visitor = Self;

    fn new(_context: &mut SimpleValidationContext<'a>) -> Self {
        UniqueArgumentDefinitionNamesRule {}
    }
}

impl UniqueArgumentDefinitionNamesRule {
    /// `checkArgUniquenessPerField(typeNode)` (its `name` and `fields`). Returns `false` in TS,
    /// which skips the node's children for this visitor.
    fn check_arg_uniqueness_per_field(&mut self, context: &mut SimpleValidationContext<'_>, type_name: &str, fields: &[FieldDefinition]) {
        for field_def in fields {
            let field_name = &field_def.name.value;
            let argument_nodes = &field_def.arguments;

            self.check_arg_uniqueness(context, format_args!("{type_name}.{field_name}"), argument_nodes);
        }
    }

    fn check_arg_uniqueness(
        &mut self,
        context: &mut SimpleValidationContext<'_>,
        // PERF: formatted only for an error message, instead of a `String` built per field.
        parent_name: std::fmt::Arguments<'_>,
        argument_nodes: &[InputValueDefinition],
    ) {
        let mut visited_argument_names: IndexSet<&str> = IndexSet::default();

        for arg_def in argument_nodes {
            if visited_argument_names.contains(arg_def.name.value.as_str()) {
                context.report_error(GraphQLError::with_code(
                    format!("Argument \"{parent_name}({}:)\" can only be defined once.", arg_def.name.value),
                    "INVALID_GRAPHQL",
                ));
            } else {
                visited_argument_names.insert(&arg_def.name.value);
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueArgumentDefinitionNamesRule {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::InterfaceTypeDefinition, Kind::InterfaceTypeExtension, Kind::ObjectTypeDefinition, Kind::ObjectTypeExtension])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::DirectiveDefinition(directive_node) => {
                let argument_nodes = &directive_node.arguments;

                self.check_arg_uniqueness(cx, format_args!("@{}", directive_node.name.value), argument_nodes);
                VisitAction::Continue
            }
            NodeRef::InterfaceTypeDefinition(n) => {
                self.check_arg_uniqueness_per_field(cx, &n.name.value, &n.fields);
                VisitAction::Skip
            }
            NodeRef::InterfaceTypeExtension(n) => {
                self.check_arg_uniqueness_per_field(cx, &n.name.value, &n.fields);
                VisitAction::Skip
            }
            NodeRef::ObjectTypeDefinition(n) => {
                self.check_arg_uniqueness_per_field(cx, &n.name.value, &n.fields);
                VisitAction::Skip
            }
            NodeRef::ObjectTypeExtension(n) => {
                self.check_arg_uniqueness_per_field(cx, &n.name.value, &n.fields);
                VisitAction::Skip
            }
            _ => VisitAction::Continue,
        }
    }
}
