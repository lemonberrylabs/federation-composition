//! Port of `src/subgraph/validation/rules/elements/tag.ts`.

use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::SpecName;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};
use graphql_core::{Directive, Kind, NodeRef, Value, VisitAction, VisitInfo, Visitor};

use super::{find_annotated_node, finish, is_interface_kind, is_object_kind, node_name};

/// `TagRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct TagRules {}

impl<'a> SubgraphRule<'a> for TagRules {
    const NAME: &'static str = "TagRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        TagRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for TagRules {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_core::VisitKinds {
        use graphql_core::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        cx: &mut SubgraphValidationContext<'a>,
        info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "tag", cx),
            NodeRef::Directive(node) => directive(node, cx, info),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node, _key, _parent, paths, ancestors)` callback.
fn directive<'a>(
    node: &'a Directive,
    context: &mut SubgraphValidationContext<'a>,
    info: &VisitInfo<'a, '_>,
) -> JsResult<()> {
    if !context.is_available_federation_directive("tag", &node.name.value) {
        return Ok(());
    }

    let Some(name_arg) = node.arguments.iter().find(|arg| arg.name.value == "name") else {
        return Ok(());
    };

    let Value::String(name_value) = &name_arg.value else {
        return Ok(());
    };

    // TODO: T18 Support @tag on ARGUMENT_DEFINITION
    // NOTE(port): the TS message says @inaccessible here; kept verbatim.
    let parent = find_annotated_node(info, "Could not find the node annotated with @inaccessible")?;
    let parent_name = node_name(parent);

    let tag = name_value.value.as_str();

    match parent.kind() {
        Kind::ScalarTypeDefinition | Kind::ScalarTypeExtension => {
            context.state_builder.scalar_type().set_tag(parent_name, tag)?;
        }
        Kind::FieldDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @tag",
                ));
            };

            if is_interface_kind(type_def.kind()) || context.state_builder.is_interface_object(node_name(type_def)) {
                context
                    .state_builder
                    .interface_type()
                    .field()
                    .set_tag(node_name(type_def), parent_name, tag)?;
            } else {
                context
                    .state_builder
                    .object_type()
                    .field()
                    .set_tag(node_name(type_def), parent_name, tag)?;
            }
        }
        Kind::InputValueDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @tag",
                ));
            };

            if matches!(
                type_def.kind(),
                Kind::InputObjectTypeDefinition | Kind::InputObjectTypeExtension
            ) {
                context
                    .state_builder
                    .input_object_type()
                    .field()
                    .set_tag(node_name(type_def), parent_name, tag)?;
            } else if is_object_kind(type_def.kind()) {
                let Some(field_def) = context.type_node_info.get_field_def() else {
                    return Err(JsError::error(
                        "Could not find the parent field of the input value annotated with @tag",
                    ));
                };

                context.state_builder.object_type().field().arg().set_tag(
                    node_name(type_def),
                    node_name(field_def),
                    parent_name,
                    tag,
                )?;
            } else if is_interface_kind(type_def.kind()) {
                let Some(field_def) = context.type_node_info.get_field_def() else {
                    return Err(JsError::error(
                        "Could not find the parent field of the input value annotated with @tag",
                    ));
                };

                context.state_builder.interface_type().field().arg().set_tag(
                    node_name(type_def),
                    node_name(field_def),
                    parent_name,
                    tag,
                )?;
            } else if type_def.kind() == Kind::DirectiveDefinition {
                context
                    .state_builder
                    .directive()
                    .arg()
                    .set_tag(node_name(type_def), parent_name, tag)?;
            }
        }
        Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
            context.state_builder.object_type().set_tag(parent_name, tag)?;
        }
        Kind::InterfaceTypeDefinition | Kind::InterfaceTypeExtension => {
            context.state_builder.interface_type().set_tag(parent_name, tag)?;
        }
        Kind::UnionTypeDefinition | Kind::UnionTypeExtension => {
            context.state_builder.union_type().set_tag(parent_name, tag)?;
        }
        Kind::InputObjectTypeDefinition | Kind::InputObjectTypeExtension => {
            context.state_builder.input_object_type().set_tag(parent_name, tag)?;
        }
        Kind::EnumTypeDefinition | Kind::EnumTypeExtension => {
            context.state_builder.enum_type().set_tag(parent_name, tag)?;
        }
        Kind::EnumValueDefinition => {
            let enum_value = parent_name;
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the enum value annotated with @tag",
                ));
            };

            context
                .state_builder
                .enum_type()
                .value()
                .set_tag(node_name(type_def), enum_value, tag)?;
        }
        _ => {}
    }

    context.state_builder.mark_spec_as_used(SpecName::Tag);

    Ok(())
}
