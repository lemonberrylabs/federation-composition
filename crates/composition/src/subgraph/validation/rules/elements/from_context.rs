//! Port of `src/subgraph/validation/rules/elements/from-context.ts`.

use graphql_js::{Definition, Directive, GraphQLError, NodeRef, Selection, Value, VisitAction, VisitInfo, Visitor};
use crate::subgraph::context::parse_context_reference;
use crate::subgraph::helpers::{named_type_from_type_node, parse_fields, print_output_type, validate_directive_against_original};
use crate::subgraph::state::{FromContext, SpecName, SubgraphType};
use crate::subgraph::validation::validation_context::{MarkAsUsedReason, SubgraphRule, SubgraphValidationContext};
use graphql_js::{ConstDirective, FieldDefinition, InputValueDefinition, Kind, SelectionSet, Type};
use crate::utils::collections::{IndexMap, IndexSet};
use crate::utils::js::JsResult;

use super::{finish, is_interface_kind, is_object_kind, node_name};

/// `FromContextDirectiveRules(context)`. Fields: `context_owners_by_name` (`indexContextOwners(context)`).
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct FromContextDirectiveRules {
    context_owners_by_name: IndexMap<String, Vec<ContextOwnerName>>,
}

impl<'a> SubgraphRule<'a> for FromContextDirectiveRules {
    const NAME: &'static str = "FromContextDirectiveRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        FromContextDirectiveRules {
            context_owners_by_name: index_context_owners(context),
        }
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for FromContextDirectiveRules {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_js::VisitKinds {
        use graphql_js::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "fromContext", cx),
            NodeRef::Directive(node) => self.directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

impl FromContextDirectiveRules {
    /// The `Directive(node)` callback.
    fn directive<'a>(&self, node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
        if !context.is_available_federation_directive("fromContext", &node.name.value) {
            return Ok(());
        }

        context.state_builder.mark_spec_as_used(SpecName::Context);

        let type_def = context.type_node_info.get_type_def();
        let field_def = context.type_node_info.get_field_def();
        let arg_def = context.type_node_info.get_argument_def();
        let argument_node: Option<&InputValueDefinition> =
            if type_def.map(|type_def| type_def.kind()) == Some(Kind::DirectiveDefinition) {
                match field_def {
                    Some(NodeRef::InputValueDefinition(field_def)) => Some(field_def),
                    _ => None,
                }
            } else {
                arg_def
            };

        let (Some(type_def), Some(field_def), Some(argument_node)) = (type_def, field_def, argument_node) else {
            return Ok(());
        };

        let type_name = node_name(type_def);
        let field_name = node_name(field_def);

        if type_def.kind() == Kind::DirectiveDefinition {
            context.report_error(GraphQLError::with_code(
                format!(
                    "@fromContext argument cannot be used on a directive definition \"@{type_name}({}:)\".",
                    argument_node.name.value
                ),
                "CONTEXT_NOT_SET",
            ));
            return Ok(());
        }

        if !is_object_kind(type_def.kind()) {
            context.report_error(GraphQLError::with_code(
                format!(
                    "@fromContext argument cannot be used on a field that exists on an abstract type \"{}\".",
                    arg_coordinate(context)
                ),
                "CONTEXT_NOT_SET",
            ));
            return Ok(());
        }

        let object_type = context.get_subgraph_object_or_interface_types().get(type_name).cloned();

        if let Some(object_type) = object_type.as_ref().filter(|object_type| !object_type.interfaces.is_empty()) {
            for implemented_interface in &object_type.interfaces {
                let interface_name = &implemented_interface.name.value;
                let interface_type = context.get_subgraph_object_or_interface_types().get(interface_name).cloned();

                if let Some(interface_type) = interface_type.filter(|interface_type| is_interface_kind(interface_type.kind)) {
                    let implemented_field = interface_type.fields.iter().find(|field| field.name.value == field_name);

                    if implemented_field.is_some() {
                        context.report_error(GraphQLError::with_code(
                            format!(
                                "@fromContext argument cannot be used on a field implementing an interface field \"{interface_name}.{field_name}\"."
                            ),
                            "CONTEXT_NOT_SET",
                        ));
                    }
                }
            }
        }

        if argument_node.default_value.is_some() {
            context.report_error(GraphQLError::with_code(
                format!(
                    "@fromContext arguments may not have a default value: \"{}\".",
                    arg_coordinate(context)
                ),
                "CONTEXT_NOT_SET",
            ));
        }

        let field_value = node.arguments.iter().find(|arg| arg.name.value == "field").map(|arg| &arg.value);

        let Some(Value::String(field_value)) = field_value else {
            return Ok(());
        };

        let reference = parse_context_reference(&field_value.value);
        // `!contextName || !selection`: an empty string is falsy too.
        let (Some(context_name), Some(selection)) = (
            reference.context.filter(|name| !name.is_empty()),
            reference.selection.filter(|selection| !selection.is_empty()),
        ) else {
            context.report_error(GraphQLError::with_code(
                format!(
                    "@fromContext argument does not reference a context \"{}\".",
                    field_value.value
                ),
                "NO_CONTEXT_IN_SELECTION",
            ));
            return Ok(());
        };

        let context_owner_names = self
            .context_owners_by_name
            .get(&context_name)
            .filter(|owners| !owners.is_empty());

        let Some(context_owner_names) = context_owner_names else {
            context.report_error(GraphQLError::with_code(
                format!(
                    "Context \"{context_name}\" is used at location \"{}\" but is never set.",
                    arg_coordinate(context)
                ),
                "CONTEXT_NOT_SET",
            ));
            return Ok(());
        };

        validate_selection(
            context,
            ValidateSelectionInput {
                context_owner_names,
                context_name: &context_name,
                selection: &selection,
                arg_def: argument_node,
            },
        )?;

        let has_resolvable_key = object_type
            .as_ref()
            .map(|object_type| object_type.directives.as_slice())
            .unwrap_or_default()
            .iter()
            .any(|directive| {
                if !context.is_available_federation_directive("key", &directive.name.value) {
                    return false;
                }

                let resolvable = directive
                    .arguments
                    .iter()
                    .find(|arg| arg.name.value == "resolvable")
                    .map(|arg| &arg.value);

                match resolvable {
                    Some(Value::Boolean(resolvable)) => resolvable.value,
                    _ => true,
                }
            });

        if !has_resolvable_key {
            context.report_error(GraphQLError::with_code(
                format!(
                    "Object \"{type_name}\" has no resolvable key but has a field with a contextual argument."
                ),
                "CONTEXT_NO_RESOLVABLE_KEY",
            ));
        }

        context.state_builder.object_type().field().arg().set_from_context(
            type_name,
            field_name,
            &argument_node.name.value,
            FromContext {
                context: context_name.clone(),
                selection: selection.clone(),
            },
        )?;

        Ok(())
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

impl SelectableType {
    /// `type.name`.
    fn name(&self) -> &str {
        match self {
            SelectableType::ObjectOrInterface { name, .. } | SelectableType::Union { name, .. } => name,
        }
    }

    /// `type.directives`.
    fn directives(&self) -> &[ConstDirective] {
        match self {
            SelectableType::ObjectOrInterface { directives, .. } | SelectableType::Union { directives, .. } => {
                directives
            }
        }
    }

    /// `type.kind === "OBJECT"`.
    fn is_object(&self) -> bool {
        matches!(self, SelectableType::ObjectOrInterface { is_interface: false, .. })
    }
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
    let mut owners_by_context: IndexMap<String, Vec<ContextOwnerName>> = IndexMap::default();

    for (type_name, type_def) in context.get_subgraph_object_or_interface_types() {
        for directive in &type_def.directives {
            remember_context_owner(
                &mut owners_by_context,
                read_context_directive_name(context, directive).as_deref(),
                type_name,
            );
        }
    }

    for definition in &context.get_document().definitions {
        let (name, directives) = match definition {
            Definition::UnionTypeDefinition(definition) => (&definition.name.value, &definition.directives),
            Definition::UnionTypeExtension(definition) => (&definition.name.value, &definition.directives),
            _ => continue,
        };

        for directive in directives {
            remember_context_owner(
                &mut owners_by_context,
                read_context_directive_name(context, directive).as_deref(),
                name,
            );
        }
    }

    owners_by_context
}

fn read_context_directive_name(context: &SubgraphValidationContext<'_>, directive: &ConstDirective) -> Option<String> {
    if !context.is_available_federation_directive("context", &directive.name.value) {
        return None;
    }

    let name = directive.arguments.iter().find(|arg| arg.name.value == "name").map(|arg| &arg.value);

    match name {
        Some(Value::String(name)) => Some(name.value.clone()),
        _ => None,
    }
}

fn remember_context_owner(
    owners_by_context: &mut IndexMap<String, Vec<ContextOwnerName>>,
    context_name: Option<&str>,
    owner_name: &str,
) {
    // `!contextName`: an empty name is falsy too.
    let Some(context_name) = context_name.filter(|name| !name.is_empty()) else {
        return;
    };

    if let Some(owners) = owners_by_context.get_mut(context_name) {
        owners.push(owner_name.to_string());
    } else {
        owners_by_context.insert(context_name.to_string(), vec![owner_name.to_string()]);
    }
}

fn arg_coordinate(context: &SubgraphValidationContext<'_>) -> String {
    let type_def = context.type_node_info.get_type_def();
    let field_def = context.type_node_info.get_field_def();
    let arg_def = context.type_node_info.get_argument_def();

    // `${x?.name.value}` prints "undefined" for a missing node.
    format!(
        "{}.{}({}:)",
        type_def.map_or("undefined", node_name),
        field_def.map_or("undefined", node_name),
        arg_def.map_or("undefined", |arg_def| arg_def.name.value.as_str())
    )
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
    let mut validator = SelectionValidator {
        context_name: input.context_name,
        arg_def: input.arg_def,
        type_views: IndexMap::default(),
        runtime_type_sets: IndexMap::default(),
    };

    let layout = parse_context_selection(input.selection)?;

    if let ContextSelection::Invalid { reason } = &layout {
        validator.invalid(context, reason);
        return Ok(());
    }

    let mut touched_type_conditions: IndexSet<String> = IndexSet::default();

    for owner_name in input.context_owner_names {
        let Some(owner) = validator.get_selectable_type(context, owner_name) else {
            continue;
        };

        let is_valid = match &layout {
            ContextSelection::Direct { values } => {
                let result = validator.resolve_selected_type(context, &owner, values)?;
                validator.validate_selected_type(context, &result)
            }
            ContextSelection::Conditional { branches } => {
                validator.validate_conditional_selections(context, &owner, branches, &mut touched_type_conditions)?
            }
            // Returned above.
            ContextSelection::Invalid { .. } => true,
        };

        if !is_valid {
            return Ok(());
        }
    }

    if let ContextSelection::Conditional { branches } = &layout {
        for type_condition in branches.keys() {
            if !touched_type_conditions.contains(type_condition) {
                validator.invalid(context, &format!("type condition \"{type_condition}\" is never used."));
                return Ok(());
            }
        }
    }

    Ok(())
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
        report_invalid_selection(context, self.context_name, reason);
    }

    fn get_selectable_type(&mut self, context: &SubgraphValidationContext<'_>, type_name: &str) -> Option<SelectableType> {
        if !self.type_views.contains_key(type_name) {
            self.type_views
                .insert(type_name.to_string(), build_selectable_type(context, type_name));
        }

        self.type_views.get(type_name).cloned().flatten()
    }

    /// The object types the given type can be at runtime.
    fn runtime_set_for(&mut self, context: &SubgraphValidationContext<'_>, ty: &SelectableType) -> IndexSet<String> {
        // NOTE(port): `if (cached)` is always true for a `Set`, even an empty one.
        if let Some(cached) = self.runtime_type_sets.get(ty.name()) {
            return cached.clone();
        }

        let runtime_types: IndexSet<String> = match ty {
            SelectableType::ObjectOrInterface {
                is_interface: false,
                name,
                ..
            } => IndexSet::from_iter([name.clone()]),
            SelectableType::Union { members, .. } => members.iter().cloned().collect(),
            SelectableType::ObjectOrInterface { name, .. } => context
                .get_subgraph_object_or_interface_types()
                .iter()
                .filter(|(_, candidate)| {
                    is_object_kind(candidate.kind) && candidate.interfaces.iter().any(|item| &item.name.value == name)
                })
                .map(|(candidate_name, _)| candidate_name.clone())
                .collect(),
        };

        self.runtime_type_sets.insert(ty.name().to_string(), runtime_types.clone());
        runtime_types
    }

    /// `type` is an object or interface `SelectableType`.
    fn get_field_definition(
        &mut self,
        context: &SubgraphValidationContext<'_>,
        ty: &SelectableType,
        field_name: &str,
    ) -> Option<FieldDefinition> {
        let SelectableType::ObjectOrInterface {
            is_interface,
            fields,
            interfaces,
            ..
        } = ty
        else {
            return None;
        };

        if let Some(field) = fields.get(field_name) {
            return Some(field.clone());
        }

        if *is_interface {
            return None;
        }

        for interface_name in interfaces {
            let interface_type = self.get_selectable_type(context, interface_name);

            let Some(SelectableType::ObjectOrInterface {
                is_interface: true,
                fields: interface_fields,
                ..
            }) = interface_type
            else {
                continue;
            };

            if let Some(interface_field) = interface_fields.get(field_name) {
                return Some(interface_field.clone());
            }
        }

        None
    }

    fn resolve_selected_type(
        &mut self,
        context: &mut SubgraphValidationContext<'_>,
        current_type: &SelectableType,
        selections: &[ContextValue],
    ) -> JsResult<SelectionTypeResult> {
        if current_type.is_object() && has_interface_object_directive(context, current_type) {
            return Ok(SelectionTypeResult::InterfaceObject {
                type_name: current_type.name().to_string(),
            });
        }

        let mut resolved_type: Option<Type> = None;

        for selection in selections {
            let next = self.resolve_selection_type(context, current_type, selection)?;

            let SelectionTypeResult::Resolved { ty: next_type } = next else {
                return Ok(next);
            };

            let Some(resolved) = &resolved_type else {
                resolved_type = Some(next_type);
                continue;
            };

            // Compatibility in both directions requires identical list/nullability shape.
            if !matches_arg_type(resolved, &next_type) || !matches_arg_type(&next_type, resolved) {
                return Ok(SelectionTypeResult::Unresolved);
            }
        }

        Ok(match resolved_type {
            Some(ty) => SelectionTypeResult::Resolved { ty },
            None => SelectionTypeResult::Unresolved,
        })
    }

    fn resolve_selection_type(
        &mut self,
        context: &mut SubgraphValidationContext<'_>,
        current_type: &SelectableType,
        selection: &ContextValue,
    ) -> JsResult<SelectionTypeResult> {
        let (field_name, next) = match selection {
            ContextValue::OnType { on_type, next } => {
                let fragment_type = match on_type.as_deref().filter(|on_type| !on_type.is_empty()) {
                    Some(on_type) => self.get_selectable_type(context, on_type),
                    None => None,
                };

                return match fragment_type {
                    Some(fragment_type) => self.resolve_selected_type(context, &fragment_type, next),
                    None => Ok(SelectionTypeResult::Unresolved),
                };
            }
            ContextValue::Field { field, next } => (field, next),
        };

        if field_name == "__typename" {
            return Ok(SelectionTypeResult::Resolved { ty: typename_type() });
        }

        if matches!(current_type, SelectableType::Union { .. }) {
            return Ok(SelectionTypeResult::Unresolved);
        }

        let Some(field) = self.get_field_definition(context, current_type, field_name) else {
            return Ok(SelectionTypeResult::UnknownField {
                owner: current_type.name().to_string(),
                field: field_name.clone(),
            });
        };

        mark_field_as_used(context, current_type, field_name)?;

        // `!selection.next`: an empty array would be truthy (it cannot be empty: a parsed
        // selection set has at least one selection).
        let Some(next) = next else {
            return Ok(SelectionTypeResult::Resolved {
                ty: nullable_type(&field.ty).clone(),
            });
        };

        let child_type = self.get_selectable_type(context, &named_type_from_type_node(&field.ty).name.value);

        let Some(child_type) = child_type else {
            return Ok(SelectionTypeResult::Unresolved);
        };

        let nested = self.resolve_selected_type(context, &child_type, next)?;

        let SelectionTypeResult::Resolved { ty: nested_type } = nested else {
            return Ok(nested);
        };

        Ok(SelectionTypeResult::Resolved {
            ty: wrap_list_modifiers(&field.ty, &nested_type),
        })
    }

    /// Returns whether the selection is valid. NOTE(port): the skeleton returned `()`; the TS
    /// returns a boolean that the callers test.
    fn validate_selected_type(&mut self, context: &mut SubgraphValidationContext<'_>, result: &SelectionTypeResult) -> bool {
        match result {
            SelectionTypeResult::UnknownField { owner, field } => {
                report_unknown_field(context, self.context_name, owner, field);
                false
            }
            SelectionTypeResult::InterfaceObject { type_name } => {
                report_interface_object(context, self.context_name, type_name);
                false
            }
            SelectionTypeResult::Unresolved => {
                self.invalid(
                    context,
                    &format!(
                        "the type of the selection does not match the expected type \"{}\"",
                        print_output_type(&self.arg_def.ty)
                    ),
                );
                false
            }
            SelectionTypeResult::Resolved { ty } => {
                if !matches_arg_type(ty, &self.arg_def.ty) {
                    self.invalid(
                        context,
                        &format!(
                            "the type of the selection \"{}\" does not match the expected type \"{}\"",
                            print_output_type(ty),
                            print_output_type(&self.arg_def.ty)
                        ),
                    );
                    return false;
                }

                true
            }
        }
    }

    /// Returns whether the branches are valid. NOTE(port): the skeleton returned `()`; the TS
    /// returns a boolean that the caller tests.
    fn validate_conditional_selections(
        &mut self,
        context: &mut SubgraphValidationContext<'_>,
        owner: &SelectableType,
        branches: &IndexMap<String, Vec<ContextValue>>,
        touched_type_conditions: &mut IndexSet<String>,
    ) -> JsResult<bool> {
        let mut saw_applicable_branch = false;
        let runtime_types = self.runtime_set_for(context, owner);

        for (branch_name, selections) in branches {
            // If no branch applies, this function reports that no type condition
            // matches the location. If a branch applies, a later step reports each
            // unused type condition.
            if !runtime_types.contains(branch_name) {
                continue;
            }

            saw_applicable_branch = true;
            touched_type_conditions.insert(branch_name.clone());

            let branch_type = self.get_selectable_type(context, branch_name);

            let Some(branch_type) = branch_type.filter(|branch_type| branch_type.is_object()) else {
                self.invalid(context, "type conditions must be an object type");
                return Ok(false);
            };

            let result = self.resolve_selected_type(context, &branch_type, selections)?;
            if !self.validate_selected_type(context, &result) {
                return Ok(false);
            }
        }

        if !saw_applicable_branch {
            self.invalid(
                context,
                &format!("no type condition matches the location \"{}\"", owner.name()),
            );
            return Ok(false);
        }

        Ok(true)
    }
}

/// `current_type` is an object or interface `SelectableType`.
fn mark_field_as_used(context: &mut SubgraphValidationContext<'_>, current_type: &SelectableType, field_name: &str) -> JsResult<()> {
    let kind = if current_type.is_object() {
        Kind::ObjectTypeDefinition
    } else {
        Kind::InterfaceTypeDefinition
    };
    context.mark_as_used(MarkAsUsedReason::Fields, kind, current_type.name(), field_name)
}

fn report_invalid_selection(context: &mut SubgraphValidationContext<'_>, context_name: &str, reason: &str) {
    let coordinate = arg_coordinate(context);
    context.report_error(GraphQLError::with_code(
        format!("Context \"{context_name}\" is used in \"{coordinate}\" but the selection is invalid: {reason}"),
        "CONTEXT_INVALID_SELECTION",
    ));
}

fn report_unknown_field(context: &mut SubgraphValidationContext<'_>, context_name: &str, owner: &str, field: &str) {
    let coordinate = arg_coordinate(context);
    context.report_error(GraphQLError::with_code(
        format!(
            "Context \"{context_name}\" is used in \"{coordinate}\" but the selection is invalid for type {owner}. Error: Cannot query field \"{field}\" on type \"{owner}\"."
        ),
        "CONTEXT_INVALID_SELECTION",
    ));
}

fn report_interface_object(context: &mut SubgraphValidationContext<'_>, context_name: &str, type_name: &str) {
    let coordinate = arg_coordinate(context);
    context.report_error(GraphQLError::with_code(
        format!(
            "Context \"{context_name}\" is used in \"{coordinate}\" but the selection is invalid: One of the types in the selection is an interfaceObject: \"{type_name}\""
        ),
        "CONTEXT_INVALID_SELECTION",
    ));
}

/// `parseContextSelection(source)`. `parseFields` can throw a `GraphQLError`.
fn parse_context_selection(source: &str) -> JsResult<ContextSelection> {
    let Some(selection_set) = parse_fields(source)? else {
        return Ok(ContextSelection::Invalid {
            reason: "no selection is made".to_string(),
        });
    };

    Ok(context_selection_from_selection_set(&selection_set))
}

/// The part of `parseContextSelection` after `parseFields`.
fn context_selection_from_selection_set(selection_set: &SelectionSet) -> ContextSelection {
    let values = match compile_context_values(selection_set) {
        CompiledContextValues::Invalid { reason } => return ContextSelection::Invalid { reason },
        CompiledContextValues::Values { values } => values,
    };

    let Some((first_selection, remaining_selections)) = values.split_first() else {
        return ContextSelection::Invalid {
            reason: "no selection is made".to_string(),
        };
    };

    if let ContextValue::Field { .. } = first_selection {
        return if remaining_selections.is_empty() {
            ContextSelection::Direct { values }
        } else {
            ContextSelection::Invalid {
                reason: "multiple selections are made".to_string(),
            }
        };
    }

    let mut branches: IndexMap<String, Vec<ContextValue>> = IndexMap::default();

    for selection in &values {
        let (on_type, next) = match selection {
            ContextValue::Field { .. } => {
                return ContextSelection::Invalid {
                    reason: "multiple fields could be selected".to_string(),
                };
            }
            ContextValue::OnType { on_type, next } => (on_type, next),
        };

        // `!selection.onType`: an empty name is falsy too (a parsed name cannot be empty).
        let Some(on_type) = on_type.as_ref().filter(|on_type| !on_type.is_empty()) else {
            return ContextSelection::Invalid {
                reason: "inline fragments must have type conditions".to_string(),
            };
        };

        // `Map.set` on an existing key keeps its position, like `IndexMap::insert`.
        branches.insert(on_type.clone(), next.clone());
    }

    if branches.len() == values.len() {
        ContextSelection::Conditional { branches }
    } else {
        ContextSelection::Invalid {
            reason: "type conditions have same name".to_string(),
        }
    }
}

fn build_selectable_type(context: &SubgraphValidationContext<'_>, type_name: &str) -> Option<SelectableType> {
    if let Some(object_or_interface) = context.get_subgraph_object_or_interface_types().get(type_name) {
        let mut fields: IndexMap<String, FieldDefinition> = IndexMap::default();
        for field in &object_or_interface.fields {
            fields.insert(field.name.value.clone(), field.clone());
        }

        return Some(SelectableType::ObjectOrInterface {
            is_interface: is_interface_kind(object_or_interface.kind),
            name: type_name.to_string(),
            fields,
            interfaces: object_or_interface
                .interfaces
                .iter()
                .map(|item| item.name.value.clone())
                .collect(),
            directives: object_or_interface.directives.clone(),
        });
    }

    if let Some(union_members) = context.get_subgraph_union_types().get(type_name) {
        let union_state = context.state_builder.state.types.get(type_name);
        return Some(SelectableType::Union {
            name: type_name.to_string(),
            members: union_members.iter().cloned().collect(),
            directives: match union_state {
                Some(SubgraphType::Union(union_state)) => union_state.ast.directives.clone(),
                _ => Vec::new(),
            },
        });
    }

    None
}

fn has_interface_object_directive(context: &SubgraphValidationContext<'_>, ty: &SelectableType) -> bool {
    ty.directives()
        .iter()
        .any(|directive| context.is_available_federation_directive("interfaceObject", &directive.name.value))
}

fn nullable_type(ty: &Type) -> &Type {
    match ty {
        Type::NonNull(n) => &n.ty,
        other => other,
    }
}

fn wrap_list_modifiers(ty: &Type, resolved_type: &Type) -> Type {
    let ty = nullable_type(ty);
    if let Type::List(list) = ty {
        return Type::list(wrap_list_modifiers(&list.ty, resolved_type));
    }

    resolved_type.clone()
}

fn matches_arg_type(select_type: &Type, arg_type: &Type) -> bool {
    if let Type::NonNull(arg_inner) = arg_type {
        let Type::NonNull(select_inner) = select_type else {
            return false;
        };
        return matches_arg_type(&select_inner.ty, &arg_inner.ty);
    }

    // A non-null selection satisfies a nullable argument.
    if let Type::NonNull(select_inner) = select_type {
        return matches_arg_type(&select_inner.ty, arg_type);
    }

    if let Type::List(arg_inner) = arg_type {
        let Type::List(select_inner) = select_type else {
            return false;
        };
        return matches_arg_type(&select_inner.ty, &arg_inner.ty);
    }

    match (select_type, arg_type) {
        (Type::Named(select_named), Type::Named(arg_named)) => select_named.name.value == arg_named.name.value,
        // `selectType.kind === Kind.LIST_TYPE` (the argument is a named type here).
        _ => false,
    }
}

fn compile_context_values(selection_set: &SelectionSet) -> CompiledContextValues {
    let mut values: Vec<ContextValue> = Vec::new();
    for selection in &selection_set.selections {
        let (directives, nested_selection_set) = match selection {
            Selection::FragmentSpread(_) => {
                return CompiledContextValues::Invalid {
                    reason: "fragment spread is not allowed".to_string(),
                };
            }
            Selection::Field(field) => {
                if field.alias.is_some() {
                    return CompiledContextValues::Invalid {
                        reason: "aliases are not allowed in the selection".to_string(),
                    };
                }
                (&field.directives, field.selection_set.as_ref())
            }
            Selection::InlineFragment(fragment) => (&fragment.directives, Some(&fragment.selection_set)),
        };

        if !directives.is_empty() {
            return CompiledContextValues::Invalid {
                reason: "directives are not allowed in the selection".to_string(),
            };
        }

        let nested = match nested_selection_set.map(compile_context_values) {
            Some(CompiledContextValues::Invalid { reason }) => return CompiledContextValues::Invalid { reason },
            Some(CompiledContextValues::Values { values }) => Some(values),
            None => None,
        };

        values.push(match selection {
            Selection::Field(field) => ContextValue::Field {
                field: field.name.value.clone(),
                next: nested,
            },
            _ => ContextValue::OnType {
                on_type: match selection {
                    Selection::InlineFragment(fragment) => {
                        fragment.type_condition.as_ref().map(|condition| condition.name.value.clone())
                    }
                    _ => None,
                },
                next: nested.unwrap_or_default(),
            },
        });
    }

    CompiledContextValues::Values { values }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphql_js::{parse, parse_type, Definition};

    fn selection_set(source: &str) -> SelectionSet {
        let document = parse(source).expect("valid query");
        match document.definitions.into_iter().next() {
            Some(Definition::OperationDefinition(operation)) => operation.selection_set,
            _ => panic!("expected an operation"),
        }
    }

    fn ty(source: &str) -> Type {
        parse_type(source).expect("valid type")
    }

    fn field(name: &str, next: Option<Vec<ContextValue>>) -> ContextValue {
        ContextValue::Field {
            field: name.to_string(),
            next,
        }
    }

    fn on_type(name: &str, next: Vec<ContextValue>) -> ContextValue {
        ContextValue::OnType {
            on_type: Some(name.to_string()),
            next,
        }
    }

    fn invalid(reason: &str) -> ContextSelection {
        ContextSelection::Invalid {
            reason: reason.to_string(),
        }
    }

    #[test]
    fn compiles_nested_fields_and_fragments() {
        assert_eq!(
            compile_context_values(&selection_set("{ a { b c } ... on T { d } ... { e } }")),
            CompiledContextValues::Values {
                values: vec![
                    field("a", Some(vec![field("b", None), field("c", None)])),
                    on_type("T", vec![field("d", None)]),
                    ContextValue::OnType {
                        on_type: None,
                        next: vec![field("e", None)],
                    },
                ],
            }
        );
    }

    #[test]
    fn rejects_spreads_aliases_and_directives() {
        let reason = |source: &str| match compile_context_values(&selection_set(source)) {
            CompiledContextValues::Invalid { reason } => reason,
            other => panic!("expected invalid, got {other:?}"),
        };
        assert_eq!(reason("{ ...F }"), "fragment spread is not allowed");
        assert_eq!(reason("{ x: a }"), "aliases are not allowed in the selection");
        assert_eq!(reason("{ a @skip(if: true) }"), "directives are not allowed in the selection");
        assert_eq!(reason("{ ... on T @x { a } }"), "directives are not allowed in the selection");
        // Nested errors bubble up.
        assert_eq!(reason("{ a { x: b } }"), "aliases are not allowed in the selection");
    }

    #[test]
    fn classifies_selections() {
        let classify = |source: &str| context_selection_from_selection_set(&selection_set(source));

        assert_eq!(
            classify("{ id }"),
            ContextSelection::Direct {
                values: vec![field("id", None)]
            }
        );
        assert_eq!(classify("{ id name }"), invalid("multiple selections are made"));
        assert_eq!(classify("{ ... on A { id } id }"), invalid("multiple fields could be selected"));
        assert_eq!(classify("{ ... { id } }"), invalid("inline fragments must have type conditions"));
        assert_eq!(
            classify("{ ... on A { id } ... on A { name } }"),
            invalid("type conditions have same name")
        );

        let mut branches = IndexMap::default();
        branches.insert("A".to_string(), vec![field("id", None)]);
        branches.insert("B".to_string(), vec![field("name", None)]);
        assert_eq!(
            classify("{ ... on A { id } ... on B { name } }"),
            ContextSelection::Conditional { branches }
        );
    }

    #[test]
    fn arg_type_matching() {
        assert!(matches_arg_type(&ty("String"), &ty("String")));
        assert!(matches_arg_type(&ty("String!"), &ty("String")));
        assert!(!matches_arg_type(&ty("String"), &ty("String!")));
        assert!(!matches_arg_type(&ty("String"), &ty("ID")));
        assert!(matches_arg_type(&ty("[String!]!"), &ty("[String]")));
        assert!(!matches_arg_type(&ty("[String]"), &ty("String")));
        assert!(!matches_arg_type(&ty("String"), &ty("[String]")));
        assert!(!matches_arg_type(&ty("[String]"), &ty("[String!]")));
    }

    #[test]
    fn wraps_list_modifiers_without_nullability() {
        assert_eq!(wrap_list_modifiers(&ty("[[A!]!]!"), &ty("String!")), ty("[[String!]]"));
        assert_eq!(wrap_list_modifiers(&ty("A!"), &ty("String")), ty("String"));
        assert_eq!(nullable_type(&ty("A!")), &ty("A"));
        assert_eq!(nullable_type(&ty("[A]")), &ty("[A]"));
    }
}
