//! Port of `src/supergraph/validation/rules/interface-subtype-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use crate::utils::state::{is_list, is_non_null, strip_list, strip_non_null};
use indexmap::{IndexMap, IndexSet};
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::scalar_type::ScalarTypeState;
use crate::supergraph::composition::union_type::UnionTypeState;

/// `InterfaceSubtypeRule(context, supergraph)`. Fields: `implementations_map` (built in `new`).
/// Visitor callbacks: `ObjectTypeField`.
pub struct InterfaceSubtypeRule<'s> {
    supergraph: &'s SupergraphState,
    implementations_map: IndexMap<String, IndexSet<String>>,
}

/// Store implementations of `iface` by `type_name` if `iface` is an interface.
fn store_implementation(
    supergraph: &SupergraphState,
    implementations_map: &mut IndexMap<String, IndexSet<String>>,
    iface: &str,
    type_name: &str,
) {
    let interface_type = get_type_from_supergraph(supergraph, iface);
    if interface_type.is_some_and(is_interface_type) {
        implementations_map
            .entry(iface.to_string())
            .or_default()
            .insert(type_name.to_string());
    }
}

impl<'s> SupergraphRule<'s> for InterfaceSubtypeRule<'s> {
    const NAME: &'static str = "InterfaceSubtypeRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        let mut implementations_map: IndexMap<String, IndexSet<String>> = IndexMap::new();

        for ty in supergraph.interface_types.values() {
            // Store implementations by interface.
            for iface in &ty.interfaces {
                store_implementation(supergraph, &mut implementations_map, iface, &ty.name);
            }
        }

        for ty in supergraph.object_types.values() {
            // Store implementations by objects.
            for iface in &ty.interfaces {
                store_implementation(supergraph, &mut implementations_map, iface, &ty.name);
            }
        }

        Ok(InterfaceSubtypeRule {
            supergraph,
            implementations_map,
        })
    }
}

impl<'s> SupergraphVisitor<'s> for InterfaceSubtypeRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        if object_type_state.interfaces.is_empty() {
            return Ok(());
        }

        for interface_name in &object_type_state.interfaces {
            let Some(interface_state) = self.supergraph.interface_types.get(interface_name) else {
                continue;
            };

            let Some(interface_field) = interface_state.fields.get(&field_state.name) else {
                continue;
            };

            if !is_type_sub_type_of(
                self.supergraph,
                &self.implementations_map,
                &field_state.r#type,
                &interface_field.r#type,
            ) {
                cx.report_error(GraphQLError::with_code(
                    format!(
                        "Interface field {interface_name}.{} expects type {} but {}.{} of type {} is not a proper subtype.",
                        interface_field.name,
                        interface_field.r#type,
                        object_type_state.name,
                        field_state.name,
                        field_state.r#type
                    ),
                    "INVALID_GRAPHQL",
                ));
            }
        }

        Ok(())
    }
}

/// Provided a type and a super type, return true if the first type is either
/// equal or a subset of the second super type (covariant).
fn is_type_sub_type_of(
    state: &SupergraphState,
    implementations_map: &IndexMap<String, IndexSet<String>>,
    maybe_sub_type_name: &str,
    super_type_name: &str,
) -> bool {
    // Equivalent type is a valid subtype
    if maybe_sub_type_name == super_type_name {
        return true;
    }

    // If superType is non-null, maybeSubType must also be non-null.
    if is_non_null(super_type_name) {
        if is_non_null(maybe_sub_type_name) {
            return is_type_sub_type_of(
                state,
                implementations_map,
                &strip_non_null(maybe_sub_type_name),
                &strip_non_null(super_type_name),
            );
        }
        return false;
    }
    if is_non_null(maybe_sub_type_name) {
        // If superType is nullable, maybeSubType may be non-null or nullable.
        return is_type_sub_type_of(state, implementations_map, &strip_non_null(maybe_sub_type_name), super_type_name);
    }

    // If superType type is a list, maybeSubType type must also be a list.
    if is_list(super_type_name) {
        if is_list(maybe_sub_type_name) {
            return is_type_sub_type_of(
                state,
                implementations_map,
                &strip_list(maybe_sub_type_name),
                &strip_list(super_type_name),
            );
        }
        return false;
    }
    if is_list(maybe_sub_type_name) {
        // If superType is not a list, maybeSubType must also be not a list.
        return false;
    }

    let super_type = get_type_from_supergraph(state, super_type_name);
    let maybe_sub_type = get_type_from_supergraph(state, maybe_sub_type_name);

    // The existence of the types was already validated.
    // If one of them does not exist, it means it's been reported by KnownTypeNamesRule.
    let (Some(super_type), Some(maybe_sub_type)) = (super_type, maybe_sub_type) else {
        return false;
    };

    // If superType type is an abstract type, check if it is super type of maybeSubType.
    // Otherwise, the child type is not a valid subtype of the parent type.
    is_abstract_type(super_type)
        && (is_interface_type(maybe_sub_type) || is_object_type(maybe_sub_type))
        && is_sub_type(implementations_map, super_type, maybe_sub_type)
}

/// `objectTypes ?? interfaceTypes ?? unionTypes`.
fn get_type_from_supergraph<'s>(state: &'s SupergraphState, name: &str) -> Option<SupergraphType<'s>> {
    if let Some(t) = state.object_types.get(name) {
        return Some(SupergraphType::Object(t));
    }
    if let Some(t) = state.interface_types.get(name) {
        return Some(SupergraphType::Interface(t));
    }
    if let Some(t) = state.union_types.get(name) {
        return Some(SupergraphType::Union(t));
    }
    None
}

/// `abstract_type` is an interface or union, `maybe_sub_type` an object or interface.
fn is_sub_type(
    implementations_map: &IndexMap<String, IndexSet<String>>,
    abstract_type: SupergraphType<'_>,
    maybe_sub_type: SupergraphType<'_>,
) -> bool {
    let maybe_sub_type_name = type_name(maybe_sub_type);
    if let SupergraphType::Union(union_type) = abstract_type {
        return union_type.members.contains(maybe_sub_type_name);
    }

    implementations_map
        .get(type_name(abstract_type))
        .is_some_and(|implementations| implementations.contains(maybe_sub_type_name))
}

/// `type.name`.
fn type_name(ty: SupergraphType<'_>) -> &str {
    match ty {
        SupergraphType::Object(t) => &t.name,
        SupergraphType::Enum(t) => &t.name,
        SupergraphType::Scalar(t) => &t.name,
        SupergraphType::Union(t) => &t.name,
        SupergraphType::Interface(t) => &t.name,
    }
}

/// `SupergraphType`.
#[derive(Clone, Copy, Debug)]
// TS: the `SupergraphType` union lists enum and scalar states, which `getTypeFromSupergraph` never returns.
#[allow(dead_code)]
enum SupergraphType<'s> {
    Object(&'s ObjectTypeState),
    Enum(&'s EnumTypeState),
    Scalar(&'s ScalarTypeState),
    Union(&'s UnionTypeState),
    Interface(&'s InterfaceTypeState),
}

fn is_abstract_type(ty: SupergraphType<'_>) -> bool {
    is_interface_type(ty) || is_union_type(ty)
}

fn is_object_type(ty: SupergraphType<'_>) -> bool {
    matches!(ty, SupergraphType::Object(_))
}

fn is_interface_type(ty: SupergraphType<'_>) -> bool {
    matches!(ty, SupergraphType::Interface(_))
}

fn is_union_type(ty: SupergraphType<'_>) -> bool {
    matches!(ty, SupergraphType::Union(_))
}
