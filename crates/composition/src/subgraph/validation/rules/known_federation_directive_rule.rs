//! Port of `src/subgraph/validation/rules/known-federation-directive-rule.ts`.

use graphql_js::{Definition, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::IndexSet;

use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};

/// `KnownFederationDirectivesRule(context)`.
/// Visitor callbacks: `Directive`.
/// Fields: the locals of the TS factory.
pub struct KnownFederationDirectivesRule {
    available_directives_set: IndexSet<String>,
    known_directives_set: IndexSet<String>,
}

impl<'a> SubgraphRule<'a> for KnownFederationDirectivesRule {
    const NAME: &'static str = "KnownFederationDirectivesRule";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        let mut available_directives_set: IndexSet<String> = IndexSet::new();
        let mut known_directives_set: IndexSet<String> = IndexSet::new();

        let known_directives = context.get_known_federation_directives();
        for directive in known_directives {
            known_directives_set.insert(directive.name.value.clone());
        }

        let available_directives = context.get_available_federation_directives();
        for directive in available_directives {
            available_directives_set.insert(directive.name.value.clone());
        }

        let ast_definitions = &context.get_document().definitions;
        for def in ast_definitions {
            if let Definition::DirectiveDefinition(def) = def {
                available_directives_set.insert(def.name.value.clone());
            }
        }

        KnownFederationDirectivesRule {
            available_directives_set,
            known_directives_set,
        }
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for KnownFederationDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::Directive(node) = node else {
            return VisitAction::Continue;
        };

        let name = &node.name.value;

        if !self.available_directives_set.contains(name) && name == "interfaceObject" {
            cx.report_error(GraphQLError::with_code(
                "Unknown directive \"@interfaceObject\". If you meant the \"@interfaceObject\" federation 2 directive, note that this schema is a federation 1 schema. To be a federation 2 schema, it needs to @link to the federation specification v2.",
                "INVALID_GRAPHQL",
            ));
            return VisitAction::Continue;
        }

        if !self.available_directives_set.contains(name)
            && self.known_directives_set.contains(name)
            && !name.starts_with("federation__")
        {
            cx.report_error(GraphQLError::with_code(
                format!(
                    "Unknown directive \"@{name}\". If you meant the \"@{name}\" federation directive, you should use fully-qualified name \"@federation__{name}\" or add \"@{name}\" to the `import` argument of the @link to the federation specification."
                ),
                "INVALID_GRAPHQL",
            ));
            return VisitAction::Continue;
        }

        VisitAction::Continue
    }
}
