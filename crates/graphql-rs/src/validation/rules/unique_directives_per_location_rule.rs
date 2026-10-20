//! Port of graphql-js `UniqueDirectivesPerLocationRule`.

use crate::ast::{Definition, Directive, Kind, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::predicates::{is_type_definition_node, is_type_extension_node};
use crate::r#type::directives::specified_directives;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `UniqueDirectivesPerLocationRule`: a non-repeatable directive
/// is used at most once per location (a type and its extensions, a
/// directive and its extensions, and the schema and its extensions each
/// share one location).
pub struct UniqueDirectivesPerLocationRule<'a> {
    unique_directive_map: IndexMap<String, bool>,
    schema_directives: IndexMap<&'a str, &'a Directive>,
    type_directives_map: IndexMap<&'a str, IndexMap<&'a str, &'a Directive>>,
    directive_directives_map: IndexMap<&'a str, IndexMap<&'a str, &'a Directive>>,
}

impl<'a> UniqueDirectivesPerLocationRule<'a> {
    pub fn new(context: &mut SDLValidationContext<'a>) -> Self {
        let mut unique_directive_map = IndexMap::default();
        // NOTE(port): no schema, so the defined directives are the specified ones.
        for directive in specified_directives() {
            unique_directive_map.insert(directive.name, !directive.is_repeatable);
        }
        for def in &context.get_document().definitions {
            if let Definition::DirectiveDefinition(def) = def {
                unique_directive_map.insert(def.name.value.clone(), !def.repeatable);
            }
        }
        UniqueDirectivesPerLocationRule {
            unique_directive_map,
            schema_directives: IndexMap::default(),
            type_directives_map: IndexMap::default(),
            directive_directives_map: IndexMap::default(),
        }
    }
}

/// The `directives` of nodes that have them.
fn directives_of(node: NodeRef<'_>) -> Option<&[Directive]> {
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

const KINDS_WITH_DIRECTIVES: &[Kind] = &[
    Kind::OperationDefinition,
    Kind::VariableDefinition,
    Kind::Field,
    Kind::FragmentSpread,
    Kind::InlineFragment,
    Kind::FragmentDefinition,
    Kind::SchemaDefinition,
    Kind::ScalarTypeDefinition,
    Kind::ObjectTypeDefinition,
    Kind::FieldDefinition,
    Kind::InputValueDefinition,
    Kind::InterfaceTypeDefinition,
    Kind::UnionTypeDefinition,
    Kind::EnumTypeDefinition,
    Kind::EnumValueDefinition,
    Kind::InputObjectTypeDefinition,
    Kind::DirectiveDefinition,
    Kind::SchemaExtension,
    Kind::DirectiveExtension,
    Kind::ScalarTypeExtension,
    Kind::ObjectTypeExtension,
    Kind::InterfaceTypeExtension,
    Kind::UnionTypeExtension,
    Kind::EnumTypeExtension,
    Kind::InputObjectTypeExtension,
];

impl<'a> Visitor<'a, SDLValidationContext<'a>> for UniqueDirectivesPerLocationRule<'a> {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(KINDS_WITH_DIRECTIVES)
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let Some(directives) = directives_of(node) else {
            return VisitAction::Continue;
        };
        // NOTE(port): an empty list is graphql-js's `undefined` or `[]`;
        // either way nothing below happens.
        let mut own_map = IndexMap::default();
        let seen_directives: &mut IndexMap<&'a str, &'a Directive> = match node {
            NodeRef::SchemaDefinition(_) | NodeRef::SchemaExtension(_) => &mut self.schema_directives,
            _ if is_type_definition_node(node) || is_type_extension_node(node) => {
                let type_name = node.name().expect("types have a name").value.as_str();
                self.type_directives_map.entry(type_name).or_default()
            }
            NodeRef::DirectiveDefinition(_) | NodeRef::DirectiveExtension(_) => {
                let directive_name = node.name().expect("directives have a name").value.as_str();
                self.directive_directives_map.entry(directive_name).or_default()
            }
            _ => &mut own_map,
        };
        for directive in directives {
            let directive_name = directive.name.value.as_str();
            if self.unique_directive_map.get(directive_name) == Some(&true) {
                if seen_directives.contains_key(directive_name) {
                    context.report_error(GraphQLError::new(format!(
                        "The directive \"@{directive_name}\" can only be used once at this location."
                    )));
                } else {
                    seen_directives.insert(directive_name, directive);
                }
            }
        }
        VisitAction::Continue
    }
}
