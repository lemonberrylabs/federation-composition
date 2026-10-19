//! Port of `src/subgraph/validation/validate-state.ts`: graphql-js-like schema validation run on
//! the built `SubgraphState` (root types, directives, types, fields, interfaces, ...).

use std::sync::LazyLock;

use graphql_js::{GraphQLError, Value};
use indexmap::{IndexMap, IndexSet};

use super::validation_context::SubgraphValidationContext;
use crate::subgraph::state::{
    Argument, EnumType, InputField, InputObjectType, InterfaceType, ObjectType, SubgraphState, SubgraphType,
    TypeKind, UnionType,
};

/// `specifiedScalars`.
static SPECIFIED_SCALARS: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    graphql_js::specified_scalar_types()
        .iter()
        .map(|t| t.name.to_string())
        .collect()
});

/// `ReportErrorFn = (message: string) => void`. Every caller pushes into the same `errors`
/// list (`reportError` in `validateSubgraphState`), so it is a `&mut dyn FnMut`.
type ReportErrorFn<'r> = &'r mut dyn FnMut(String);

/// The result of `isOutputType`/`isInputType`: a boolean or the `SKIP` symbol (the type does
/// not exist; already reported by `KnownTypeNamesRule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeCheck {
    Yes,
    No,
    Skip,
}

/// `validateSubgraphState(state, context)`: errors with `extensions.code` `INVALID_GRAPHQL`.
/// The context is only read (`isLinkSpecDirective`, `isLinkSpecType`); the caller passes the
/// state through `&context.state_builder.state`.
pub fn validate_subgraph_state(state: &SubgraphState, context: &SubgraphValidationContext<'_>) -> Vec<GraphQLError> {
    todo!()
}

fn validate_root_types(state: &SubgraphState, report_error: ReportErrorFn<'_>) {
    todo!()
}

/// `capitalize(str)`: first char upper-cased, the rest lower-cased.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

fn validate_directives(state: &SubgraphState, report_error: ReportErrorFn<'_>, context: &SubgraphValidationContext<'_>) {
    todo!()
}

fn validate_types(state: &SubgraphState, report_error: ReportErrorFn<'_>) {
    todo!()
}

fn validate_name(report_error: ReportErrorFn<'_>, name: &str) {
    todo!()
}

/// `validateFields(state, reportError, type)` for an object or interface type.
fn validate_fields(state: &SubgraphState, report_error: ReportErrorFn<'_>, ty: &SubgraphType) {
    todo!()
}

fn is_validate_default_value(
    state: &SubgraphState,
    report_error: ReportErrorFn<'_>,
    input_type_printed: &str,
    value: &Value,
) -> bool {
    todo!()
}

fn validate_union_members(state: &SubgraphState, report_error: ReportErrorFn<'_>, union: &UnionType) {
    todo!()
}

fn validate_enum_values(report_error: ReportErrorFn<'_>, enum_type: &EnumType) {
    todo!()
}

fn validate_input_fields(state: &SubgraphState, report_error: ReportErrorFn<'_>, input_obj: &InputObjectType) {
    todo!()
}

/// `validateInterfaces(state, implementationsMap, reportError, type)` for an object or
/// interface type.
fn validate_interfaces(
    state: &SubgraphState,
    implementations_map: &mut IndexMap<String, IndexSet<String>>,
    report_error: ReportErrorFn<'_>,
    ty: &SubgraphType,
) {
    todo!()
}

fn validate_type_implements_ancestors(report_error: ReportErrorFn<'_>, ty: &SubgraphType, interface_type: &InterfaceType) {
    todo!()
}

fn validate_type_implements_interface(
    state: &SubgraphState,
    implementations_map: &mut IndexMap<String, IndexSet<String>>,
    report_error: ReportErrorFn<'_>,
    ty: &SubgraphType,
    interface_type: &InterfaceType,
) {
    todo!()
}

/// `isTypeSubTypeOf(state, implementationsMap, maybeSubTypeName, superTypeName)`.
pub fn is_type_sub_type_of(
    state: &SubgraphState,
    implementations_map: &IndexMap<String, IndexSet<String>>,
    maybe_sub_type_name: &str,
    super_type_name: &str,
) -> bool {
    todo!()
}

/// `isSubType(implementationsMap, abstractType, maybeSubType)`: `abstract_type` is an interface
/// or union, `maybe_sub_type` an object or interface.
fn is_sub_type(
    implementations_map: &IndexMap<String, IndexSet<String>>,
    abstract_type: &SubgraphType,
    maybe_sub_type: &SubgraphType,
) -> bool {
    todo!()
}

/// The closure returned by `createInputObjectCircularRefsValidator(state, reportError)`;
/// `detect_cycle_recursive` is that closure. `visitedTypes` is a set of object identities in
/// TS; input object names are unique in `state.types`, so names identify them.
struct InputObjectCircularRefsValidator<'s> {
    state: &'s SubgraphState,
    visited_types: IndexSet<String>,
    field_path: Vec<String>,
    /// `fieldPathIndexByTypeName` (`undefined` entries are removed).
    field_path_index_by_type_name: IndexMap<String, usize>,
}

fn create_input_object_circular_refs_validator(state: &SubgraphState) -> InputObjectCircularRefsValidator<'_> {
    InputObjectCircularRefsValidator {
        state,
        visited_types: IndexSet::new(),
        field_path: Vec::new(),
        field_path_index_by_type_name: IndexMap::new(),
    }
}

impl InputObjectCircularRefsValidator<'_> {
    fn detect_cycle_recursive(&mut self, input_obj: &InputObjectType, report_error: ReportErrorFn<'_>) {
        todo!()
    }
}

fn is_introspection_type(type_name: &str) -> bool {
    graphql_js::is_introspection_type(type_name)
}

fn is_abstract_type(ty: &SubgraphType) -> bool {
    is_interface_type(ty) || is_union_type(ty)
}

fn is_required_argument(arg: &Argument) -> bool {
    crate::utils::state::is_non_null(&arg.r#type) && arg.default_value.is_none()
}

fn is_required_input_field(arg: &InputField) -> bool {
    crate::utils::state::is_non_null(&arg.r#type) && arg.default_value.is_none()
}

fn is_output_type(state: &SubgraphState, type_name: &str) -> TypeCheck {
    todo!()
}

/// `isInputType(state, typeName)`.
pub fn is_input_type(state: &SubgraphState, type_name: &str) -> TypeCheck {
    todo!()
}

pub fn type_exists(state: &SubgraphState, type_name: &str) -> bool {
    state.types.contains_key(type_name) || SPECIFIED_SCALARS.contains(type_name)
}

pub fn is_input_object_type(ty: &SubgraphType) -> bool {
    ty.kind() == TypeKind::InputObject
}

pub fn is_scalar_type(ty: &SubgraphType) -> bool {
    ty.kind() == TypeKind::Scalar
}

pub fn is_enum_type(ty: &SubgraphType) -> bool {
    ty.kind() == TypeKind::Enum
}

pub fn is_object_type(ty: &SubgraphType) -> bool {
    ty.kind() == TypeKind::Object
}

pub fn is_interface_type(ty: &SubgraphType) -> bool {
    ty.kind() == TypeKind::Interface
}

pub fn is_union_type(ty: &SubgraphType) -> bool {
    ty.kind() == TypeKind::Union
}

pub fn is_directive(ty: &SubgraphType) -> bool {
    ty.kind() == TypeKind::Directive
}
