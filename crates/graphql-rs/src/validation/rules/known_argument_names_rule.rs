//! Port of graphql-js `KnownArgumentNamesRule`: the
//! `KnownArgumentNamesOnDirectivesRule` part, which is the SDL rule.
//!
//! NOTE(port): `KnownArgumentNamesRule` itself (arguments of fields and
//! fragments in executable documents) is not ported.

use crate::ast::{Definition, Kind, NodeRef};
use crate::collections::IndexMap;
use crate::error::GraphQLError;
use crate::jsutils::did_you_mean::did_you_mean;
use crate::jsutils::suggestion_list::suggestion_list;
use crate::r#type::directives::specified_directives;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `KnownArgumentNamesOnDirectivesRule`: every argument of a
/// directive is defined by the directive.
pub struct KnownArgumentNamesOnDirectivesRule {
    directive_args: IndexMap<String, Vec<String>>,
}

impl KnownArgumentNamesOnDirectivesRule {
    pub fn new(context: &mut SDLValidationContext<'_>) -> Self {
        let mut directive_args = IndexMap::default();
        // NOTE(port): no schema, so the defined directives are the specified ones.
        for directive in specified_directives() {
            directive_args.insert(directive.name, directive.args.into_iter().map(|arg| arg.name).collect());
        }
        for def in &context.get_document().definitions {
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

impl<'a> Visitor<'a, SDLValidationContext<'a>> for KnownArgumentNamesOnDirectivesRule {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::Directive])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        let NodeRef::Directive(directive_node) = node else {
            return VisitAction::Continue;
        };
        let directive_name = &directive_node.name.value;
        if let Some(known_args) = self.directive_args.get(directive_name) {
            for arg_node in &directive_node.arguments {
                let arg_name = &arg_node.name.value;
                if !known_args.contains(arg_name) {
                    let suggestions = suggestion_list(arg_name, known_args);
                    context.report_error(GraphQLError::new(format!(
                        "Unknown argument \"{arg_name}\" on directive \"@{directive_name}\".{}",
                        did_you_mean(None, &suggestions)
                    )));
                }
            }
        }
        VisitAction::Skip
    }
}
