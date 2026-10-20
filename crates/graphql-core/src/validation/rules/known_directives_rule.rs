//! Port of graphql-js `KnownDirectivesRule`.

use crate::ast::{Definition, Kind, NodeRef, OperationType};
use crate::collections::IndexMap;
use crate::directive_location::DirectiveLocation;
use crate::error::GraphQLError;
use crate::r#type::directives::specified_directives;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{Ancestor, VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `KnownDirectivesRule`: every directive is defined and used in
/// one of its locations.
pub struct KnownDirectivesRule {
    locations_map: IndexMap<String, Vec<DirectiveLocation>>,
}

impl KnownDirectivesRule {
    pub fn new(context: &mut SDLValidationContext<'_>) -> Self {
        let mut locations_map = IndexMap::default();
        // NOTE(port): no schema, so the defined directives are the specified ones.
        for directive in specified_directives() {
            locations_map.insert(directive.name, directive.locations);
        }
        for def in &context.get_document().definitions {
            if let Definition::DirectiveDefinition(def) = def {
                locations_map.insert(
                    def.name.value.clone(),
                    def.locations
                        .iter()
                        // NOTE(port): graphql-js compares the names as strings. The parser
                        // accepts only directive locations, but a document built in code (or
                        // deserialized) can name anything, which no location matches.
                        .filter_map(|name| name.value.parse::<DirectiveLocation>().ok())
                        .collect(),
                );
            }
        }
        KnownDirectivesRule { locations_map }
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for KnownDirectivesRule {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::Directive])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let NodeRef::Directive(node) = node else {
            return VisitAction::Continue;
        };
        let name = &node.name.value;
        let Some(locations) = self.locations_map.get(name) else {
            context.report_error(GraphQLError::new(format!("Unknown directive \"@{name}\".")));
            return VisitAction::Continue;
        };
        let candidate_location = get_directive_location_for_ast_path(info.ancestors);
        if !locations.contains(&candidate_location) {
            context.report_error(GraphQLError::new(format!(
                "Directive \"@{name}\" may not be used on {candidate_location}."
            )));
        }
        VisitAction::Continue
    }
}

fn ancestor_node<'a>(ancestors: &[Ancestor<'a>], from_end: usize) -> Option<NodeRef<'a>> {
    ancestors
        .len()
        .checked_sub(from_end)
        .and_then(|i| ancestors[i].as_node())
}

/// graphql-js `getDirectiveLocationForASTPath`.
fn get_directive_location_for_ast_path(ancestors: &[Ancestor<'_>]) -> DirectiveLocation {
    // TS: invariant(appliedTo != null && 'kind' in appliedTo); a directive is
    // always held by a node's `directives` list.
    let applied_to = ancestor_node(ancestors, 1).expect("Unexpected invariant triggered.");
    match applied_to {
        NodeRef::OperationDefinition(op) => get_directive_location_for_operation(op.operation),
        NodeRef::Field(_) => DirectiveLocation::Field,
        NodeRef::FragmentSpread(_) => DirectiveLocation::FragmentSpread,
        NodeRef::InlineFragment(_) => DirectiveLocation::InlineFragment,
        NodeRef::FragmentDefinition(_) => DirectiveLocation::FragmentDefinition,
        NodeRef::VariableDefinition(_) => {
            let parent_node = ancestor_node(ancestors, 3).expect("Unexpected invariant triggered.");
            if parent_node.kind() == Kind::OperationDefinition {
                DirectiveLocation::VariableDefinition
            } else {
                DirectiveLocation::FragmentVariableDefinition
            }
        }
        NodeRef::SchemaDefinition(_) | NodeRef::SchemaExtension(_) => DirectiveLocation::Schema,
        NodeRef::ScalarTypeDefinition(_) | NodeRef::ScalarTypeExtension(_) => DirectiveLocation::Scalar,
        NodeRef::ObjectTypeDefinition(_) | NodeRef::ObjectTypeExtension(_) => DirectiveLocation::Object,
        NodeRef::FieldDefinition(_) => DirectiveLocation::FieldDefinition,
        NodeRef::InterfaceTypeDefinition(_) | NodeRef::InterfaceTypeExtension(_) => DirectiveLocation::Interface,
        NodeRef::UnionTypeDefinition(_) | NodeRef::UnionTypeExtension(_) => DirectiveLocation::Union,
        NodeRef::EnumTypeDefinition(_) | NodeRef::EnumTypeExtension(_) => DirectiveLocation::Enum,
        NodeRef::EnumValueDefinition(_) => DirectiveLocation::EnumValue,
        NodeRef::InputObjectTypeDefinition(_) | NodeRef::InputObjectTypeExtension(_) => DirectiveLocation::InputObject,
        NodeRef::InputValueDefinition(_) => {
            let parent_node = ancestor_node(ancestors, 3).expect("Unexpected invariant triggered.");
            match parent_node.kind() {
                Kind::InputObjectTypeDefinition | Kind::InputObjectTypeExtension => {
                    DirectiveLocation::InputFieldDefinition
                }
                _ => DirectiveLocation::ArgumentDefinition,
            }
        }
        NodeRef::DirectiveDefinition(_) | NodeRef::DirectiveExtension(_) => DirectiveLocation::DirectiveDefinition,
        // TS: invariant(false, 'Unexpected kind: ' + inspect(appliedTo.kind));
        // no other node has directives.
        other => panic!("Unexpected kind: \"{}\"", other.kind()),
    }
}

/// graphql-js `getDirectiveLocationForOperation`.
fn get_directive_location_for_operation(operation: OperationType) -> DirectiveLocation {
    match operation {
        OperationType::Query => DirectiveLocation::Query,
        OperationType::Mutation => DirectiveLocation::Mutation,
        OperationType::Subscription => DirectiveLocation::Subscription,
    }
}
