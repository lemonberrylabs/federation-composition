//! Port of `src/subgraph/validation/rules/provided-required-arguments-on-directives-rule.ts`.

use graphql_js::{Definition, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::utils::collections::{IndexMap, IndexSet};

use crate::graphql::printer::print;
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::InputValueDefinition;

/// `ProvidedRequiredArgumentsOnDirectivesRule(context)`.
/// Visitor callbacks: `Directive: { leave(directiveNode) }`.
/// Fields: the locals of the TS factory.
pub struct ProvidedRequiredArgumentsOnDirectivesRule<'a> {
    /// `requiredArgsMap`: directive name to its required arguments by name.
    /// NOTE(port): plain objects in TS; GraphQL names are never integer-like, so insertion
    /// order is the iteration order.
    required_args_map: IndexMap<&'a str, IndexMap<&'a str, &'a InputValueDefinition>>,
}

impl<'a> SimpleRule<'a> for ProvidedRequiredArgumentsOnDirectivesRule<'a> {
    const NAME: &'static str = "ProvidedRequiredArgumentsOnDirectivesRule";
    type Visitor = Self;

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        let mut required_args_map: IndexMap<&'a str, IndexMap<&'a str, &'a InputValueDefinition>> = IndexMap::default();

        let ast_definitions = &context.get_document().definitions;
        for def in ast_definitions {
            if let Definition::DirectiveDefinition(def) = def {
                let required_args = def.arguments.iter().filter(|arg| is_required_argument_node(arg));
                let mut args: IndexMap<&'a str, &'a InputValueDefinition> = IndexMap::default();

                for required_arg in required_args {
                    args.insert(&required_arg.name.value, required_arg);
                }
                required_args_map.insert(&def.name.value, args);
            }
        }

        ProvidedRequiredArgumentsOnDirectivesRule { required_args_map }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for ProvidedRequiredArgumentsOnDirectivesRule<'a> {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_js::VisitKinds {
        use graphql_js::{Kind, VisitKinds};
        VisitKinds::enter_leave(&[], &[Kind::Directive])
    }

    // Validate on leave to allow for deeper errors to appear first.
    fn leave(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::Directive(directive_node) = node else {
            return VisitAction::Continue;
        };

        let directive_name = &directive_node.name.value;
        if let Some(required_args) = self.required_args_map.get(directive_name.as_str()) {
            let arg_node_map: IndexSet<&str> =
                directive_node.arguments.iter().map(|arg| arg.name.value.as_str()).collect();
            for (arg_name, arg_def) in required_args {
                if !arg_node_map.contains(arg_name) {
                    let arg_type = print(&arg_def.ty);
                    // NOTE(port): `versionInfo.major < 17` picks the graphql-js 16 message; the
                    // `graphql_js` crate is graphql-js 17.
                    let message = if graphql_js::VERSION_INFO.major < 17 {
                        format!(
                            "Directive \"@{directive_name}\" argument \"{arg_name}\" of type \"{arg_type}\" is required, but it was not provided."
                        )
                    } else {
                        format!(
                            "Argument \"@{directive_name}({arg_name}:)\" of type \"{arg_type}\" is required, but it was not provided."
                        )
                    };
                    cx.report_error(GraphQLError::with_code(message, "INVALID_GRAPHQL"));
                }
            }
        }

        VisitAction::Continue
    }
}

fn is_required_argument_node(arg: &InputValueDefinition) -> bool {
    matches!(arg.ty, graphql_js::Type::NonNull(_)) && arg.default_value.is_none()
}
