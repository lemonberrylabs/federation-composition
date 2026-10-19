//! Port of `src/subgraph/validation/rules/elements/compose-directive.ts`.

use graphql_js::{Definition, Directive, GraphQLError, NodeRef, Value, VisitAction, VisitInfo, Visitor};
use indexmap::IndexMap;
use crate::specifications::link::LinkImportKind;
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::JsResult;

use super::finish;

/// `ComposeDirectiveRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct ComposeDirectiveRules {}

impl<'a> SubgraphRule<'a> for ComposeDirectiveRules {
    const NAME: &'static str = "ComposeDirectiveRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        ComposeDirectiveRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ComposeDirectiveRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "composeDirective", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// `str.replace(/^@/, "")`.
fn strip_at(value: &str) -> &str {
    value.strip_prefix('@').unwrap_or(value)
}

/// `{ code: "DIRECTIVE_COMPOSITION_ERROR", subgraphName }`.
fn composition_error(message: String, subgraph_name: &str) -> GraphQLError {
    let mut extensions = IndexMap::new();
    extensions.insert(
        "code".to_string(),
        serde_json::Value::String("DIRECTIVE_COMPOSITION_ERROR".to_string()),
    );
    extensions.insert(
        "subgraphName".to_string(),
        serde_json::Value::String(subgraph_name.to_string()),
    );
    GraphQLError::with_extensions(message, extensions)
}

/// The `Directive(node)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("composeDirective", &node.name.value) {
        return Ok(());
    }

    if !context.satisfies_version_range(">= v2.1") {
        return Ok(());
    }

    let name_arg = node.arguments.iter().find(|arg| arg.name.value == "name");

    let Some(Value::String(name_value)) = name_arg.map(|arg| &arg.value) else {
        return Ok(());
    };

    let name = strip_at(&name_value.value);

    let document = context.get_document();
    let matching_directive = document.definitions.iter().find_map(|definition| match definition {
        Definition::DirectiveDefinition(directive) if directive.name.value == name => Some(directive),
        _ => None,
    });

    if let Some(matching_directive) = matching_directive {
        // check if it has a linked spec
        let has_spec = context.state_builder.state.links.iter().any(|link| {
            link.imports.iter().any(|im| {
                im.kind == LinkImportKind::Directive
                    && match &im.alias {
                        // NOTE(port): `im.alias ? ...` is a truthiness test; an empty alias falls
                        // back to the name.
                        Some(alias) if !alias.is_empty() => strip_at(alias) == name,
                        _ => strip_at(&im.name) == name,
                    }
            })
        });

        if !has_spec {
            let subgraph_name = context.get_subgraph_name();
            context.report_error(composition_error(
                format!(
                    "Directive \"@{name}\" in subgraph \"{subgraph_name}\" cannot be composed because it is not a member of a core feature"
                ),
                subgraph_name,
            ));
            return Ok(());
        }

        context.state_builder.directive().set_composed(&matching_directive.name.value)?;
        context
            .state_builder
            .composed_directives
            .insert(matching_directive.name.value.clone());
    } else {
        let subgraph_name = context.get_subgraph_name();
        context.report_error(composition_error(
            format!(
                "Could not find matching directive definition for argument to @composeDirective \"@{name}\" in subgraph \"{subgraph_name}\"."
            ),
            subgraph_name,
        ));
    }

    Ok(())
}
