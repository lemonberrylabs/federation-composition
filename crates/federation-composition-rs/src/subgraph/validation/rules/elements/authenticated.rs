//! Port of `src/subgraph/validation/rules/elements/authenticated.ts`.

use graphql_rs::{Directive, GraphQLError, Kind, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::SpecName;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};

use super::{find_annotated_node, finish, is_interface_kind, is_object_kind, node_name};

/// `AuthenticatedRule(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct AuthenticatedRule {}

impl<'a> SubgraphRule<'a> for AuthenticatedRule {
    const NAME: &'static str = "AuthenticatedRule";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        AuthenticatedRule {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for AuthenticatedRule {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "authenticated", cx),
            NodeRef::Directive(node) => directive(node, cx, info),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node, _key, _parent, paths, ancestors)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> JsResult<()> {
    if !context.is_available_federation_directive("authenticated", &node.name.value) {
        return Ok(());
    }

    context.state_builder.mark_spec_as_used(SpecName::Authenticated);

    let parent = find_annotated_node(info, "Could not find the node annotated with @authenticated")?;

    // FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
    match parent.kind() {
        Kind::FieldDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @authenticated",
                ));
            };

            if is_interface_kind(type_def.kind()) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Invalid use of @authenticated on field \"{}.{}\": @authenticated cannot be applied on interfaces, interface fields and interface objects",
                        node_name(type_def),
                        node_name(parent)
                    ),
                    "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
                ));
                return Ok(());
            }

            if is_object_kind(type_def.kind()) {
                context
                    .state_builder
                    .object_type()
                    .field()
                    .set_authenticated(node_name(type_def), node_name(parent))?;
            }
        }
        Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
            if context.state_builder.is_interface_object(node_name(parent)) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Invalid use of @authenticated on interface object \"{}\": @authenticated cannot be applied on interfaces, interface fields and interface objects",
                        node_name(parent)
                    ),
                    "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
                ));
                return Ok(());
            }
            context.state_builder.object_type().set_authenticated(node_name(parent))?;
        }
        // NOTE(port): the TS lists `Kind.INTERFACE_TYPE_DEFINITION` twice and never
        // `INTERFACE_TYPE_EXTENSION`, so an interface extension matches no case.
        Kind::InterfaceTypeDefinition => {
            context.report_error(GraphQLError::with_code(
                format!(
                    "Invalid use of @authenticated on interface \"{}\": @authenticated cannot be applied on interfaces, interface fields and interface objects",
                    node_name(parent)
                ),
                "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
            ));
            return Ok(());
        }
        Kind::ScalarTypeDefinition | Kind::ScalarTypeExtension => {
            context.state_builder.scalar_type().set_authenticated(node_name(parent))?;
        }
        Kind::EnumTypeDefinition | Kind::EnumTypeExtension => {
            context.state_builder.enum_type().set_authenticated(node_name(parent))?;
        }
        _ => {}
    }

    Ok(())
}
