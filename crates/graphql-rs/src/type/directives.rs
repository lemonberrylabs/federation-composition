//! Port of graphql-js `type/directives`: `GraphQLDirective` and the specified
//! directives.

use std::fmt;

use crate::ast::{DirectiveDefinition, DirectiveExtension};
use crate::collections::IndexMap;
use crate::directive_location::DirectiveLocation;
use crate::error::GraphQLError;
use crate::jsutils::JsValue;
use crate::specified::DEFAULT_DEPRECATION_REASON;

use super::assert_name::assert_name;
use super::definition::{ArgumentParent, DefaultInput, GraphQLArgument, GraphQLArgumentConfig, GraphQLType};

/// graphql-js `GraphQLDirectiveConfig`.
#[derive(Clone, Debug, Default)]
pub struct GraphQLDirectiveConfig {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<DirectiveLocation>,
    pub args: IndexMap<String, GraphQLArgumentConfig>,
    pub is_repeatable: bool,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<DirectiveDefinition>,
    pub extension_ast_nodes: Vec<DirectiveExtension>,
}

/// graphql-js `GraphQLDirective`.
#[derive(Clone, Debug)]
pub struct GraphQLDirective {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<DirectiveLocation>,
    pub is_repeatable: bool,
    pub deprecation_reason: Option<String>,
    pub ast_node: Option<DirectiveDefinition>,
    pub extension_ast_nodes: Vec<DirectiveExtension>,
    pub args: Vec<GraphQLArgument>,
}

impl GraphQLDirective {
    pub fn new(config: GraphQLDirectiveConfig) -> Result<Self, GraphQLError> {
        assert_name(&config.name)?;
        let mut args = Vec::with_capacity(config.args.len());
        for (arg_name, arg_config) in config.args {
            args.push(GraphQLArgument::new(
                ArgumentParent::Directive(config.name.clone()),
                &arg_name,
                arg_config,
            )?);
        }
        Ok(GraphQLDirective {
            name: config.name,
            description: config.description,
            locations: config.locations,
            is_repeatable: config.is_repeatable,
            deprecation_reason: config.deprecation_reason,
            ast_node: config.ast_node,
            extension_ast_nodes: config.extension_ast_nodes,
            args,
        })
    }
}

impl fmt::Display for GraphQLDirective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.name)
    }
}

fn arg(ty: GraphQLType, description: &str, default: Option<JsValue>) -> GraphQLArgumentConfig {
    GraphQLArgumentConfig {
        description: Some(description.to_string()),
        ty,
        default: default.map(DefaultInput::Value),
        deprecation_reason: None,
        ast_node: None,
    }
}

fn specified(
    name: &str,
    description: &str,
    locations: Vec<DirectiveLocation>,
    args: Vec<(&str, GraphQLArgumentConfig)>,
) -> GraphQLDirective {
    GraphQLDirective::new(GraphQLDirectiveConfig {
        name: name.to_string(),
        description: Some(description.to_string()),
        locations,
        args: args
            .into_iter()
            .map(|(name, config)| (name.to_string(), config))
            .collect(),
        ..Default::default()
    })
    .expect("specified directive names are valid")
}

fn non_null(name: &str) -> GraphQLType {
    GraphQLType::non_null(GraphQLType::named(name))
}

/// graphql-js `GraphQLIncludeDirective`.
pub fn graphql_include_directive() -> GraphQLDirective {
    specified(
        "include",
        "Directs the executor to include this field or fragment only when the `if` argument is true.",
        vec![
            DirectiveLocation::Field,
            DirectiveLocation::FragmentSpread,
            DirectiveLocation::InlineFragment,
        ],
        vec![("if", arg(non_null("Boolean"), "Included when true.", None))],
    )
}

/// graphql-js `GraphQLSkipDirective`.
pub fn graphql_skip_directive() -> GraphQLDirective {
    specified(
        "skip",
        "Directs the executor to skip this field or fragment when the `if` argument is true.",
        vec![
            DirectiveLocation::Field,
            DirectiveLocation::FragmentSpread,
            DirectiveLocation::InlineFragment,
        ],
        vec![("if", arg(non_null("Boolean"), "Skipped when true.", None))],
    )
}

/// graphql-js `GraphQLDeprecatedDirective`.
pub fn graphql_deprecated_directive() -> GraphQLDirective {
    specified(
        "deprecated",
        "Marks an element of a GraphQL schema as no longer supported.",
        vec![
            DirectiveLocation::FieldDefinition,
            DirectiveLocation::ArgumentDefinition,
            DirectiveLocation::InputFieldDefinition,
            DirectiveLocation::EnumValue,
            DirectiveLocation::DirectiveDefinition,
        ],
        vec![(
            "reason",
            arg(
                non_null("String"),
                "Explains why this element was deprecated, usually also including a suggestion for how to access supported similar data. Formatted using the Markdown syntax, as specified by [CommonMark](https://commonmark.org/).",
                Some(JsValue::String(DEFAULT_DEPRECATION_REASON.to_string())),
            ),
        )],
    )
}

/// graphql-js `GraphQLSpecifiedByDirective`.
pub fn graphql_specified_by_directive() -> GraphQLDirective {
    specified(
        "specifiedBy",
        "Exposes a URL that specifies the behavior of this scalar.",
        vec![DirectiveLocation::Scalar],
        vec![(
            "url",
            arg(
                non_null("String"),
                "The URL that specifies the behavior of this scalar.",
                None,
            ),
        )],
    )
}

/// graphql-js `GraphQLOneOfDirective`.
pub fn graphql_one_of_directive() -> GraphQLDirective {
    specified(
        "oneOf",
        "Indicates exactly one field must be supplied and this field must not be `null`.",
        vec![DirectiveLocation::InputObject],
        vec![],
    )
}

/// graphql-js `specifiedDirectives`.
pub fn specified_directives() -> Vec<GraphQLDirective> {
    vec![
        graphql_include_directive(),
        graphql_skip_directive(),
        graphql_deprecated_directive(),
        graphql_specified_by_directive(),
        graphql_one_of_directive(),
    ]
}

/// graphql-js `isSpecifiedDirective` (by name, as graphql-js compares).
pub fn is_specified_directive(directive: &GraphQLDirective) -> bool {
    crate::specified::is_specified_directive(&directive.name)
}
