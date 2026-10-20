//! Port of `src/supergraph/validation/rules/auth-on-requires-rule.ts`.

use crate::subgraph::helpers::parse_fields;
use crate::supergraph::composition::enum_type::EnumTypeState;
use crate::supergraph::composition::interface_type::InterfaceTypeState;
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::scalar_type::ScalarTypeState;
use crate::supergraph::composition::union_type::UnionTypeState;
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::auth::merge_scope_policies;
use crate::utils::graphql::extract_named_type_name;
use crate::utils::js::{JsError, JsResult};
use graphql_core::{is_specified_scalar_type, GraphQLError, Selection};
use graphql_core::{Field, InlineFragment, SelectionSet};

/// `AuthOnRequiresRule(context, supergraph)`. Validates that fields annotated with `@requires`
/// have appropriate auth directives (`@authenticated`, `@requiresScopes`, `@policy`) to ensure
/// transitive fields accessed via the `@requires` selection set are properly protected.
/// Visitor callbacks: `ObjectTypeField`.
pub struct AuthOnRequiresRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for AuthOnRequiresRule<'s> {
    const NAME: &'static str = "AuthOnRequiresRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(AuthOnRequiresRule { supergraph })
    }
}

impl<'s> SupergraphVisitor<'s> for AuthOnRequiresRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_type_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        // Check each graph where this field is defined with @requires
        for (graph_id, field_in_graph) in &field_state.by_graph {
            let Some(requires) = field_in_graph.requires.as_deref().filter(|r| !r.is_empty()) else {
                continue;
            };

            // Parse the @requires selection set string into a SelectionSet AST
            let Some(selection_set) = parse_fields(requires)? else {
                continue;
            };

            let mut provisioned_access = ProvisionedAccess::new(object_type_state, field_state);

            // Recursively traverse the selection set and ensure access requirements are met
            let field_lacking_access = ensure_access_to_selection_set(
                self.supergraph,
                RequiresCurrentType::Object(object_type_state),
                &selection_set,
                &mut provisioned_access,
            )?;

            if let Some(field_lacking_access) = field_lacking_access {
                let graph_name = cx.graph_id_to_name(graph_id)?;
                cx.report_error(create_access_requirement_error(
                    &graph_name,
                    &format!("{}.{}", object_type_state.name, field_state.name),
                    &field_lacking_access,
                ));
                return Ok(());
            }
        }

        Ok(())
    }
}

/// `FieldCoordinate | TypeCoordinate` (`Type.field` or `Type`).
pub type Coordinate = String;

/// `ObjectTypeState | InterfaceTypeState | UnionTypeState`.
#[derive(Clone, Copy, Debug)]
pub enum RequiresCurrentType<'s> {
    Object(&'s ObjectTypeState),
    Interface(&'s InterfaceTypeState),
    Union(&'s UnionTypeState),
}

impl RequiresCurrentType<'_> {
    /// `currentType.name`.
    fn name(&self) -> &str {
        match self {
            RequiresCurrentType::Object(t) => &t.name,
            RequiresCurrentType::Interface(t) => &t.name,
            RequiresCurrentType::Union(t) => &t.name,
        }
    }

    /// The type as the `required` argument of `canAccess` (`None` for a union, which has no
    /// auth requirements and is never checked).
    fn required_access(&self) -> Option<RequiredAccess<'_>> {
        match self {
            RequiresCurrentType::Object(t) => Some(RequiredAccess::of_object(t)),
            RequiresCurrentType::Interface(t) => Some(RequiredAccess::of_interface(t)),
            RequiresCurrentType::Union(_) => None,
        }
    }
}

/// `ensureAccessToSelectionSet(supergraph, currentType, selectionSet, provisionedAccess)`:
/// the first coordinate the provisioned access cannot reach. Throws on unsupported selections.
pub fn ensure_access_to_selection_set(
    supergraph: &SupergraphState,
    current_type: RequiresCurrentType<'_>,
    selection_set: &SelectionSet,
    provisioned_access: &mut ProvisionedAccess,
) -> JsResult<Option<Coordinate>> {
    // For `@requires` the selection starts on the type that gives the access, so
    // this check always passes. For `@fromContext` it starts on the context
    // provider, which can have its own auth requirements.
    if let Some(required) = current_type.required_access() {
        if !provisioned_access.can_access(required) {
            return Ok(Some(current_type.name().to_string()));
        }
    }

    for selection in &selection_set.selections {
        match selection {
            Selection::Field(field) => {
                // For union types, only __typename can be selected directly (or inline fragments).
                // When __typename is selected, we must verify access to all possible member types.
                if let RequiresCurrentType::Union(union_type) = current_type {
                    if field.name.value != "__typename" {
                        return Err(JsError::error("Cannot select fields directly on union types."));
                    }

                    for member_type_name in &union_type.members {
                        let member_type = supergraph.object_types.get(member_type_name);
                        if let Some(member_type) = member_type {
                            if !provisioned_access.can_access(RequiredAccess::of_object(member_type)) {
                                return Ok(Some(member_type_name.clone()));
                            }
                        }
                    }

                    return Ok(None);
                }

                let field_lacking_access = ensure_access_to_field(supergraph, current_type, field, provisioned_access)?;

                if field_lacking_access.is_some() {
                    return Ok(field_lacking_access);
                }
            }
            Selection::InlineFragment(inline_fragment) => {
                let field_lacking_access =
                    ensure_access_to_inline_fragment(supergraph, current_type, inline_fragment, provisioned_access)?;

                if field_lacking_access.is_some() {
                    return Ok(field_lacking_access);
                }
            }
            Selection::FragmentSpread(_) => {
                return Err(JsError::error("Fragment spreads are not supported in @requires."));
            }
        }
    }

    Ok(None)
}

/// `fieldDetails` of `ensureAccessToField`.
struct FieldDetails {
    r#type: String,
    scopes: Vec<Vec<String>>,
    policies: Vec<Vec<String>>,
    authenticated: bool,
}

/// The output type of a field: `objectTypes ?? interfaceTypes ?? enumTypes ?? scalarTypes ??
/// unionTypes`.
#[derive(Clone, Copy)]
enum OutputType<'s> {
    Object(&'s ObjectTypeState),
    Interface(&'s InterfaceTypeState),
    Enum(&'s EnumTypeState),
    Scalar(&'s ScalarTypeState),
    Union(&'s UnionTypeState),
}

/// `current_type` is an object or interface.
fn ensure_access_to_field(
    supergraph: &SupergraphState,
    current_type: RequiresCurrentType<'_>,
    field_node: &Field,
    provisioned_access: &mut ProvisionedAccess,
) -> JsResult<Option<Coordinate>> {
    if field_node.name.value == "__typename" {
        // __typename is always accessible when its parent type is accessible
        return Ok(None);
    }

    let field_name = &field_node.name.value;
    let current_type_name = current_type.name();

    let mut field_details: Option<FieldDetails> = None;

    // `currentType.fields.get(fieldNode.name.value)`
    let found = match current_type {
        RequiresCurrentType::Object(t) => t.fields.get(field_name).map(|f| FieldDetails {
            r#type: f.r#type.clone(),
            scopes: f.scopes.clone(),
            policies: f.policies.clone(),
            authenticated: f.authenticated,
        }),
        RequiresCurrentType::Interface(t) => t.fields.get(field_name).map(|f| FieldDetails {
            r#type: f.r#type.clone(),
            scopes: f.scopes.clone(),
            policies: f.policies.clone(),
            authenticated: f.authenticated,
        }),
        // NOTE(port): never called with a union (the caller handles unions first).
        RequiresCurrentType::Union(_) => None,
    };

    if let Some(found) = found {
        field_details = Some(found);
    } else {
        match current_type {
            RequiresCurrentType::Interface(t) => {
                return Err(JsError::error(format!(
                    "Field \"{field_name}\" not found on interface type \"{}\".",
                    t.name
                )));
            }
            RequiresCurrentType::Object(object_type) => {
                // Looks like the field is missing, possibly because it's defined on an interface object,
                // so we need to check interfaces implemented by this object type.
                // We need to create a field state that merges all interface definitions
                for interface_name in &object_type.interfaces {
                    let Some(interface_type) = supergraph.interface_types.get(interface_name) else {
                        return Err(JsError::error(format!(
                            "Interface \"{interface_name}\" implemented by \"{}\" not found in supergraph.",
                            object_type.name
                        )));
                    };

                    if let Some(interface_field_state) = interface_type.fields.get(field_name) {
                        field_details = Some(match field_details.take() {
                            None => FieldDetails {
                                r#type: interface_field_state.r#type.clone(),
                                scopes: merge_scope_policies(&[], &interface_field_state.scopes),
                                policies: merge_scope_policies(&[], &interface_field_state.policies),
                                authenticated: interface_field_state.authenticated,
                            },
                            Some(previous) => FieldDetails {
                                r#type: interface_field_state.r#type.clone(),
                                scopes: merge_scope_policies(&previous.scopes, &interface_field_state.scopes),
                                policies: merge_scope_policies(&previous.policies, &interface_field_state.policies),
                                authenticated: if interface_field_state.authenticated {
                                    true
                                } else {
                                    previous.authenticated
                                },
                            },
                        });
                    }
                }
            }
            RequiresCurrentType::Union(_) => {}
        }
    }

    let Some(field_details) = field_details else {
        return Err(JsError::error(format!(
            "Field \"{field_name}\" not found on type \"{current_type_name}\"."
        )));
    };

    if !provisioned_access.can_access(RequiredAccess {
        authenticated: field_details.authenticated,
        policies: &field_details.policies,
        scopes: &field_details.scopes,
    }) {
        return Ok(Some(format!("{current_type_name}.{field_name}")));
    }

    let Some(output_type_name) = extract_named_type_name(&field_details.r#type) else {
        return Err(JsError::error(format!(
            "Unable to extract output type name from field \"{current_type_name}.{field_name}\" type \"{}\".",
            field_details.r#type
        )));
    };

    if is_specified_scalar_type(&output_type_name) {
        // Specified scalars (Int, String, etc.) are always accessible
        return Ok(None);
    }

    let output_type = if let Some(t) = supergraph.object_types.get(&output_type_name) {
        OutputType::Object(t)
    } else if let Some(t) = supergraph.interface_types.get(&output_type_name) {
        OutputType::Interface(t)
    } else if let Some(t) = supergraph.enum_types.get(&output_type_name) {
        OutputType::Enum(t)
    } else if let Some(t) = supergraph.scalar_types.get(&output_type_name) {
        OutputType::Scalar(t)
    } else if let Some(t) = supergraph.union_types.get(&output_type_name) {
        OutputType::Union(t)
    } else {
        return Err(JsError::error(format!(
            "Output type \"{output_type_name}\" of field \"{current_type_name}.{field_name}\" not found in supergraph."
        )));
    };

    let output_required_access = match output_type {
        OutputType::Object(t) => Some(RequiredAccess::of_object(t)),
        OutputType::Interface(t) => Some(RequiredAccess::of_interface(t)),
        OutputType::Enum(t) => Some(RequiredAccess {
            authenticated: t.authenticated,
            policies: &t.policies,
            scopes: &t.scopes,
        }),
        OutputType::Scalar(t) => Some(RequiredAccess {
            authenticated: t.authenticated,
            policies: &t.policies,
            scopes: &t.scopes,
        }),
        OutputType::Union(_) => None,
    };
    if let Some(required) = output_required_access {
        if !provisioned_access.can_access(required) {
            return Ok(Some(format!("{current_type_name}.{field_name}")));
        }
    }

    // Nothing else to check if there is no selection set
    let Some(selection_set) = &field_node.selection_set else {
        return Ok(None);
    };

    let next_type = match output_type {
        OutputType::Enum(EnumTypeState { name, .. }) | OutputType::Scalar(ScalarTypeState { name, .. }) => {
            return Err(JsError::error(format!(
                "Field \"{current_type_name}.{field_name}\" of type \"{name}\" cannot have a selection set."
            )));
        }
        OutputType::Object(t) => RequiresCurrentType::Object(t),
        OutputType::Interface(t) => RequiresCurrentType::Interface(t),
        OutputType::Union(t) => RequiresCurrentType::Union(t),
    };

    ensure_access_to_selection_set(supergraph, next_type, selection_set, provisioned_access)
}

fn ensure_access_to_inline_fragment(
    supergraph: &SupergraphState,
    current_type: RequiresCurrentType<'_>,
    inline_fragment: &InlineFragment,
    provisioned_access: &mut ProvisionedAccess,
) -> JsResult<Option<Coordinate>> {
    let concrete_type = match &inline_fragment.type_condition {
        Some(type_condition) => {
            let name = &type_condition.name.value;
            if let Some(t) = supergraph.object_types.get(name) {
                Some(RequiresCurrentType::Object(t))
            } else {
                supergraph.interface_types.get(name).map(RequiresCurrentType::Interface)
            }
        }
        None => Some(current_type),
    };

    let Some(concrete_type) = concrete_type else {
        return Err(JsError::error(format!(
            "Type \"{}\" not found in supergraph for inline fragment.",
            current_type.name()
        )));
    };

    if matches!(concrete_type, RequiresCurrentType::Union(_)) {
        return Err(JsError::error(
            "Cannot have inline fragments on union types without type conditions.",
        ));
    }

    if let Some(required) = concrete_type.required_access() {
        if !provisioned_access.can_access(required) {
            return Ok(Some(concrete_type.name().to_string()));
        }
    }

    ensure_access_to_selection_set(
        supergraph,
        concrete_type,
        &inline_fragment.selection_set,
        provisioned_access,
    )
}

fn create_access_requirement_error(
    graph_name: &str,
    field_with_requires_coordinate: &str,
    auth_coordinate: &str,
) -> GraphQLError {
    let str_data_ref = if auth_coordinate.contains('.') {
        format!("field \"{auth_coordinate}\"")
    } else {
        format!("type \"{auth_coordinate}\"")
    };

    GraphQLError::with_code(
        format!(
            "[{graph_name}] Field \"{field_with_requires_coordinate}\" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive {str_data_ref} data from @requires selection set."
        ),
        "MISSING_TRANSITIVE_AUTH_REQUIREMENTS",
    )
}

/// The `required` argument of `canAccess`.
#[derive(Clone, Copy, Debug)]
pub struct RequiredAccess<'r> {
    pub authenticated: bool,
    pub policies: &'r [Vec<String>],
    pub scopes: &'r [Vec<String>],
}

impl<'r> RequiredAccess<'r> {
    pub fn of_object(t: &'r ObjectTypeState) -> Self {
        RequiredAccess {
            authenticated: t.authenticated,
            policies: &t.policies,
            scopes: &t.scopes,
        }
    }

    pub fn of_interface(t: &'r InterfaceTypeState) -> Self {
        RequiredAccess {
            authenticated: t.authenticated,
            policies: &t.policies,
            scopes: &t.scopes,
        }
    }
}

/// `ProvisionedAccess`: the auth a field with `@requires` (or `@fromContext`) is guaranteed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProvisionedAccess {
    pub scopes: Vec<Vec<String>>,
    pub policies: Vec<Vec<String>>,
    pub authenticated: bool,
}

impl ProvisionedAccess {
    pub fn new(object_type_state: &ObjectTypeState, field_state: &ObjectTypeFieldState) -> Self {
        let mut this = ProvisionedAccess {
            scopes: Vec::new(),
            policies: Vec::new(),
            authenticated: false,
        };

        if object_type_state.authenticated {
            this.authenticated = true;
        }

        if !object_type_state.scopes.is_empty() {
            this.scopes = object_type_state.scopes.clone();
        }

        if !object_type_state.policies.is_empty() {
            this.policies = object_type_state.policies.clone();
        }

        if field_state.authenticated {
            this.authenticated = true;
        }

        if !field_state.scopes.is_empty() {
            this.scopes = merge_scope_policies(&this.scopes, &field_state.scopes);
        }

        if !field_state.policies.is_empty() {
            this.policies = merge_scope_policies(&this.policies, &field_state.policies);
        }

        this
    }

    pub fn can_access(&self, required: RequiredAccess<'_>) -> bool {
        if required.authenticated && !self.authenticated {
            return false;
        }

        // Check scopes
        for required_scope_group in required.scopes {
            let satisfied_by_any = self
                .scopes
                .iter()
                .any(|provided_group| required_scope_group.iter().all(|scope| provided_group.contains(scope)));

            if !satisfied_by_any {
                return false;
            }
        }

        // Check policies
        for required_policy_group in required.policies {
            let satisfied_by_any = self.policies.iter().any(|provided_group| {
                required_policy_group
                    .iter()
                    .all(|policy| provided_group.contains(policy))
            });

            if !satisfied_by_any {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups(v: &[&[&str]]) -> Vec<Vec<String>> {
        v.iter().map(|g| g.iter().map(|s| s.to_string()).collect()).collect()
    }

    #[test]
    fn can_access_checks_authenticated_scopes_and_policies() {
        let access = ProvisionedAccess {
            scopes: groups(&[&["a", "b"], &["c"]]),
            policies: groups(&[&["p"]]),
            authenticated: false,
        };
        let none: Vec<Vec<String>> = Vec::new();
        assert!(access.can_access(RequiredAccess {
            authenticated: false,
            policies: &none,
            scopes: &none,
        }));
        assert!(!access.can_access(RequiredAccess {
            authenticated: true,
            policies: &none,
            scopes: &none,
        }));
        let scopes = groups(&[&["a"], &["c"]]);
        assert!(access.can_access(RequiredAccess {
            authenticated: false,
            policies: &none,
            scopes: &scopes,
        }));
        let scopes = groups(&[&["a", "c"]]);
        assert!(!access.can_access(RequiredAccess {
            authenticated: false,
            policies: &none,
            scopes: &scopes,
        }));
        let policies = groups(&[&["q"]]);
        assert!(!access.can_access(RequiredAccess {
            authenticated: false,
            policies: &policies,
            scopes: &none,
        }));
        // `[].every(..)` is true: an empty required group is satisfied by any provided group,
        // but not when nothing is provided.
        let empty_group = groups(&[&[]]);
        assert!(access.can_access(RequiredAccess {
            authenticated: false,
            policies: &none,
            scopes: &empty_group,
        }));
        assert!(!ProvisionedAccess::default().can_access(RequiredAccess {
            authenticated: false,
            policies: &none,
            scopes: &empty_group,
        }));
    }

    #[test]
    fn access_requirement_error_names_field_or_type() {
        let e = create_access_requirement_error("a", "T.f", "U.g");
        assert_eq!(
            e.message,
            "[a] Field \"T.f\" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive field \"U.g\" data from @requires selection set."
        );
        assert_eq!(e.code(), Some("MISSING_TRANSITIVE_AUTH_REQUIREMENTS"));
        let e = create_access_requirement_error("a", "T.f", "U");
        assert!(e.message.contains("transitive type \"U\" data"));
    }
}
