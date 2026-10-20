//! Port of graphql-js `validation/specifiedRules`: `specifiedSDLRules`.
//!
//! NOTE(port): `specifiedRules` (the rules for executable documents) is not
//! ported.

use crate::visitor::Visitor;

use super::rules::known_argument_names_rule::KnownArgumentNamesOnDirectivesRule;
use super::rules::known_directives_rule::KnownDirectivesRule;
use super::rules::known_type_names_rule::KnownTypeNamesRule;
use super::rules::lone_schema_definition_rule::LoneSchemaDefinitionRule;
use super::rules::possible_type_extensions_rule::PossibleTypeExtensionsRule;
use super::rules::provided_required_arguments_rule::ProvidedRequiredArgumentsOnDirectivesRule;
use super::rules::unique_argument_definition_names_rule::UniqueArgumentDefinitionNamesRule;
use super::rules::unique_argument_names_rule::UniqueArgumentNamesRule;
use super::rules::unique_directive_names_rule::UniqueDirectiveNamesRule;
use super::rules::unique_directives_per_location_rule::UniqueDirectivesPerLocationRule;
use super::rules::unique_enum_value_names_rule::UniqueEnumValueNamesRule;
use super::rules::unique_field_definition_names_rule::UniqueFieldDefinitionNamesRule;
use super::rules::unique_input_field_names_rule::UniqueInputFieldNamesRule;
use super::rules::unique_operation_types_rule::UniqueOperationTypesRule;
use super::rules::unique_type_names_rule::UniqueTypeNamesRule;
use super::validation_context::SDLValidationContext;

/// The visitors of graphql-js `specifiedSDLRules.map(rule => rule(context))`,
/// in order.
pub fn specified_sdl_rules<'a>(
    context: &mut SDLValidationContext<'a>,
) -> Vec<Box<dyn Visitor<'a, SDLValidationContext<'a>> + 'a>> {
    vec![
        Box::new(LoneSchemaDefinitionRule::new(context)),
        Box::new(UniqueOperationTypesRule::new(context)),
        Box::new(UniqueTypeNamesRule::new(context)),
        Box::new(UniqueEnumValueNamesRule::new(context)),
        Box::new(UniqueFieldDefinitionNamesRule::new(context)),
        Box::new(UniqueArgumentDefinitionNamesRule::new(context)),
        Box::new(UniqueDirectiveNamesRule::new(context)),
        Box::new(KnownTypeNamesRule::new(context)),
        Box::new(KnownDirectivesRule::new(context)),
        Box::new(UniqueDirectivesPerLocationRule::new(context)),
        Box::new(PossibleTypeExtensionsRule::new(context)),
        Box::new(KnownArgumentNamesOnDirectivesRule::new(context)),
        Box::new(UniqueArgumentNamesRule::new(context)),
        Box::new(UniqueInputFieldNamesRule::new(context)),
        Box::new(ProvidedRequiredArgumentsOnDirectivesRule::new(context)),
    ]
}
