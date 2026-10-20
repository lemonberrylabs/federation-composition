//! Port of graphql-js `ProvidedRequiredArgumentsRule`: the
//! `ProvidedRequiredArgumentsOnDirectivesRule` part, which is the SDL rule.
//!
//! NOTE(port): `ProvidedRequiredArgumentsRule` itself (fields and fragment
//! spreads in executable documents) is not ported.

use crate::ast::{Definition, InputValueDefinition, Kind, NodeRef, Type};
use crate::collections::{IndexMap, IndexSet};
use crate::error::GraphQLError;
use crate::printer::print;
use crate::r#type::definition::is_required_argument;
use crate::r#type::directives::specified_directives;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `ProvidedRequiredArgumentsOnDirectivesRule`: a directive is
/// given every required argument.
pub struct ProvidedRequiredArgumentsOnDirectivesRule {
    /// Directive name to its required arguments' names and printed types
    /// (`isType(argDef.type) ? inspect(argDef.type) : print(argDef.type)`).
    required_args_map: IndexMap<String, IndexMap<String, String>>,
}

impl ProvidedRequiredArgumentsOnDirectivesRule {
    pub fn new(context: &mut SDLValidationContext<'_>) -> Self {
        let mut required_args_map = IndexMap::default();
        // NOTE(port): no schema, so the defined directives are the specified ones.
        for directive in specified_directives() {
            required_args_map.insert(
                directive.name.clone(),
                directive
                    .args
                    .iter()
                    .filter(|arg| is_required_argument(arg))
                    .map(|arg| (arg.name.clone(), arg.ty.to_string()))
                    .collect(),
            );
        }
        for def in &context.get_document().definitions {
            if let Definition::DirectiveDefinition(def) = def {
                required_args_map.insert(
                    def.name.value.clone(),
                    def.arguments
                        .iter()
                        .filter(|arg| is_required_argument_node(arg))
                        .map(|arg| (arg.name.value.clone(), print(&arg.ty)))
                        .collect(),
                );
            }
        }
        ProvidedRequiredArgumentsOnDirectivesRule { required_args_map }
    }
}

/// graphql-js `isRequiredArgumentNode`.
fn is_required_argument_node(arg: &InputValueDefinition) -> bool {
    matches!(arg.ty, Type::NonNull(_)) && arg.default_value.is_none()
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for ProvidedRequiredArgumentsOnDirectivesRule {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter_leave(&[], &[Kind::Directive])
    }

    fn leave(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let NodeRef::Directive(directive_node) = node else {
            return VisitAction::Continue;
        };
        let directive_name = &directive_node.name.value;
        if let Some(required_args) = self.required_args_map.get(directive_name) {
            let arg_node_map: IndexSet<&str> = directive_node
                .arguments
                .iter()
                .map(|arg| arg.name.value.as_str())
                .collect();
            for (arg_name, arg_type) in required_args {
                if !arg_node_map.contains(arg_name.as_str()) {
                    context.report_error(GraphQLError::new(format!(
                        "Argument \"@{directive_name}({arg_name}:)\" of type \"{arg_type}\" is required, but it was not provided."
                    )));
                }
            }
        }
        VisitAction::Continue
    }
}
