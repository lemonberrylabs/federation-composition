//! Port of `src/subgraph/validation/rules/known-argument-names-on-directives-rule.ts`.

use graphql_js::{Definition, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::{IndexMap, IndexSet};

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `KnownArgumentNamesOnDirectivesRule(context)`.
/// Visitor callbacks: `Directive`.
/// Fields: the locals of the TS factory.
pub struct KnownArgumentNamesOnDirectivesRule {
    directive_args: IndexMap<String, IndexSet<String>>,
}

impl<'a> SimpleRule<'a> for KnownArgumentNamesOnDirectivesRule {
    const NAME: &'static str = "KnownArgumentNamesOnDirectivesRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        let mut directive_args: IndexMap<String, IndexSet<String>> = IndexMap::new();

        let ast_definitions = &context.get_document().definitions;
        for def in ast_definitions {
            if let Definition::DirectiveDefinition(def) = def {
                directive_args.insert(
                    def.name.value.clone(),
                    def.arguments.iter().map(|arg| arg.name.value.clone()).collect(),
                );
            }
        }

        KnownArgumentNamesOnDirectivesRule { directive_args }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for KnownArgumentNamesOnDirectivesRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::Directive(directive_node) = node {
            let directive_name = &directive_node.name.value;
            let known_args = self.directive_args.get(directive_name);

            if let Some(known_args) = known_args {
                for arg_node in &directive_node.arguments {
                    let arg_name = &arg_node.name.value;
                    if !known_args.contains(arg_name) {
                        cx.report_error(GraphQLError::with_code(
                            format!("Unknown argument \"{arg_name}\" on directive \"@{directive_name}\"."),
                            "INVALID_GRAPHQL",
                        ));
                    }
                }
            }
        }
        VisitAction::Continue
    }
}
