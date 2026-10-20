//! Port of `src/supergraph/validation/validate-supergraph.ts`.

use graphql_core::GraphQLError;

use super::rules::auth_on_context_rule::AuthOnContextRule;
use super::rules::auth_on_requires_rule::AuthOnRequiresRule;
use super::rules::contextual_argument_rule::ContextualArgumentRule;
use super::rules::default_value_uses_inaccessible_rule::DefaultValueUsesInaccessibleRule;
use super::rules::directive_composition_rule::DirectiveCompositionRule;
use super::rules::enum_values_rule::EnumValuesRule;
use super::rules::extension_with_base::ExtensionWithBaseRule;
use super::rules::external_argument_missing_rule::ExternalArgumentMissingRule;
use super::rules::external_missing_on_base_rule::ExternalMissingOnBaseRule;
use super::rules::external_type_mismatch_rule::ExternalTypeMismatchRule;
use super::rules::field_argument_default_mismatch_rule::FieldArgumentDefaultMismatchRule;
use super::rules::field_arguments_of_the_same_type_rule::FieldArgumentsOfTheSameTypeRule;
use super::rules::fields_of_the_same_type_rule::FieldsOfTheSameTypeRule;
use super::rules::input_field_default_mismatch_rule::InputFieldDefaultMismatchRule;
use super::rules::input_object_values_rule::InputObjectValuesRule;
use super::rules::interface_field_no_implementation_rule::InterfaceFieldNoImplementationRule;
use super::rules::interface_key_missing_implementation_type::InterfaceKeyMissingImplementationTypeRule;
use super::rules::interface_object_usage_error::InterfaceObjectUsageErrorRule;
use super::rules::interface_subtype_rule::InterfaceSubtypeRule;
use super::rules::invalid_field_sharing_rule::InvalidFieldSharingRule;
use super::rules::link_import_name_mismatch_rule::link_import_name_mismatch_rule;
use super::rules::list_size_slicing_arguments_rule::ListSizeSlicingArgumentsRule;
use super::rules::no_inaccessible_on_implemented_interface_fields_rule::NoInaccessibleOnImplementedInterfaceFieldsRule;
use super::rules::only_inaccessible_children_rule::OnlyInaccessibleChildrenRule;
use super::rules::override_label_with_requires::OverrideLabelWithRequiresRule;
use super::rules::override_source_has_override::OverrideSourceHasOverrideRule;
use super::rules::referenced_inaccessible_rule::ReferencedInaccessibleRule;
use super::rules::required_argument_missing_in_some_subgraph_rule::RequiredArgumentMissingInSomeSubgraph;
use super::rules::required_argument_or_field_is_not_inaccessible_rule::RequiredArgumentOrFieldIsNotInaccessibleRule;
use super::rules::required_input_field_missing_in_some_subgraph_rule::RequiredInputFieldMissingInSomeSubgraphRule;
use super::rules::required_query_rule::required_query_rule;
use super::rules::satisfiablity_rule::SatisfiabilityRule;
use super::rules::subgraph_name_rule::SubgraphNameRule;
use super::rules::types_of_the_same_kind_rule::types_of_the_same_kind_rule;
use super::validation_context::{create_supergraph_validation_context, SupergraphRule, SupergraphValidationContext};
use crate::subgraph::validation::validate_subgraph::InternalOptions;
use crate::supergraph::composition::visitor::{visit_supergraph_state, SupergraphVisitor};
use crate::supergraph::state::{SubgraphStates, SupergraphState, SupergraphStateBuilder};
use crate::utils::js::JsResult;

/// A pre-supergraph rule: `(context) => void`.
pub type PreSupergraphRule = fn(&mut SupergraphValidationContext);

/// `preSupergraphRules`, with the TS function names used by `disableValidationRules`.
pub const PRE_SUPERGRAPH_RULES: [(&str, PreSupergraphRule); 3] = [
    ("RequiredQueryRule", required_query_rule),
    ("TypesOfTheSameKindRule", types_of_the_same_kind_rule),
    ("LinkImportNameMismatchRule", link_import_name_mismatch_rule),
];

/// `validateSupergraph(subgraphStates, state, __internal?)`.
/// The post-supergraph rules (`postSupergraphRules`, then `selectionDependentRules` if no
/// error was reported) are `SupergraphRule` structs, built in list order with
/// [`supergraph_rule`] and visited with `visit_supergraph_state`.
pub fn validate_supergraph(
    subgraph_states: SubgraphStates,
    state: &mut SupergraphStateBuilder,
    internal: Option<&InternalOptions>,
) -> JsResult<Vec<GraphQLError>> {
    let mut context = create_supergraph_validation_context(subgraph_states.clone());

    for subgraph_state in subgraph_states.values() {
        state.add_subgraph(subgraph_state.clone())?;
    }

    let rules_to_skip: Vec<String> = internal
        .and_then(|i| i.disable_validation_rules.clone())
        .unwrap_or_default();

    for (name, rule) in PRE_SUPERGRAPH_RULES {
        if rules_to_skip.iter().any(|r| r == name) {
            continue;
        }
        rule(&mut context);
    }

    for subgraph_state in subgraph_states.values() {
        state.visit_subgraph_state(subgraph_state.clone());
    }

    state.compose_supergraph_state()?;

    // `runRules(postSupergraphRules)`.
    {
        let supergraph = state.get_supergraph_state();
        let skip = rules_to_skip.as_slice();
        let cx = &mut context;
        let mut visitors: Vec<Box<dyn SupergraphVisitor<'_> + '_>> = vec![
            supergraph_rule::<InterfaceFieldNoImplementationRule>(skip, cx, supergraph)?,
            supergraph_rule::<ExtensionWithBaseRule>(skip, cx, supergraph)?,
            supergraph_rule::<FieldsOfTheSameTypeRule>(skip, cx, supergraph)?,
            supergraph_rule::<FieldArgumentsOfTheSameTypeRule>(skip, cx, supergraph)?,
            supergraph_rule::<EnumValuesRule>(skip, cx, supergraph)?,
            supergraph_rule::<OverrideSourceHasOverrideRule>(skip, cx, supergraph)?,
            supergraph_rule::<OverrideLabelWithRequiresRule>(skip, cx, supergraph)?,
            supergraph_rule::<ExternalMissingOnBaseRule>(skip, cx, supergraph)?,
            supergraph_rule::<InputObjectValuesRule>(skip, cx, supergraph)?,
            supergraph_rule::<RequiredArgumentMissingInSomeSubgraph>(skip, cx, supergraph)?,
            supergraph_rule::<RequiredInputFieldMissingInSomeSubgraphRule>(skip, cx, supergraph)?,
            supergraph_rule::<ExternalArgumentMissingRule>(skip, cx, supergraph)?,
            supergraph_rule::<InputFieldDefaultMismatchRule>(skip, cx, supergraph)?,
            supergraph_rule::<FieldArgumentDefaultMismatchRule>(skip, cx, supergraph)?,
            supergraph_rule::<DefaultValueUsesInaccessibleRule>(skip, cx, supergraph)?,
            supergraph_rule::<OnlyInaccessibleChildrenRule>(skip, cx, supergraph)?,
            supergraph_rule::<ReferencedInaccessibleRule>(skip, cx, supergraph)?,
            supergraph_rule::<DirectiveCompositionRule>(skip, cx, supergraph)?,
            supergraph_rule::<InterfaceObjectUsageErrorRule>(skip, cx, supergraph)?,
            supergraph_rule::<InterfaceKeyMissingImplementationTypeRule>(skip, cx, supergraph)?,
            supergraph_rule::<ExternalTypeMismatchRule>(skip, cx, supergraph)?,
            supergraph_rule::<InvalidFieldSharingRule>(skip, cx, supergraph)?,
            supergraph_rule::<SubgraphNameRule>(skip, cx, supergraph)?,
            supergraph_rule::<RequiredArgumentOrFieldIsNotInaccessibleRule>(skip, cx, supergraph)?,
            supergraph_rule::<InterfaceSubtypeRule>(skip, cx, supergraph)?,
            supergraph_rule::<NoInaccessibleOnImplementedInterfaceFieldsRule>(skip, cx, supergraph)?,
            supergraph_rule::<ListSizeSlicingArgumentsRule>(skip, cx, supergraph)?,
            supergraph_rule::<ContextualArgumentRule>(skip, cx, supergraph)?,
        ];
        visit_supergraph_state(supergraph, cx, &mut visitors)?;
    }
    // NOTE(port): the TS rules mutate the state in place during the visit; apply the writes
    // they recorded now (see `SupergraphValidationContext::pending_state_updates`).
    apply_state_updates(&mut context, state);

    let errors = context.collect_reported_errors();

    if !errors.is_empty() {
        return Ok(errors);
    }

    // These resolve @key/@requires/@provides/@fromContext selection sets against the merged
    // supergraph state. A selection set is written against a single subgraph, so when subgraphs
    // disagree on a field's type (FIELD_TYPE_MISMATCH, EXTERNAL_TYPE_MISMATCH, TYPE_KIND_MISMATCH...)
    // it may point at fields the merged type does not have, and these rules throw instead of
    // reporting. Composition already failed at that point, so run them only on a coherent state.
    // `runRules(selectionDependentRules)`.
    {
        let supergraph = state.get_supergraph_state();
        let skip = rules_to_skip.as_slice();
        let cx = &mut context;
        let mut visitors: Vec<Box<dyn SupergraphVisitor<'_> + '_>> = vec![
            supergraph_rule::<SatisfiabilityRule>(skip, cx, supergraph)?,
            supergraph_rule::<AuthOnRequiresRule>(skip, cx, supergraph)?,
            supergraph_rule::<AuthOnContextRule>(skip, cx, supergraph)?,
        ];
        visit_supergraph_state(supergraph, cx, &mut visitors)?;
    }
    apply_state_updates(&mut context, state);

    Ok(context.collect_reported_errors())
}

/// Applies the supergraph state writes the rules recorded during a visit, in order.
fn apply_state_updates(context: &mut SupergraphValidationContext, state: &mut SupergraphStateBuilder) {
    let updates = context.take_state_updates();
    if updates.is_empty() {
        return;
    }
    let supergraph = state.get_supergraph_state_mut();
    for update in updates {
        update.apply(supergraph);
    }
}

/// `rulesToSkip.includes(rule.name) ? {} : rule(context, supergraph)`.
fn supergraph_rule<'s, R: SupergraphRule<'s>>(
    rules_to_skip: &[String],
    context: &mut SupergraphValidationContext,
    supergraph: &'s SupergraphState,
) -> JsResult<Box<dyn SupergraphVisitor<'s> + 's>> {
    if rules_to_skip.iter().any(|r| r == R::NAME) {
        return Ok(Box::new(EmptySupergraphVisitor));
    }

    Ok(Box::new(R::new(context, supergraph)?))
}

/// An empty visitor (`{}`), used for skipped rules.
struct EmptySupergraphVisitor;

impl SupergraphVisitor<'_> for EmptySupergraphVisitor {}
