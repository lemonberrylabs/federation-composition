//! Port of `src/subgraph/validation/rules/elements/from-context.ts`.

use graphql_js::{GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use graphql_js::{ConstDirective, FieldDefinition, InputValueDefinition, SelectionSet, Type};
use indexmap::{IndexMap, IndexSet};
use crate::utils::js::JsResult;

/// `FromContextDirectiveRules(context)`. Fields: `context_owners_by_name` (`indexContextOwners(context)`).
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct FromContextDirectiveRules {}

impl<'a> SubgraphRule<'a> for FromContextDirectiveRules {
    const NAME: &'static str = "FromContextDirectiveRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        todo!()
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for FromContextDirectiveRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}

/// `ContextOwnerName`.
type ContextOwnerName = String;

/// `typenameType`: `String!`.
fn typename_type() -> Type {
    Type::non_null(Type::named("String"))
}

/// `SelectableType`.
#[derive(Clone, Debug, PartialEq)]
enum SelectableType {
    /// `kind: "OBJECT" | "INTERFACE"`.
    ObjectOrInterface {
        is_interface: bool,
        name: String,
        fields: IndexMap<String, FieldDefinition>,
        interfaces: Vec<String>,
        directives: Vec<ConstDirective>,
    },
    /// `kind: "UNION"`.
    Union {
        name: String,
        members: Vec<String>,
        directives: Vec<ConstDirective>,
    },
}

/// `ContextValue`: a field path or a type-conditioned alternative, without GraphQL syntax.
#[derive(Clone, Debug, PartialEq)]
enum ContextValue {
    /// `{ field, next? }`.
    Field { field: String, next: Option<Vec<ContextValue>> },
    /// `{ onType, next }`.
    OnType { on_type: Option<String>, next: Vec<ContextValue> },
}

/// `ContextSelection`.
#[derive(Clone, Debug, PartialEq)]
enum ContextSelection {
    Direct { values: Vec<ContextValue> },
    Conditional { branches: IndexMap<String, Vec<ContextValue>> },
    Invalid { reason: String },
}

/// `SelectionTypeResult`.
#[derive(Clone, Debug, PartialEq)]
enum SelectionTypeResult {
    Resolved { ty: Type },
    Unresolved,
    UnknownField { owner: String, field: String },
    InterfaceObject { type_name: String },
}

/// The result of `compileContextValues`.
#[derive(Clone, Debug, PartialEq)]
enum CompiledContextValues {
    Values { values: Vec<ContextValue> },
    Invalid { reason: String },
}

fn index_context_owners(context: &SubgraphValidationContext<'_>) -> IndexMap<String, Vec<ContextOwnerName>> {
    todo!()
}

fn read_context_directive_name(context: &SubgraphValidationContext<'_>, directive: &ConstDirective) -> Option<String> {
    todo!()
}

fn remember_context_owner(
    owners_by_context: &mut IndexMap<String, Vec<ContextOwnerName>>,
    context_name: Option<&str>,
    owner_name: &str,
) {
    todo!()
}

fn arg_coordinate(context: &SubgraphValidationContext<'_>) -> String {
    todo!()
}

/// The `input` of `validateSelection`.
struct ValidateSelectionInput<'i> {
    context_owner_names: &'i [ContextOwnerName],
    context_name: &'i str,
    selection: &'i str,
    arg_def: &'i InputValueDefinition,
}

/// `validateSelection(input)`. Its nested functions (`invalid`, `getSelectableType`,
/// `runtimeSetFor`, `getFieldDefinition`, `resolveSelectedType`, `resolveSelectionType`,
/// `validateSelectedType`, `validateConditionalSelections`) and their shared locals
/// (`typeViews`, `runtimeTypeSets`) are the [`SelectionValidator`] methods and fields.
fn validate_selection(context: &mut SubgraphValidationContext<'_>, input: ValidateSelectionInput<'_>) -> JsResult<()> {
    todo!()
}

/// The locals shared by the nested functions of `validateSelection`.
struct SelectionValidator<'i> {
    context_name: &'i str,
    arg_def: &'i InputValueDefinition,
    type_views: IndexMap<String, Option<SelectableType>>,
    runtime_type_sets: IndexMap<String, IndexSet<String>>,
}

impl SelectionValidator<'_> {
    fn invalid(&self, context: &mut SubgraphValidationContext<'_>, reason: &str) {
        todo!()
    }

    fn get_selectable_type(&mut self, context: &SubgraphValidationContext<'_>, type_name: &str) -> Option<SelectableType> {
        todo!()
    }

    /// The object types the given type can be at runtime.
    fn runtime_set_for(&mut self, context: &SubgraphValidationContext<'_>, ty: &SelectableType) -> IndexSet<String> {
        todo!()
    }

    /// `type` is an object or interface `SelectableType`.
    fn get_field_definition(
        &mut self,
        context: &SubgraphValidationContext<'_>,
        ty: &SelectableType,
        field_name: &str,
    ) -> Option<FieldDefinition> {
        todo!()
    }

    fn resolve_selected_type(
        &mut self,
        context: &mut SubgraphValidationContext<'_>,
        current_type: &SelectableType,
        selections: &[ContextValue],
    ) -> JsResult<SelectionTypeResult> {
        todo!()
    }

    fn resolve_selection_type(
        &mut self,
        context: &mut SubgraphValidationContext<'_>,
        current_type: &SelectableType,
        selection: &ContextValue,
    ) -> JsResult<SelectionTypeResult> {
        todo!()
    }

    fn validate_selected_type(&mut self, context: &mut SubgraphValidationContext<'_>, result: &SelectionTypeResult) {
        todo!()
    }

    fn validate_conditional_selections(
        &mut self,
        context: &mut SubgraphValidationContext<'_>,
        owner: &SelectableType,
        branches: &IndexMap<String, Vec<ContextValue>>,
        touched_type_conditions: &mut IndexSet<String>,
    ) -> JsResult<()> {
        todo!()
    }
}

/// `current_type` is an object or interface `SelectableType`.
fn mark_field_as_used(context: &mut SubgraphValidationContext<'_>, current_type: &SelectableType, field_name: &str) -> JsResult<()> {
    todo!()
}

fn report_invalid_selection(context: &mut SubgraphValidationContext<'_>, context_name: &str, reason: &str) {
    todo!()
}

fn report_unknown_field(context: &mut SubgraphValidationContext<'_>, context_name: &str, owner: &str, field: &str) {
    todo!()
}

fn report_interface_object(context: &mut SubgraphValidationContext<'_>, context_name: &str, type_name: &str) {
    todo!()
}

/// `parseContextSelection(source)`. `parseFields` can throw a `GraphQLError`.
fn parse_context_selection(source: &str) -> JsResult<ContextSelection> {
    todo!()
}

fn build_selectable_type(context: &SubgraphValidationContext<'_>, type_name: &str) -> Option<SelectableType> {
    todo!()
}

fn has_interface_object_directive(context: &SubgraphValidationContext<'_>, ty: &SelectableType) -> bool {
    todo!()
}

fn nullable_type(ty: &Type) -> &Type {
    match ty {
        Type::NonNull(n) => &n.ty,
        other => other,
    }
}

fn wrap_list_modifiers(ty: &Type, resolved_type: &Type) -> Type {
    todo!()
}

fn matches_arg_type(select_type: &Type, arg_type: &Type) -> bool {
    todo!()
}

fn compile_context_values(selection_set: &SelectionSet) -> CompiledContextValues {
    todo!()
}
