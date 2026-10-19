//! Port of `src/subgraph/validation/rules/known-argument-names-on-directives-rule.ts`.

use graphql_js::{Definition, GraphQLError, InputValueDefinition, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::utils::collections::IndexMap;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};

/// `KnownArgumentNamesOnDirectivesRule(context)`.
/// Visitor callbacks: `Directive`.
/// Fields: the locals of the TS factory.
pub struct KnownArgumentNamesOnDirectivesRule;

/// The visitor [`KnownArgumentNamesOnDirectivesRule`] returns.
pub struct KnownArgumentNamesOnDirectivesVisitor<'a> {
    /// PERF: `Map<string, Set<string>>` in TS; here the names borrow from the document and each
    /// value is the definition's `arguments` list. Only `.has(name)` is asked of a value, which
    /// a scan of the list answers the same way.
    directive_args: IndexMap<&'a str, &'a [InputValueDefinition]>,
}

impl<'a> SimpleRule<'a> for KnownArgumentNamesOnDirectivesRule {
    const NAME: &'static str = "KnownArgumentNamesOnDirectivesRule";
    type Visitor = KnownArgumentNamesOnDirectivesVisitor<'a>;

    fn new(context: &mut SimpleValidationContext<'a>) -> KnownArgumentNamesOnDirectivesVisitor<'a> {
        let mut directive_args: IndexMap<&'a str, &'a [InputValueDefinition]> = IndexMap::default();

        let ast_definitions = &context.get_document().definitions;
        for def in ast_definitions {
            if let Definition::DirectiveDefinition(def) = def {
                directive_args.insert(&def.name.value, &def.arguments);
            }
        }

        KnownArgumentNamesOnDirectivesVisitor { directive_args }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for KnownArgumentNamesOnDirectivesVisitor<'a> {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_js::VisitKinds {
        use graphql_js::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::Directive])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        if let NodeRef::Directive(directive_node) = node {
            let directive_name = &directive_node.name.value;
            let known_args = self.directive_args.get(directive_name.as_str());

            if let Some(known_args) = known_args {
                for arg_node in &directive_node.arguments {
                    let arg_name = &arg_node.name.value;
                    if !known_args.iter().any(|arg| arg.name.value == *arg_name) {
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
