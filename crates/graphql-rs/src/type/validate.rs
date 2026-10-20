//! Port of graphql-js `type/validate`: `validateSchema` and
//! `assertValidSchema`.
//!
//! NOTE(port): the AST nodes graphql-js attaches to each error are not
//! collected (this crate's `GraphQLError` carries no nodes), so the helpers
//! that only find them (`getOperationTypeNode`, `getAllImplementsInterfaceNodes`,
//! `getUnionMemberTypeNodes`, `getDeprecatedDirectiveNode`) are not ported.

use crate::ast::{OperationType, Value};
use crate::collections::{IndexMap, IndexSet};
use crate::error::GraphQLError;
use crate::jsutils::capitalize::capitalize;
use crate::jsutils::format_list::and_list;
use crate::jsutils::print_path_array::print_path_array;
use crate::jsutils::JsValue;
use crate::utilities::type_comparators::{is_equal_type, is_type_sub_type_of};
use crate::utilities::validate_input_value::{validate_input_literal, validate_input_value};

use super::definition::{
    is_input_type, is_output_type, is_required_argument, is_required_input_field, DefaultInput, GraphQLEnumType,
    GraphQLField, GraphQLInputObjectType, GraphQLInputValue, GraphQLNamedType, GraphQLType, GraphQLUnionType,
    TypeLookup,
};
use super::introspection::is_introspection_type;
use super::schema::GraphQLSchema;

/// graphql-js `validateSchema(schema)`: the schema's validation errors,
/// computed once. `Err` is a throw: resolving an enum's values (which
/// graphql-js does lazily, here) can throw.
pub fn validate_schema(schema: &GraphQLSchema) -> Result<Vec<GraphQLError>, GraphQLError> {
    if let Some(errors) = schema.validation_errors.get() {
        return Ok(errors.clone());
    }
    let mut context = SchemaValidationContext {
        errors: Vec::new(),
        schema,
    };
    validate_root_types(&mut context);
    validate_directives(&mut context);
    validate_types(&mut context)?;
    let errors = context.errors;
    Ok(schema.validation_errors.get_or_init(|| errors).clone())
}

/// graphql-js `assertValidSchema(schema)`: throws an `Error` with the
/// messages of all errors joined by blank lines (or the error
/// `validateSchema` throws).
pub fn assert_valid_schema(schema: &GraphQLSchema) -> Result<(), String> {
    let errors = validate_schema(schema).map_err(|error| error.message)?;
    if !errors.is_empty() {
        let messages: Vec<&str> = errors.iter().map(|error| error.message.as_str()).collect();
        return Err(messages.join("\n\n"));
    }
    Ok(())
}

/// graphql-js `SchemaValidationContext`.
struct SchemaValidationContext<'s> {
    errors: Vec<GraphQLError>,
    schema: &'s GraphQLSchema,
}

impl SchemaValidationContext<'_> {
    fn report_error(&mut self, message: String) {
        self.errors.push(GraphQLError::new(message));
    }
}

fn validate_root_types(context: &mut SchemaValidationContext<'_>) {
    let schema = context.schema;
    if schema.get_query_type().is_none() {
        context.report_error("Query root type must be provided.".to_string());
    }
    // `AccumulatorMap` keyed by the root type object.
    let mut root_types_map: IndexMap<&str, Vec<OperationType>> = IndexMap::default();
    for operation_type in [
        OperationType::Query,
        OperationType::Mutation,
        OperationType::Subscription,
    ] {
        if let Some(root_type) = schema.get_root_type(operation_type) {
            if !root_type.is_object_type() {
                let operation_type_str = capitalize(operation_type.as_str());
                let root_type_str = root_type.name();
                context.report_error(if operation_type == OperationType::Query {
                    format!("{operation_type_str} root type must be Object type, it cannot be {root_type_str}.")
                } else {
                    format!(
                        "{operation_type_str} root type must be Object type if provided, it cannot be {root_type_str}."
                    )
                });
            } else {
                root_types_map.entry(root_type.name()).or_default().push(operation_type);
            }
        }
    }
    for (root_type, operation_types) in root_types_map {
        if operation_types.len() > 1 {
            let operations: Vec<String> = operation_types.iter().map(|o| o.as_str().to_string()).collect();
            let operation_list = and_list(&operations);
            context.report_error(format!(
                "All root types must be different, \"{root_type}\" type is used as {operation_list} root types."
            ));
        }
    }
}

fn validate_directives(context: &mut SchemaValidationContext<'_>) {
    let schema = context.schema;
    for directive in schema.get_directives() {
        // NOTE(port): every directive is a `GraphQLDirective`, so the
        // `isDirective` check never fails.
        validate_name(context, &directive.name);
        if directive.locations.is_empty() {
            context.report_error(format!("Directive {directive} must include 1 or more locations."));
        }
        for arg in &directive.args {
            validate_name(context, &arg.name);
            if !is_input_type(schema, &arg.ty) {
                context.report_error(format!("The type of {arg} must be Input Type but got: {}.", arg.ty));
            }
            if is_required_argument(arg) && arg.deprecation_reason.is_some() {
                context.report_error(format!("Required argument {arg} cannot be deprecated."));
            }
            validate_default_value(context, arg);
        }
    }
}

fn validate_default_value(context: &mut SchemaValidationContext<'_>, input_value: &dyn GraphQLInputValue) {
    let Some(default_input) = input_value.default_input() else {
        return;
    };
    let mut errors: Vec<(GraphQLError, Vec<crate::jsutils::path::PathItem>)> = Vec::new();
    validate_default_input(
        default_input,
        input_value.input_type(),
        &mut |error, path| errors.push((error, path)),
        false,
        context.schema,
    );
    if errors.is_empty() {
        return;
    }
    if let DefaultInput::Value(_) = default_input {
        // NOTE(port): graphql-js tries `uncoerceDefaultValue` here, to suggest
        // the value a `{ value }` default meant. Such defaults come only from
        // the introspection types, which are valid, so this is never reached
        // and is not ported.
    }
    for (error, path) in errors {
        context.report_error(format!(
            "{input_value} has invalid default value{}: {}",
            print_path_array(&path),
            error.message
        ));
    }
}

/// graphql-js `validateDefaultInput(defaultInput, inputType, onError, hideSuggestions)`.
pub fn validate_default_input(
    default_input: &DefaultInput,
    input_type: &GraphQLType,
    on_error: &mut crate::utilities::validate_input_value::OnError<'_>,
    hide_suggestions: bool,
    types: &dyn TypeLookup,
) {
    match default_input {
        DefaultInput::Literal(literal) => {
            validate_input_literal(literal, input_type, on_error, hide_suggestions, types)
        }
        DefaultInput::Value(value) => validate_input_value(Some(value), input_type, on_error, hide_suggestions, types),
    }
}

fn validate_name(context: &mut SchemaValidationContext<'_>, name: &str) {
    if name.starts_with("__") {
        context.report_error(format!(
            "Name \"{name}\" must not begin with \"__\", which is reserved by GraphQL introspection."
        ));
    }
}

/// graphql-js `finiteValueStates` entries.
struct FiniteValueState<'s> {
    input_obj: &'s GraphQLInputObjectType,
    /// `{ field, target }`: field names and target input object names.
    targets: Vec<(&'s str, &'s str)>,
    /// Indexes into the states.
    dependents: Vec<usize>,
    unresolved_target_count: usize,
    has_finite_value: bool,
}

fn validate_types(context: &mut SchemaValidationContext<'_>) -> Result<(), GraphQLError> {
    let schema = context.schema;
    let mut validate_input_object_default_value_circular_refs = InputObjectDefaultValueCircularRefsValidator::default();
    let type_map = schema.get_type_map();
    let mut finite_value_states: IndexMap<&str, FiniteValueState<'_>> = IndexMap::default();
    for named_type in type_map.values() {
        if !is_introspection_type(named_type.name()) {
            validate_name(context, named_type.name());
        }
        match named_type {
            GraphQLNamedType::Object(object) => {
                validate_fields(context, &object.name, object.fields());
                validate_interfaces(context, named_type, object.interfaces(), object.fields());
            }
            GraphQLNamedType::Interface(interface) => {
                validate_fields(context, &interface.name, interface.fields());
                validate_interfaces(context, named_type, interface.interfaces(), interface.fields());
            }
            GraphQLNamedType::Union(union) => validate_union_members(context, union),
            GraphQLNamedType::Enum(enum_type) => validate_enum_values(context, enum_type)?,
            GraphQLNamedType::InputObject(input_obj) => {
                validate_input_fields(context, input_obj);
                finite_value_states.insert(
                    &input_obj.name,
                    FiniteValueState {
                        input_obj,
                        targets: Vec::new(),
                        dependents: Vec::new(),
                        unresolved_target_count: 0,
                        has_finite_value: false,
                    },
                );
                validate_input_object_default_value_circular_refs.validate(context, input_obj);
            }
            GraphQLNamedType::Scalar(_) => {}
        }
    }
    detect_input_object_non_finite_values(context, finite_value_states);
    Ok(())
}

fn validate_fields(
    context: &mut SchemaValidationContext<'_>,
    type_name: &str,
    fields: &IndexMap<String, GraphQLField>,
) {
    let schema = context.schema;
    if fields.is_empty() {
        context.report_error(format!("Type {type_name} must define one or more fields."));
    }
    for field in fields.values() {
        validate_name(context, &field.name);
        if !is_output_type(schema, &field.ty) {
            context.report_error(format!(
                "The type of {field} must be Output Type but got: {}.",
                field.ty
            ));
        }
        for arg in &field.args {
            validate_name(context, &arg.name);
            if !is_input_type(schema, &arg.ty) {
                context.report_error(format!("The type of {arg} must be Input Type but got: {}.", arg.ty));
            }
            if is_required_argument(arg) && arg.deprecation_reason.is_some() {
                context.report_error(format!("Required argument {arg} cannot be deprecated."));
            }
            validate_default_value(context, arg);
        }
    }
}

/// The `getInterfaces()` and `getFields()` of an object or interface type.
fn interfaces_and_fields(named_type: &GraphQLNamedType) -> Option<(&[String], &IndexMap<String, GraphQLField>)> {
    match named_type {
        GraphQLNamedType::Object(t) => Some((t.interfaces(), t.fields())),
        GraphQLNamedType::Interface(t) => Some((t.interfaces(), t.fields())),
        _ => None,
    }
}

fn validate_interfaces(
    context: &mut SchemaValidationContext<'_>,
    named_type: &GraphQLNamedType,
    interfaces: &[String],
    fields: &IndexMap<String, GraphQLField>,
) {
    let schema = context.schema;
    let mut iface_type_names: IndexSet<&str> = IndexSet::default();
    for iface_name in interfaces {
        let iface = schema.get_type(iface_name).expect("interfaces are in the type map");
        if !iface.is_interface_type() {
            context.report_error(format!(
                "Type {named_type} must only implement Interface types, it cannot implement {iface}."
            ));
            continue;
        }
        if named_type.name() == iface.name() {
            context.report_error(format!(
                "Type {named_type} cannot implement itself because it would create a circular reference."
            ));
            continue;
        }
        if iface_type_names.contains(iface.name()) {
            context.report_error(format!("Type {named_type} can only implement {iface} once."));
            continue;
        }
        iface_type_names.insert(iface.name());
        validate_type_implements_ancestors(context, named_type, interfaces, iface);
        validate_type_implements_interface(context, named_type, fields, iface);
    }
}

fn validate_type_implements_interface(
    context: &mut SchemaValidationContext<'_>,
    named_type: &GraphQLNamedType,
    type_field_map: &IndexMap<String, GraphQLField>,
    iface: &GraphQLNamedType,
) {
    let schema = context.schema;
    let (_, iface_fields) = interfaces_and_fields(iface).expect("an interface type");
    for iface_field in iface_fields.values() {
        let Some(type_field) = type_field_map.get(&iface_field.name) else {
            context.report_error(format!(
                "Interface field {iface_field} expected but {named_type} does not provide it."
            ));
            continue;
        };
        if !is_type_sub_type_of(schema, &type_field.ty, &iface_field.ty) {
            context.report_error(format!(
                "Interface field {iface_field} expects type {} but {type_field} is type {}.",
                iface_field.ty, type_field.ty
            ));
        }
        for iface_arg in &iface_field.args {
            let Some(type_arg) = type_field.args.iter().find(|arg| arg.name == iface_arg.name) else {
                context.report_error(format!(
                    "Interface field argument {iface_arg} expected but {type_field} does not provide it."
                ));
                continue;
            };
            if !is_equal_type(&iface_arg.ty, &type_arg.ty) {
                context.report_error(format!(
                    "Interface field argument {iface_arg} expects type {} but {type_arg} is type {}.",
                    iface_arg.ty, type_arg.ty
                ));
            }
        }
        for type_arg in &type_field.args {
            if is_required_argument(type_arg) {
                let iface_arg = iface_field.args.iter().find(|arg| arg.name == type_arg.name);
                if iface_arg.is_none() {
                    context.report_error(format!(
                        "Argument \"{type_arg}\" must not be required type \"{}\" if not provided by the Interface field \"{iface_field}\".",
                        type_arg.ty
                    ));
                }
            }
        }
        if type_field.deprecation_reason.is_some() && iface_field.deprecation_reason.is_none() {
            context.report_error(format!(
                "Interface field {}.{} is not deprecated, so implementation field {}.{} must not be deprecated.",
                iface.name(),
                iface_field.name,
                named_type.name(),
                type_field.name
            ));
        }
    }
}

fn validate_type_implements_ancestors(
    context: &mut SchemaValidationContext<'_>,
    named_type: &GraphQLNamedType,
    iface_interfaces: &[String],
    iface: &GraphQLNamedType,
) {
    let (transitives, _) = interfaces_and_fields(iface).expect("an interface type");
    for transitive in transitives {
        if !iface_interfaces.contains(transitive) {
            context.report_error(if transitive == named_type.name() {
                format!("Type {named_type} cannot implement {iface} because it would create a circular reference.")
            } else {
                format!("Type {named_type} must implement {transitive} because it is implemented by {iface}.")
            });
        }
    }
}

fn validate_union_members(context: &mut SchemaValidationContext<'_>, union: &GraphQLUnionType) {
    let schema = context.schema;
    let member_types = union.types();
    if member_types.is_empty() {
        context.report_error(format!(
            "Union type {} must define one or more member types.",
            union.name
        ));
    }
    let mut included_type_names: IndexSet<&str> = IndexSet::default();
    for member_type in member_types {
        if included_type_names.contains(member_type.as_str()) {
            context.report_error(format!(
                "Union type {} can only include type {member_type} once.",
                union.name
            ));
            continue;
        }
        included_type_names.insert(member_type);
        if !schema
            .get_type(member_type)
            .is_some_and(GraphQLNamedType::is_object_type)
        {
            context.report_error(format!(
                "Union type {} can only include Object types, it cannot include {member_type}.",
                union.name
            ));
        }
    }
}

fn validate_enum_values(
    context: &mut SchemaValidationContext<'_>,
    enum_type: &GraphQLEnumType,
) -> Result<(), GraphQLError> {
    let enum_values = enum_type.get_values()?;
    if enum_values.is_empty() {
        context.report_error(format!("Enum type {} must define one or more values.", enum_type.name));
    }
    for enum_value in enum_values {
        validate_name(context, &enum_value.name);
    }
    Ok(())
}

fn validate_input_fields(context: &mut SchemaValidationContext<'_>, input_obj: &GraphQLInputObjectType) {
    let schema = context.schema;
    let fields = input_obj.fields();
    if fields.is_empty() {
        context.report_error(format!(
            "Input Object type {} must define one or more fields.",
            input_obj.name
        ));
    }
    for field in fields.values() {
        validate_name(context, &field.name);
        if !is_input_type(schema, &field.ty) {
            context.report_error(format!("The type of {field} must be Input Type but got: {}.", field.ty));
        }
        if is_required_input_field(field) && field.deprecation_reason.is_some() {
            context.report_error(format!("Required input field {field} cannot be deprecated."));
        }
        validate_default_value(context, field);
        if input_obj.is_one_of {
            validate_one_of_input_object_field(input_obj, field, context);
        }
    }
}

fn validate_one_of_input_object_field(
    ty: &GraphQLInputObjectType,
    field: &super::definition::GraphQLInputField,
    context: &mut SchemaValidationContext<'_>,
) {
    if field.ty.is_non_null() {
        context.report_error(format!(
            "OneOf input field {}.{} must be nullable.",
            ty.name, field.name
        ));
    }
    if field.default.is_some() {
        context.report_error(format!(
            "OneOf input field {}.{} cannot have a default value.",
            ty.name, field.name
        ));
    }
}

fn detect_input_object_non_finite_values<'s>(
    context: &mut SchemaValidationContext<'s>,
    mut finite_value_states: IndexMap<&'s str, FiniteValueState<'s>>,
) {
    let schema = context.schema;
    let mut input_objects_with_finite_values: Vec<usize> = Vec::new();
    fn mark_input_object_has_finite_value(
        states: &mut IndexMap<&str, FiniteValueState<'_>>,
        stack: &mut Vec<usize>,
        index: usize,
    ) {
        let state = &mut states[index];
        if !state.has_finite_value {
            state.has_finite_value = true;
            stack.push(index);
        }
    }
    for index in 0..finite_value_states.len() {
        let input_obj = finite_value_states[index].input_obj;
        let fields = input_obj.fields();
        for field in fields.values() {
            let Some(target) = get_finite_value_target(schema, input_obj, &field.ty) else {
                continue;
            };
            finite_value_states[index].targets.push((field.name.as_str(), target));
            if let Some(target_state) = finite_value_states.get_mut(target) {
                target_state.dependents.push(index);
            }
        }
        let targets_len = finite_value_states[index].targets.len();
        if input_obj.is_one_of {
            if fields.is_empty() || targets_len < fields.len() {
                mark_input_object_has_finite_value(
                    &mut finite_value_states,
                    &mut input_objects_with_finite_values,
                    index,
                );
            }
        } else {
            finite_value_states[index].unresolved_target_count = targets_len;
            if targets_len == 0 {
                mark_input_object_has_finite_value(
                    &mut finite_value_states,
                    &mut input_objects_with_finite_values,
                    index,
                );
            }
        }
    }
    while let Some(next_finite_value_state) = input_objects_with_finite_values.pop() {
        let dependents = finite_value_states[next_finite_value_state].dependents.clone();
        for dependent_state in dependents {
            if finite_value_states[dependent_state].has_finite_value {
                continue;
            }
            if finite_value_states[dependent_state].input_obj.is_one_of {
                mark_input_object_has_finite_value(
                    &mut finite_value_states,
                    &mut input_objects_with_finite_values,
                    dependent_state,
                );
                continue;
            }
            finite_value_states[dependent_state].unresolved_target_count -= 1;
            if finite_value_states[dependent_state].unresolved_target_count == 0 {
                mark_input_object_has_finite_value(
                    &mut finite_value_states,
                    &mut input_objects_with_finite_values,
                    dependent_state,
                );
            }
        }
    }

    let mut cycles = CycleReporter {
        states: &finite_value_states,
        visited_types: IndexSet::default(),
        field_path: Vec::new(),
        field_path_index_by_type: IndexMap::default(),
        errors: Vec::new(),
    };
    for index in 0..finite_value_states.len() {
        if !finite_value_states[index].has_finite_value {
            cycles.report_cycle_recursive(index);
        }
    }
    for message in cycles.errors {
        context.report_error(message);
    }
}

/// The `reportCycleRecursive` closure and the locals it shares.
struct CycleReporter<'r, 's> {
    states: &'r IndexMap<&'s str, FiniteValueState<'s>>,
    visited_types: IndexSet<&'s str>,
    field_path: Vec<String>,
    field_path_index_by_type: IndexMap<&'s str, usize>,
    errors: Vec<String>,
}

impl<'s> CycleReporter<'_, 's> {
    fn report_cycle_recursive(&mut self, index: usize) {
        let state = &self.states[index];
        let input_obj = state.input_obj;
        let name = input_obj.name.as_str();
        if self.visited_types.contains(name) {
            return;
        }
        self.visited_types.insert(name);
        self.field_path_index_by_type.insert(name, self.field_path.len());
        for &(field_name, target) in &state.targets {
            let Some(target_index) = self.states.get_index_of(target) else {
                continue;
            };
            if self.states[target_index].has_finite_value {
                continue;
            }
            let cycle_index = self.field_path_index_by_type.get(target).copied();
            self.field_path.push(format!("{name}.{field_name}"));
            match cycle_index {
                None => self.report_cycle_recursive(target_index),
                Some(cycle_index) => {
                    let path_str = self.field_path[cycle_index..].join(", ");
                    self.errors.push(format!(
                        "Input Object {target} cannot be provided a finite value because it references itself through fields: {path_str}."
                    ));
                }
            }
            self.field_path.pop();
        }
        self.field_path_index_by_type.shift_remove(name);
    }
}

/// graphql-js `getFiniteValueTarget`: the input object a field needs a
/// finite value of, by name.
fn get_finite_value_target<'s>(
    schema: &'s GraphQLSchema,
    input_obj: &GraphQLInputObjectType,
    field_type: &'s GraphQLType,
) -> Option<&'s str> {
    let is_input_object = |name: &str| {
        schema
            .get_type(name)
            .is_some_and(GraphQLNamedType::is_input_object_type)
    };
    if input_obj.is_one_of {
        if let GraphQLType::Named(name) = field_type {
            if is_input_object(name) {
                return Some(name);
            }
        }
        return None;
    }
    if let GraphQLType::NonNull(of_type) = field_type {
        if let GraphQLType::Named(name) = of_type.as_ref() {
            if is_input_object(name) {
                return Some(name);
            }
        }
    }
    None
}

/// graphql-js `createInputObjectDefaultValueCircularRefsValidator`: the
/// returned function is [`Self::validate`]; the closure locals are fields.
#[derive(Default)]
struct InputObjectDefaultValueCircularRefsValidator {
    visited_fields: IndexSet<String>,
    /// `[fieldStr, astNode]` entries; the nodes are not tracked.
    field_path: Vec<String>,
    /// `fieldStr` to the length of `field_path` after its push.
    field_path_index: IndexMap<String, usize>,
}

impl InputObjectDefaultValueCircularRefsValidator {
    fn validate(&mut self, context: &mut SchemaValidationContext<'_>, input_obj: &GraphQLInputObjectType) {
        self.detect_value_default_value_cycle(context, input_obj, &JsValue::Object(IndexMap::default()));
    }

    fn input_object_of<'s>(
        context: &SchemaValidationContext<'s>,
        ty: &GraphQLType,
    ) -> Option<&'s GraphQLInputObjectType> {
        match context.schema.get_type(ty.named_type()) {
            Some(GraphQLNamedType::InputObject(input_object)) => Some(input_object),
            _ => None,
        }
    }

    fn detect_value_default_value_cycle(
        &mut self,
        context: &mut SchemaValidationContext<'_>,
        input_obj: &GraphQLInputObjectType,
        default_value: &JsValue,
    ) {
        let object = match default_value {
            JsValue::List(items) => {
                for item_value in items {
                    self.detect_value_default_value_cycle(context, input_obj, item_value);
                }
                return;
            }
            JsValue::Object(object) => object,
            _ => return,
        };
        for field in input_obj.fields().values() {
            let Some(named_field_type) = Self::input_object_of(context, &field.ty) else {
                continue;
            };
            match object.get(&field.name) {
                Some(field_value) => self.detect_value_default_value_cycle(context, named_field_type, field_value),
                None => self.detect_field_default_value_cycle(
                    context,
                    field,
                    named_field_type,
                    format!("{}.{}", input_obj.name, field.name),
                ),
            }
        }
    }

    fn detect_literal_default_value_cycle(
        &mut self,
        context: &mut SchemaValidationContext<'_>,
        input_obj: &GraphQLInputObjectType,
        default_value: &Value,
    ) {
        let object = match default_value {
            Value::List(list) => {
                for item_literal in &list.values {
                    self.detect_literal_default_value_cycle(context, input_obj, item_literal);
                }
                return;
            }
            Value::Object(object) => object,
            _ => return,
        };
        // `keyMap`: the last field of a name wins.
        let mut field_nodes = IndexMap::default();
        for field in &object.fields {
            field_nodes.insert(field.name.value.as_str(), field);
        }
        for field in input_obj.fields().values() {
            let Some(named_field_type) = Self::input_object_of(context, &field.ty) else {
                continue;
            };
            match field_nodes.get(field.name.as_str()) {
                Some(field_node) => {
                    self.detect_literal_default_value_cycle(context, named_field_type, &field_node.value)
                }
                None => self.detect_field_default_value_cycle(
                    context,
                    field,
                    named_field_type,
                    format!("{}.{}", input_obj.name, field.name),
                ),
            }
        }
    }

    fn detect_field_default_value_cycle(
        &mut self,
        context: &mut SchemaValidationContext<'_>,
        field: &super::definition::GraphQLInputField,
        field_type: &GraphQLInputObjectType,
        field_str: String,
    ) {
        let Some(default_input) = &field.default else {
            return;
        };
        if let Some(&cycle_index) = self.field_path_index.get(&field_str) {
            let via = if cycle_index < self.field_path.len() {
                format!(
                    " via the default values of: {}",
                    self.field_path[cycle_index..].join(", ")
                )
            } else {
                String::new()
            };
            context.report_error(format!(
                "Invalid circular reference. The default value of Input Object field {field_str} references itself{via}."
            ));
            return;
        }
        if !self.visited_fields.contains(&field_str) {
            self.visited_fields.insert(field_str.clone());
            self.field_path.push(field_str.clone());
            self.field_path_index.insert(field_str.clone(), self.field_path.len());
            match default_input {
                DefaultInput::Literal(literal) => self.detect_literal_default_value_cycle(context, field_type, literal),
                DefaultInput::Value(value) => self.detect_value_default_value_cycle(context, field_type, value),
            }
            self.field_path.pop();
            // `fieldPathIndex[fieldStr] = undefined`.
            self.field_path_index.shift_remove(&field_str);
        }
    }
}
