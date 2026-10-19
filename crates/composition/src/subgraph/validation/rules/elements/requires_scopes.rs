//! Port of `src/subgraph/validation/rules/elements/requires-scopes.ts`.

use graphql_js::{Directive, GraphQLError, Kind, NodeRef, Value, VisitAction, VisitInfo, Visitor};
use crate::graphql::printer::print;
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::state::SpecName;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};

use super::{find_annotated_node, finish, is_interface_kind, is_object_kind, node_name};

/// `RequiresScopesRule(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(node, _key, _parent, paths, ancestors)`.
/// Fields: the locals of the TS factory.
pub struct RequiresScopesRule {}

impl<'a> SubgraphRule<'a> for RequiresScopesRule {
    const NAME: &'static str = "RequiresScopesRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        RequiresScopesRule {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for RequiresScopesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "requiresScopes", cx),
            NodeRef::Directive(node) => directive(node, cx, info),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `Directive(node, _key, _parent, paths, ancestors)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> JsResult<()> {
    if !context.is_available_federation_directive("requiresScopes", &node.name.value) {
        return Ok(());
    }

    let Some(scopes_arg) = node.arguments.iter().find(|arg| arg.name.value == "scopes") else {
        return Err(JsError::error("Expected @requiresScopes to have a \"scopes\" argument"));
    };

    let Value::List(scopes_list) = &scopes_arg.value else {
        return Err(JsError::error("Expected \"@requiresScopes(scopes:)\" to be a list"));
    };

    let mut scopes: Vec<Vec<String>> = Vec::new();
    for scopes_values in &scopes_list.values {
        let Value::List(scopes_values) = scopes_values else {
            return Err(JsError::error(
                "Expected \"@requiresScopes(scopes:)\" to be in [[requiresScopes__Scope!]!]! format",
            ));
        };

        let mut scopes_or: Vec<String> = Vec::new();
        for scope in &scopes_values.values {
            let Value::String(scope) = scope else {
                return Err(JsError::error(format!(
                    "Expected \"@requiresScopes(scopes:)\" to be in [[requiresScopes__Scope!]!]! format, received [[{}]!]!",
                    print(&scopes_arg.value)
                )));
            };

            scopes_or.push(scope.value.clone());
        }
        scopes.push(scopes_or);
    }

    context.state_builder.mark_spec_as_used(SpecName::RequiresScopes);

    let parent = find_annotated_node(info, "Could not find the node annotated with @requiresScopes")?;

    match parent.kind() {
        Kind::FieldDefinition => {
            let Some(type_def) = context.type_node_info.get_type_def() else {
                return Err(JsError::error(
                    "Could not find the parent type of the field annotated with @requiresScopes",
                ));
            };

            if is_interface_kind(type_def.kind()) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Invalid use of @requiresScopes on field \"{}.{}\": @requiresScopes cannot be applied on interfaces, interface fields and interface objects",
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
                    .set_scopes(node_name(type_def), node_name(parent), &scopes)?;
            }
        }
        Kind::ObjectTypeDefinition | Kind::ObjectTypeExtension => {
            if context.state_builder.is_interface_object(node_name(parent)) {
                context.report_error(GraphQLError::with_code(
                    format!(
                        "Invalid use of @requiresScopes on interface object \"{}\": @requiresScopes cannot be applied on interfaces, interface fields and interface objects",
                        node_name(parent)
                    ),
                    "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
                ));
                return Ok(());
            }

            context.state_builder.object_type().set_scopes(node_name(parent), &scopes)?;
        }
        // NOTE(port): the TS lists `Kind.INTERFACE_TYPE_DEFINITION` twice and never
        // `INTERFACE_TYPE_EXTENSION`, so an interface extension matches no case.
        Kind::InterfaceTypeDefinition => {
            context.report_error(GraphQLError::with_code(
                format!(
                    "Invalid use of @requiresScopes on interface \"{}\": @requiresScopes cannot be applied on interfaces, interface fields and interface objects",
                    node_name(parent)
                ),
                "AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE",
            ));
            return Ok(());
        }
        Kind::ScalarTypeDefinition | Kind::ScalarTypeExtension => {
            context.state_builder.scalar_type().set_scopes(node_name(parent), &scopes)?;
        }
        Kind::EnumTypeDefinition | Kind::EnumTypeExtension => {
            context.state_builder.enum_type().set_scopes(node_name(parent), &scopes)?;
        }
        _ => {}
    }

    Ok(())
}
