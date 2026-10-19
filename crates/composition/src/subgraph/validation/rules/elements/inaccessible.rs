//! Port of `src/subgraph/validation/rules/elements/inaccessible.ts`.

use graphql_js::{Directive, Kind, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::SpecName;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};

use super::{find_annotated_node, finish, is_interface_kind, is_object_kind, node_name};

/// `InaccessibleRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct InaccessibleRules {}

impl<'a> SubgraphRule<'a> for InaccessibleRules {
    const NAME: &'static str = "InaccessibleRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        InaccessibleRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for InaccessibleRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "inaccessible", cx),
            NodeRef::Directive(node) => directive(node, cx, info),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node, _key, _parent, paths, ancestors)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> JsResult<()> {
    if !context.is_available_federation_directive("inaccessible", &node.name.value) {
        return Ok(());
    }

    let parent = find_annotated_node(info, "Could not find the node annotated with @inaccessible")?;
    let parent_name = node_name(parent);

    match parent.kind() {
        Kind::ScalarTypeDefinition | Kind::ScalarTypeExtension => {
            context.state_builder.scalar_type().set_inaccessible(parent_name)?;
        }
        Kind::FieldDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @inaccessible",
                ));
            };

            if is_interface_kind(type_def.kind()) {
                context
                    .state_builder
                    .interface_type()
                    .field()
                    .set_inaccessible(node_name(type_def), parent_name)?;
            } else {
                context
                    .state_builder
                    .object_type()
                    .field()
                    .set_inaccessible(node_name(type_def), parent_name)?;
            }
        }
        Kind::InputValueDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @inaccessible",
                ));
            };

            if matches!(type_def.kind(), Kind::InputObjectTypeDefinition | Kind::InputObjectTypeExtension) {
                context
                    .state_builder
                    .input_object_type()
                    .field()
                    .set_inaccessible(node_name(type_def), parent_name)?;
            } else if is_object_kind(type_def.kind()) {
                let Some(field_def) = context.type_node_info.get_field_def() else {
                    return Err(JsError::error(
                        "Could not find the parent field of the input value annotated with @inaccessible",
                    ));
                };

                context.state_builder.object_type().field().arg().set_inaccessible(
                    node_name(type_def),
                    node_name(field_def),
                    parent_name,
                )?;
            } else if is_interface_kind(type_def.kind()) {
                let Some(field_def) = context.type_node_info.get_field_def() else {
                    return Err(JsError::error(
                        "Could not find the parent field of the input value annotated with @inaccessible",
                    ));
                };

                context.state_builder.interface_type().field().arg().set_inaccessible(
                    node_name(type_def),
                    node_name(field_def),
                    parent_name,
                )?;
            } else if type_def.kind() == Kind::DirectiveDefinition {
                context
                    .state_builder
                    .directive()
                    .arg()
                    .set_inaccessible(node_name(type_def), parent_name)?;
            }
        }
        Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
            context.state_builder.object_type().set_inaccessible(parent_name)?;
        }
        Kind::InterfaceTypeDefinition | Kind::InterfaceTypeExtension => {
            context.state_builder.interface_type().set_inaccessible(parent_name)?;
        }
        Kind::UnionTypeDefinition | Kind::UnionTypeExtension => {
            context.state_builder.union_type().set_inaccessible(parent_name)?;
        }
        Kind::InputObjectTypeDefinition | Kind::InputObjectTypeExtension => {
            context.state_builder.input_object_type().set_inaccessible(parent_name)?;
        }
        Kind::EnumTypeDefinition | Kind::EnumTypeExtension => {
            context.state_builder.enum_type().set_inaccessible(parent_name)?;
        }
        Kind::EnumValueDefinition => {
            let enum_value = parent_name;
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the enum value annotated with @inaccessible",
                ));
            };

            context
                .state_builder
                .enum_type()
                .value()
                .set_inaccessible(node_name(type_def), enum_value)?;
        }
        _ => {}
    }

    context.state_builder.mark_spec_as_used(SpecName::Inaccessible);

    Ok(())
}
