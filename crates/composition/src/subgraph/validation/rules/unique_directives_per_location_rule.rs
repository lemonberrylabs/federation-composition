//! Port of `src/subgraph/validation/rules/unique-directives-per-location-rule.ts`.

use graphql_js::{Definition, Directive, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::utils::collections::{IndexMap, IndexSet};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `UniqueDirectivesPerLocationRule(context)`.
/// Visitor callbacks: generic `enter(node)`.
/// Fields: the locals of the TS factory.
pub struct UniqueDirectivesPerLocationRule;

/// The visitor [`UniqueDirectivesPerLocationRule`] returns.
pub struct UniqueDirectivesPerLocationVisitor<'a> {
    unique_directive_map: IndexMap<&'a str, bool>,
    schema_directives: IndexSet<&'a str>,
    type_directives_map: IndexMap<&'a str, IndexSet<&'a str>>,
}

impl<'a> SimpleRule<'a> for UniqueDirectivesPerLocationRule {
    const NAME: &'static str = "UniqueDirectivesPerLocationRule";
    type Visitor = UniqueDirectivesPerLocationVisitor<'a>;

    fn new(context: &mut SimpleValidationContext<'a>) -> UniqueDirectivesPerLocationVisitor<'a> {
        let mut unique_directive_map: IndexMap<&'a str, bool> = IndexMap::default();

        let ast_definitions = &context.get_document().definitions;
        for def in ast_definitions {
            if let Definition::DirectiveDefinition(def) = def {
                unique_directive_map.insert(&def.name.value, !def.repeatable);
            }
        }

        UniqueDirectivesPerLocationVisitor {
            unique_directive_map,
            schema_directives: IndexSet::default(),
            type_directives_map: IndexMap::default(),
        }
    }
}

/// `node.directives` for every node kind that has a `directives` field.
fn node_directives(node: NodeRef<'_>) -> Option<&[Directive]> {
    Some(match node {
        NodeRef::OperationDefinition(n) => &n.directives,
        NodeRef::VariableDefinition(n) => &n.directives,
        NodeRef::Field(n) => &n.directives,
        NodeRef::FragmentSpread(n) => &n.directives,
        NodeRef::InlineFragment(n) => &n.directives,
        NodeRef::FragmentDefinition(n) => &n.directives,
        NodeRef::SchemaDefinition(n) => &n.directives,
        NodeRef::ScalarTypeDefinition(n) => &n.directives,
        NodeRef::ObjectTypeDefinition(n) => &n.directives,
        NodeRef::FieldDefinition(n) => &n.directives,
        NodeRef::InputValueDefinition(n) => &n.directives,
        NodeRef::InterfaceTypeDefinition(n) => &n.directives,
        NodeRef::UnionTypeDefinition(n) => &n.directives,
        NodeRef::EnumTypeDefinition(n) => &n.directives,
        NodeRef::EnumValueDefinition(n) => &n.directives,
        NodeRef::InputObjectTypeDefinition(n) => &n.directives,
        NodeRef::DirectiveDefinition(n) => &n.directives,
        NodeRef::SchemaExtension(n) => &n.directives,
        NodeRef::DirectiveExtension(n) => &n.directives,
        NodeRef::ScalarTypeExtension(n) => &n.directives,
        NodeRef::ObjectTypeExtension(n) => &n.directives,
        NodeRef::InterfaceTypeExtension(n) => &n.directives,
        NodeRef::UnionTypeExtension(n) => &n.directives,
        NodeRef::EnumTypeExtension(n) => &n.directives,
        NodeRef::InputObjectTypeExtension(n) => &n.directives,
        _ => return None,
    })
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for UniqueDirectivesPerLocationVisitor<'a> {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_js::VisitKinds {
        use graphql_js::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::OperationDefinition, Kind::VariableDefinition, Kind::Field, Kind::FragmentSpread, Kind::InlineFragment, Kind::FragmentDefinition, Kind::SchemaDefinition, Kind::ScalarTypeDefinition, Kind::ObjectTypeDefinition, Kind::FieldDefinition, Kind::InputValueDefinition, Kind::InterfaceTypeDefinition, Kind::UnionTypeDefinition, Kind::EnumTypeDefinition, Kind::EnumValueDefinition, Kind::InputObjectTypeDefinition, Kind::DirectiveDefinition, Kind::SchemaExtension, Kind::DirectiveExtension, Kind::ScalarTypeExtension, Kind::ObjectTypeExtension, Kind::InterfaceTypeExtension, Kind::UnionTypeExtension, Kind::EnumTypeExtension, Kind::InputObjectTypeExtension])
    }

    // Many different AST nodes may contain directives. Rather than listing
    // them all, just listen for entering any node, and check to see if it
    // defines any directives.
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        // NOTE(port): an absent `directives` and an empty list behave the same here.
        let Some(directives) = node_directives(node) else {
            return VisitAction::Continue;
        };

        let mut local_seen: IndexSet<&'a str> = IndexSet::default();
        let seen_directives: &mut IndexSet<&'a str> =
            if matches!(node, NodeRef::SchemaDefinition(_) | NodeRef::SchemaExtension(_)) {
                &mut self.schema_directives
            } else if graphql_js::is_type_definition_node(node) || graphql_js::is_type_extension_node(node) {
                let type_name = node.name().map(|n| n.value.as_str()).unwrap_or_default();
                self.type_directives_map.entry(type_name).or_default()
            } else {
                &mut local_seen
            };

        for directive in directives {
            let directive_name = &directive.name.value;

            if self.unique_directive_map.get(directive_name.as_str()).copied().unwrap_or(false) {
                if seen_directives.contains(directive_name.as_str()) {
                    cx.report_error(GraphQLError::with_code(
                        format!("The directive \"@{directive_name}\" can only be used once at this location."),
                        "INVALID_GRAPHQL",
                    ));
                } else {
                    seen_directives.insert(&directive.name.value);
                }
            }
        }

        VisitAction::Continue
    }
}
