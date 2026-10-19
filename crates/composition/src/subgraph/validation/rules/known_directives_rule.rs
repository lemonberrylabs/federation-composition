//! Port of `src/subgraph/validation/rules/known-directives-rule.ts`.

use graphql_js::{Definition, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::{IndexMap, IndexSet};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::{Ancestor, DirectiveLocation, OperationType};

/// `KnownDirectivesRule(context)`.
/// Visitor callbacks: `Directive(node, _key, _parent, _path, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct KnownDirectivesRule {
    locations_map: IndexMap<String, IndexSet<String>>,
}

impl<'a> SimpleRule<'a> for KnownDirectivesRule {
    const NAME: &'static str = "KnownDirectivesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        let mut locations_map: IndexMap<String, IndexSet<String>> = IndexMap::new();

        let ast_definitions = &context.get_document().definitions;
        for def in ast_definitions {
            if let Definition::DirectiveDefinition(def) = def {
                locations_map.insert(
                    def.name.value.clone(),
                    def.locations.iter().map(|name| name.value.clone()).collect(),
                );
            }
        }

        for specified_directive in graphql_js::specified_directives() {
            // If the directive is already defined by the schema, leave it be.
            if !locations_map.contains_key(&specified_directive.name.value) {
                locations_map.insert(
                    specified_directive.name.value.clone(),
                    specified_directive.locations.iter().map(|loc| loc.value.clone()).collect(),
                );
            }
        }

        KnownDirectivesRule { locations_map }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for KnownDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::Directive(node) = node else {
            return VisitAction::Continue;
        };

        let name = &node.name.value;
        let Some(locations) = self.locations_map.get(name) else {
            cx.report_error(GraphQLError::with_code(
                format!("Unknown directive \"@{name}\"."),
                "INVALID_GRAPHQL",
            ));
            return VisitAction::Continue;
        };

        let candidate_location = get_directive_location_for_ast_path(info.ancestors);
        if let Some(candidate_location) = candidate_location {
            if !locations.contains(candidate_location.as_str()) {
                cx.report_error(GraphQLError::with_code(
                    format!(
                        "Directive \"@{name}\" may not be used on {}.",
                        candidate_location.as_str()
                    ),
                    "INVALID_GRAPHQL",
                ));
            }
        }

        VisitAction::Continue
    }
}

/// `getDirectiveLocationForASTPath(ancestors)`: the location of the node the directive is
/// applied to (`ancestors[ancestors.length - 1]`).
/// NOTE(port): the TS throws `Expected a node` when that ancestor is a list; a directive's
/// last ancestor is always the node holding its `directives` list.
fn get_directive_location_for_ast_path(ancestors: &[Ancestor<'_>]) -> Option<DirectiveLocation> {
    let applied_to = ancestors
        .last()
        .and_then(|a| a.as_node())
        .expect("Expected a node");

    match applied_to {
        NodeRef::OperationDefinition(op) => Some(get_directive_location_for_operation(op.operation)),
        NodeRef::Field(_) => Some(DirectiveLocation::Field),
        NodeRef::FragmentSpread(_) => Some(DirectiveLocation::FragmentSpread),
        NodeRef::InlineFragment(_) => Some(DirectiveLocation::InlineFragment),
        NodeRef::FragmentDefinition(_) => Some(DirectiveLocation::FragmentDefinition),
        NodeRef::VariableDefinition(_) => Some(DirectiveLocation::VariableDefinition),
        NodeRef::SchemaDefinition(_) | NodeRef::SchemaExtension(_) => Some(DirectiveLocation::Schema),
        NodeRef::ScalarTypeDefinition(_) | NodeRef::ScalarTypeExtension(_) => Some(DirectiveLocation::Scalar),
        NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_) => Some(DirectiveLocation::Object),
        NodeRef::FieldDefinition(_) => Some(DirectiveLocation::FieldDefinition),
        NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_) => {
            Some(DirectiveLocation::Interface)
        }
        NodeRef::UnionTypeDefinition(_) | NodeRef::UnionTypeExtension(_) => Some(DirectiveLocation::Union),
        NodeRef::EnumTypeDefinition(_) | NodeRef::EnumTypeExtension(_) => Some(DirectiveLocation::Enum),
        NodeRef::EnumValueDefinition(_) => Some(DirectiveLocation::EnumValue),
        NodeRef::InputObjectTypeDefinition(_) | NodeRef::InputObjectTypeExtension(_) => {
            Some(DirectiveLocation::InputObject)
        }
        NodeRef::InputValueDefinition(_) => {
            let parent_node = ancestors
                .len()
                .checked_sub(3)
                .and_then(|i| ancestors[i].as_node())
                .expect("Expected a node");

            Some(if parent_node.kind() == graphql_js::Kind::InputObjectTypeDefinition {
                DirectiveLocation::InputFieldDefinition
            } else {
                DirectiveLocation::ArgumentDefinition
            })
        }
        _ => None,
    }
}

fn get_directive_location_for_operation(operation: OperationType) -> DirectiveLocation {
    match operation {
        OperationType::Query => DirectiveLocation::Query,
        OperationType::Mutation => DirectiveLocation::Mutation,
        OperationType::Subscription => DirectiveLocation::Subscription,
    }
}
