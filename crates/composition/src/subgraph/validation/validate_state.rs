//! Port of `src/subgraph/validation/validate-state.ts`: graphql-js-like schema validation run on
//! the built `SubgraphState` (root types, directives, types, fields, interfaces, ...).

use std::sync::LazyLock;

use graphql_js::{GraphQLError, Value};
use crate::utils::collections::{IndexMap, IndexSet};

use super::validation_context::SubgraphValidationContext;
use crate::subgraph::state::{
    Argument, EnumType, Field, InputField, InputObjectType, InterfaceType, SubgraphState, SubgraphType,
    TypeKind, UnionType,
};
use crate::utils::format::and_list;
use crate::utils::state::{is_list, is_non_null, strip_list, strip_non_null, strip_type_modifiers};

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

impl TypeCheck {
    fn from_bool(b: bool) -> TypeCheck {
        if b {
            TypeCheck::Yes
        } else {
            TypeCheck::No
        }
    }
}

/// `validateSubgraphState(state, context)`: errors with `extensions.code` `INVALID_GRAPHQL`.
/// The context is only read (`isLinkSpecDirective`, `isLinkSpecType`); the caller passes the
/// state through `&context.state_builder.state`.
pub fn validate_subgraph_state(state: &SubgraphState, context: &SubgraphValidationContext<'_>) -> Vec<GraphQLError> {
    let mut errors: Vec<GraphQLError> = Vec::new();

    let mut report_error = |message: String| {
        errors.push(GraphQLError::with_code(message, "INVALID_GRAPHQL"));
    };

    validate_root_types(state, &mut report_error);
    validate_directives(state, &mut report_error, context);
    validate_types(state, &mut report_error);

    errors
}

fn validate_root_types(state: &SubgraphState, report_error: ReportErrorFn<'_>) {
    let mut root_types_map: IndexMap<String, IndexSet<&'static str>> = IndexMap::default();

    for (root_type_kind, root_type_name) in state.schema.iter() {
        let root_type_kind = root_type_kind.as_str();

        if !root_type_name.is_empty() {
            let Some(root_type) = state.types.get(root_type_name) else {
                // The existence of the type was already validated.
                // If it doesn't exist, it means it's been reported by KnownTypeNamesRule.
                continue;
            };

            if !is_object_type(root_type) {
                let operation_type_str = capitalize(&root_type_kind.replacen("Type", "", 1));
                report_error(if root_type_kind == "queryType" {
                    format!("{operation_type_str} root type must be Object type, it cannot be {root_type_name}.")
                } else {
                    format!(
                        "{operation_type_str} root type must be Object type if provided, it cannot be {root_type_name}."
                    )
                });
            } else if let Some(existing) = root_types_map.get_mut(root_type_name) {
                existing.insert(root_type_kind);
            } else {
                root_types_map.insert(root_type_name.to_string(), IndexSet::from_iter([root_type_kind]));
            }
        }
    }

    for (root_type_name, operation_types) in &root_types_map {
        if operation_types.len() > 1 {
            let operation_list = and_list(
                &operation_types
                    .iter()
                    .map(|op| capitalize(&op.replacen("Type", "", 1)))
                    .collect::<Vec<_>>(),
                true,
                None,
            );
            report_error(format!(
                "All root types must be different, \"{root_type_name}\" type is used as {operation_list} root types."
            ));
        }
    }
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
    for directive in state.types.values() {
        let SubgraphType::Directive(directive) = directive else {
            continue;
        };

        if context.is_link_spec_directive(&directive.name) {
            continue;
        }

        // Ensure they are named correctly.
        validate_name(report_error, &directive.name);

        // Ensure the arguments are valid.
        for (arg_name, arg) in &directive.args {
            // Ensure they are named correctly.
            validate_name(report_error, arg_name);

            // Ensure the type is an input type.
            let arg_input_type_name = strip_type_modifiers(&arg.r#type);

            if context.is_link_spec_type(&arg_input_type_name) {
                continue;
            }

            let is_input = is_input_type(state, &arg_input_type_name);

            if is_input == TypeCheck::Skip {
                continue;
            }

            if is_input == TypeCheck::No {
                report_error(format!(
                    "The type of @{}({}:) must be Input Type but got: {}.",
                    directive.name, arg.name, arg.r#type
                ));
            }

            if is_required_argument(arg) && arg.deprecated.is_some() {
                report_error(format!(
                    "Required argument @{}({}:) cannot be deprecated.",
                    directive.name, arg.name
                ));
            }
        }
    }
}

fn validate_types(state: &SubgraphState, report_error: ReportErrorFn<'_>) {
    let mut validate_input_object_circular_refs = create_input_object_circular_refs_validator(state);
    let mut implementations_map: IndexMap<String, IndexSet<String>> = IndexMap::default();

    for ty in state.types.values() {
        // Store implementations by interface (for interfaces) and by objects (for objects).
        let (type_name, interfaces) = match ty {
            SubgraphType::Interface(t) => (&t.name, &t.interfaces),
            SubgraphType::Object(t) => (&t.name, &t.interfaces),
            _ => continue,
        };
        for iface in interfaces {
            if let Some(interface_type) = state.types.get(iface) {
                if is_interface_type(interface_type) {
                    match implementations_map.get_mut(iface) {
                        None => {
                            implementations_map.insert(iface.clone(), IndexSet::from_iter([type_name.clone()]));
                        }
                        Some(implementations) => {
                            implementations.insert(type_name.clone());
                        }
                    }
                }
            }
        }
    }

    for ty in state.types.values() {
        // Ensure it is named correctly (excluding introspection types).
        if !is_introspection_type(ty.name()) {
            validate_name(report_error, ty.name());
        }

        match ty {
            SubgraphType::Object(_) => {
                // Ensure fields are valid
                validate_fields(state, report_error, ty);

                // Ensure objects implement the interfaces they claim to.
                validate_interfaces(state, &mut implementations_map, report_error, ty);
            }
            SubgraphType::Interface(_) => {
                // Ensure fields are valid.
                validate_fields(state, report_error, ty);

                // Ensure interfaces implement the interfaces they claim to.
                validate_interfaces(state, &mut implementations_map, report_error, ty);
            }
            SubgraphType::Union(union) => {
                // Ensure Unions include valid member types.
                validate_union_members(state, report_error, union);
            }
            SubgraphType::Enum(enum_type) => {
                // Ensure Enums have valid values.
                validate_enum_values(report_error, enum_type);
            }
            SubgraphType::InputObject(input_obj) => {
                // Ensure Input Object fields are valid.
                validate_input_fields(state, report_error, input_obj);

                // Ensure Input Objects do not contain non-nullable circular references
                validate_input_object_circular_refs.detect_cycle_recursive(input_obj, report_error);
            }
            _ => {}
        }
    }
}

fn validate_name(report_error: ReportErrorFn<'_>, name: &str) {
    // Ensure names are valid, however introspection types opt out.
    if name.starts_with("__") {
        report_error(format!(
            "Name \"{name}\" must not begin with \"__\", which is reserved by GraphQL introspection."
        ));
    }
}

/// `type.name`, `type.fields` and `type.interfaces` of an object or interface type.
fn object_or_interface_parts(ty: &SubgraphType) -> (&str, &IndexMap<String, Field>, &IndexSet<String>) {
    match ty {
        SubgraphType::Object(t) => (&t.name, &t.fields, &t.interfaces),
        SubgraphType::Interface(t) => (&t.name, &t.fields, &t.interfaces),
        _ => unreachable!("an object or interface type"),
    }
}

/// `validateFields(state, reportError, type)` for an object or interface type.
fn validate_fields(state: &SubgraphState, report_error: ReportErrorFn<'_>, ty: &SubgraphType) {
    let (type_name, fields, _) = object_or_interface_parts(ty);
    let is_root_type = Some(type_name) == state.schema.query_type()
        || Some(type_name) == state.schema.mutation_type()
        || Some(type_name) == state.schema.subscription_type();

    // Objects and Interfaces both must define one or more fields.
    if fields.is_empty() && !is_root_type {
        report_error(format!("Type {type_name} must define one or more fields."));
    }

    for field in fields.values() {
        // Ensure they are named correctly.
        validate_name(report_error, &field.name);

        let field_type_name = strip_type_modifiers(&field.r#type);
        let field_type_exists = type_exists(state, &field_type_name);

        if !field_type_exists {
            // The existence of the type was already validated.
            // If it doesn't exist, it means it's been reported by KnownTypeNamesRule.
            continue;
        }

        // Ensure the type is an output type
        let is_output = is_output_type(state, &field_type_name);

        if is_output == TypeCheck::Skip {
            continue;
        }

        if is_output == TypeCheck::No {
            report_error(format!(
                "The type of \"{type_name}.{}\" must be Output Type but got: \"{}\".",
                field.name, field.r#type
            ));
        }

        // Ensure the arguments are valid
        for arg in field.args.values() {
            let arg_name = &arg.name;

            // Ensure they are named correctly.
            validate_name(report_error, arg_name);

            let arg_type_name = strip_type_modifiers(&arg.r#type);
            let arg_type_exists = type_exists(state, &arg_type_name);

            if !arg_type_exists {
                // The existence of the type was already validated.
                // If it doesn't exist, it means it's been reported by KnownTypeNamesRule.
                continue;
            }

            // Ensure the type is an input type
            let is_input = is_input_type(state, &arg_type_name);

            if is_input == TypeCheck::Skip {
                continue;
            }

            if is_input == TypeCheck::No {
                let is_list = arg.r#type.ends_with(']');
                let is_non_null = arg.r#type.ends_with('!');
                let extra = if is_list {
                    ", a ListType"
                } else if is_non_null {
                    ", a NonNullType"
                } else {
                    ""
                };

                report_error(format!(
                    "The type of \"{type_name}.{}({arg_name}:)\" must be Input Type but got \"{}\"{extra}.",
                    field.name, arg.r#type
                ));
            }

            if is_required_argument(arg) && arg.deprecated.is_some() {
                report_error(format!(
                    "Required argument {type_name}.{}({arg_name}:) cannot be deprecated.",
                    field.name
                ));
            }

            // Ensure default value is valid
            if let Some(default_value) = &arg.default_value {
                let value = parse_default_value(default_value);
                if !is_validate_default_value(state, report_error, &arg.r#type, &value) {
                    report_error(format!(
                        "Invalid default value (got: {default_value}) provided for argument {type_name}.{}({}:) of type {}.",
                        field.name, arg.name, arg.r#type
                    ));
                }
            }
        }
    }
}

/// `parseValue(defaultValue)`. The default values in the state are printed AST values, which
/// always parse back.
fn parse_default_value(default_value: &str) -> Value {
    graphql_js::parse_value(default_value).expect("printed default values parse")
}

fn is_validate_default_value(
    state: &SubgraphState,
    report_error: ReportErrorFn<'_>,
    input_type_printed: &str,
    value: &Value,
) -> bool {
    if is_non_null(input_type_printed) {
        if matches!(value, Value::Null(_)) {
            return false;
        }

        return is_validate_default_value(state, report_error, &strip_non_null(input_type_printed), value);
    }

    if matches!(value, Value::Null(_)) {
        // At this point, NULL is acceptable for all types.
        return true;
    }

    let input_type_name = strip_type_modifiers(input_type_printed);
    let input_type = state.types.get(&input_type_name);

    if let Some(input_type) = input_type {
        if is_scalar_type(input_type) {
            return true;
        }
    }

    if is_list(input_type_printed) {
        if let Value::List(list) = value {
            return list
                .values
                .iter()
                .all(|val| is_validate_default_value(state, report_error, &strip_list(input_type_printed), val));
        }

        return is_validate_default_value(state, report_error, &strip_list(input_type_printed), value);
    }

    if SPECIFIED_SCALARS.contains(&input_type_name) {
        let specified_scalar = graphql_js::specified::specified_scalar_type(&input_type_name)
            .expect("specifiedScalarTypes contains the name");

        // graphql-js 17 keeps legacy coercion in parseLiteral; its strict replacement
        // is unavailable in graphql-js 16, which we continue to support.
        // NOTE(port): the `graphql_js` crate is graphql-js 17, so this is the
        // `versionInfo.major >= 17` branch: `coerceInputLiteral`, which throws on invalid input.
        if matches!(value, Value::Variable(_)) {
            return false;
        }
        return specified_scalar.coerce_input_literal(value).is_ok();
    }

    let Some(input_type) = input_type else {
        return true; // The existence of the type was already validated.
    };

    if let SubgraphType::InputObject(input_type) = input_type {
        let Value::Object(object) = value else {
            return false;
        };

        let fields = &input_type.fields;

        for ast_field in &object.fields {
            let Some(field) = fields.get(&ast_field.name.value) else {
                return false;
            };

            if !is_validate_default_value(state, report_error, &field.r#type, &ast_field.value) {
                return false;
            }
        }

        return true;
    }

    if let SubgraphType::Enum(input_type) = input_type {
        let enum_value = match value {
            Value::Enum(v) => &v.value,
            Value::String(v) => &v.value,
            _ => return false,
        };

        return input_type.values.contains_key(enum_value);
    }

    false
}

fn validate_union_members(state: &SubgraphState, report_error: ReportErrorFn<'_>, union: &UnionType) {
    let member_types = &union.members;

    if member_types.is_empty() {
        report_error(format!("Union type {} must define one or more member types.", union.name));
    }

    // NOTE(port): `members` is a set, so the "only once" branch never fires; kept as in TS.
    let mut included_type_names: IndexSet<&str> = IndexSet::default();
    for member_type in member_types {
        if included_type_names.contains(member_type.as_str()) {
            report_error(format!(
                "Union type {} can only include type {member_type} once.",
                union.name
            ));
            continue;
        }
        included_type_names.insert(member_type);
        let ty = state.types.get(member_type);

        if !ty.is_some_and(is_object_type) {
            report_error(format!(
                "Union type {} can only include Object types, it cannot include {member_type}.",
                union.name
            ));
        }
    }
}

fn validate_enum_values(report_error: ReportErrorFn<'_>, enum_type: &EnumType) {
    let enum_values = &enum_type.values;

    if enum_values.is_empty() {
        report_error(format!("Enum type {} must define one or more values.", enum_type.name));
    }

    for enum_value in enum_values.keys() {
        // Ensure valid name.
        validate_name(report_error, enum_value);
    }
}

fn validate_input_fields(state: &SubgraphState, report_error: ReportErrorFn<'_>, input_obj: &InputObjectType) {
    let fields = &input_obj.fields;

    if fields.is_empty() {
        report_error(format!("Input Object type {} must define one or more fields.", input_obj.name));
    }

    // Ensure the arguments are valid
    for field in fields.values() {
        // Ensure they are named correctly.
        validate_name(report_error, &field.name);

        let field_type_name = strip_type_modifiers(&field.r#type);
        let field_type_exists = type_exists(state, &field_type_name);

        if !field_type_exists {
            // The existence of the type was already validated.
            // If it doesn't exist, it means it's been reported by KnownTypeNamesRule.
            continue;
        }

        // Ensure the type is an input type
        let is_input = is_input_type(state, &field_type_name);

        if is_input == TypeCheck::Skip {
            continue;
        }

        if is_input == TypeCheck::No {
            let is_list = field.r#type.ends_with(']');
            let is_non_null = field.r#type.ends_with('!');
            let extra = if is_list {
                ", a ListType"
            } else if is_non_null {
                ", a NonNullType"
            } else {
                ""
            };
            report_error(format!(
                "The type of {}.{} must be Input Type but got \"{}\"{extra}.",
                input_obj.name, field.name, field.r#type
            ));
        }

        if is_required_input_field(field) && field.deprecated.is_some() {
            report_error(format!(
                "Required input field {}.{} cannot be deprecated.",
                input_obj.name, field.name
            ));
        }

        // Ensure default value is valid
        if let Some(default_value) = &field.default_value {
            let value = parse_default_value(default_value);
            if !is_validate_default_value(state, report_error, &field.r#type, &value) {
                report_error(format!(
                    "Invalid default value (got: {default_value}) provided for input field {}.{} of type {}.",
                    input_obj.name, field.name, field.r#type
                ));
            }
        }
    }
}

/// `validateInterfaces(state, implementationsMap, reportError, type)` for an object or
/// interface type.
fn validate_interfaces(
    state: &SubgraphState,
    implementations_map: &mut IndexMap<String, IndexSet<String>>,
    report_error: ReportErrorFn<'_>,
    ty: &SubgraphType,
) {
    let (type_name, _, interfaces) = object_or_interface_parts(ty);
    let mut iface_type_names: IndexSet<&str> = IndexSet::default();
    for iface in interfaces {
        let Some(interface_type) = state.types.get(iface) else {
            // The existence of the type was already validated.
            // If it doesn't exist, it means it's been reported by KnownTypeNamesRule.
            continue;
        };

        let SubgraphType::Interface(interface_type) = interface_type else {
            report_error(format!(
                "Type {type_name} must only implement Interface types, it cannot implement {iface}."
            ));
            continue;
        };

        if type_name == iface {
            report_error(format!(
                "Type {type_name} cannot implement itself because it would create a circular reference."
            ));
            continue;
        }

        // NOTE(port): `interfaces` is a set, so the "only once" branch never fires; kept as in TS.
        if iface_type_names.contains(iface.as_str()) {
            report_error(format!("Type {type_name} can only implement {iface} once."));
            continue;
        }

        iface_type_names.insert(iface);

        validate_type_implements_ancestors(report_error, ty, interface_type);
        validate_type_implements_interface(state, implementations_map, report_error, ty, interface_type);
    }
}

fn validate_type_implements_ancestors(report_error: ReportErrorFn<'_>, ty: &SubgraphType, interface_type: &InterfaceType) {
    let (type_name, _, iface_interfaces) = object_or_interface_parts(ty);
    for transitive in &interface_type.interfaces {
        if !iface_interfaces.contains(transitive) {
            report_error(if transitive == type_name {
                format!(
                    "Type {type_name} cannot implement {} because it would create a circular reference.",
                    interface_type.name
                )
            } else {
                format!(
                    "Type {type_name} must implement {transitive} because it is implemented by {}.",
                    interface_type.name
                )
            });
        }
    }
}

fn validate_type_implements_interface(
    state: &SubgraphState,
    implementations_map: &mut IndexMap<String, IndexSet<String>>,
    report_error: ReportErrorFn<'_>,
    ty: &SubgraphType,
    interface_type: &InterfaceType,
) {
    let (type_name, type_field_map, _) = object_or_interface_parts(ty);

    // Assert each interface field is implemented.
    for iface_field in interface_type.fields.values() {
        let field_name = &iface_field.name;

        // Assert interface field exists on type.
        let Some(type_field) = type_field_map.get(field_name) else {
            report_error(format!(
                "Interface field {}.{field_name} expected but {type_name} does not provide it.",
                interface_type.name
            ));
            continue;
        };

        // Assert interface field type is satisfied by type field type, by being
        // a valid subtype. (covariant)
        if !is_type_sub_type_of(state, implementations_map, &type_field.r#type, &iface_field.r#type) {
            report_error(format!(
                "Interface field {}.{field_name} expects type {} but {type_name}.{field_name} of type {} is not a proper subtype.",
                interface_type.name, iface_field.r#type, type_field.r#type
            ));
        }

        // Assert each interface field arg is implemented.
        for iface_arg in iface_field.args.values() {
            let arg_name = &iface_arg.name;

            // Assert interface field arg exists on object field.
            let Some(type_arg) = type_field.args.get(arg_name) else {
                report_error(format!(
                    "Interface field argument {}.{field_name}({arg_name}:) expected but {type_name}.{field_name} does not provide it.",
                    interface_type.name
                ));
                continue;
            };

            // Assert interface field arg type matches object field arg type.
            if iface_arg.r#type != type_arg.r#type {
                report_error(format!(
                    "Interface field argument {}.{field_name}({arg_name}:) expects type {} but {type_name}.{field_name}({arg_name}:) is type {}.",
                    interface_type.name, iface_arg.r#type, type_arg.r#type
                ));
            }
        }

        // Assert additional arguments must not be required.
        for type_arg in type_field.args.values() {
            let arg_name = &type_arg.name;
            let iface_arg = iface_field.args.get(arg_name);
            if iface_arg.is_none() && is_required_argument(type_arg) {
                report_error(format!(
                    "Object field {type_name}.{field_name} includes required argument {arg_name} that is missing from the Interface field {}.{field_name}.",
                    interface_type.name
                ));
            }
        }
    }
}

/// `isTypeSubTypeOf(state, implementationsMap, maybeSubTypeName, superTypeName)`.
/// Provided a type and a super type, return true if the first type is either
/// equal or a subset of the second super type (covariant).
pub fn is_type_sub_type_of(
    state: &SubgraphState,
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

    let super_type = state.types.get(super_type_name);
    let maybe_sub_type = state.types.get(maybe_sub_type_name);

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

/// `isSubType(implementationsMap, abstractType, maybeSubType)`: `abstract_type` is an interface
/// or union, `maybe_sub_type` an object or interface.
fn is_sub_type(
    implementations_map: &IndexMap<String, IndexSet<String>>,
    abstract_type: &SubgraphType,
    maybe_sub_type: &SubgraphType,
) -> bool {
    if let SubgraphType::Union(abstract_type) = abstract_type {
        // set = new Set<GraphQLObjectType>(abstractType.getTypes());
        return abstract_type.members.contains(maybe_sub_type.name());
    }

    implementations_map
        .get(abstract_type.name())
        .is_some_and(|set| set.contains(maybe_sub_type.name()))
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
        visited_types: IndexSet::default(),
        field_path: Vec::new(),
        field_path_index_by_type_name: IndexMap::default(),
    }
}

impl InputObjectCircularRefsValidator<'_> {
    // This does a straight-forward DFS to find cycles.
    // It does not terminate when a cycle was found but continues to explore
    // the graph to find all possible cycles.
    fn detect_cycle_recursive(&mut self, input_obj: &InputObjectType, report_error: ReportErrorFn<'_>) {
        if self.visited_types.contains(&input_obj.name) {
            return;
        }

        self.visited_types.insert(input_obj.name.clone());
        self.field_path_index_by_type_name
            .insert(input_obj.name.clone(), self.field_path.len());

        let state = self.state;
        for field in input_obj.fields.values() {
            if is_non_null(&field.r#type) {
                if let Some(SubgraphType::InputObject(field_type)) = state.types.get(&strip_non_null(&field.r#type)) {
                    let cycle_index = self.field_path_index_by_type_name.get(&field_type.name).copied();

                    self.field_path.push(field.name.clone());
                    match cycle_index {
                        None => self.detect_cycle_recursive(field_type, report_error),
                        Some(cycle_index) => {
                            let cycle_path = &self.field_path[cycle_index..];
                            report_error(format!(
                                "Cannot reference Input Object \"{}\" within itself through a series of non-null fields: \"{}\".",
                                field_type.name,
                                cycle_path.join(".")
                            ));
                        }
                    }
                    self.field_path.pop();
                }
            }
        }

        self.field_path_index_by_type_name.shift_remove(&input_obj.name);
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
    let Some(ty) = state.types.get(type_name) else {
        if SPECIFIED_SCALARS.contains(type_name) {
            return TypeCheck::Yes;
        }

        // The existence of the type was already validated.
        // See: KnownTypeNamesRule
        // No need to throw an exception here, because it affects the rest of the validation logic.
        return TypeCheck::Skip;
    };

    // Only input types are not output types (scalars and enums are in both).
    // This is a quick check, to make it least expensive possible.
    TypeCheck::from_bool(!is_input_object_type(ty))
}

/// `isInputType(state, typeName)`.
pub fn is_input_type(state: &SubgraphState, type_name: &str) -> TypeCheck {
    let Some(ty) = state.types.get(type_name) else {
        if SPECIFIED_SCALARS.contains(type_name) {
            return TypeCheck::Yes;
        }

        // The existence of the type was already validated.
        // See: KnownTypeNamesRule
        // No need to throw an exception here, because it affects the rest of the validation logic.
        return TypeCheck::Skip;
    };

    TypeCheck::from_bool(is_scalar_type(ty) || is_enum_type(ty) || is_input_object_type(ty))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_lowercases_the_rest() {
        assert_eq!(capitalize("query"), "Query");
        assert_eq!(capitalize("subscription"), "Subscription");
        assert_eq!(capitalize("mUTATION"), "Mutation");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn root_type_kind_names_capitalize_like_ts() {
        use crate::subgraph::state::RootTypeKind;
        let names: Vec<String> = [RootTypeKind::QueryType, RootTypeKind::MutationType, RootTypeKind::SubscriptionType]
            .iter()
            .map(|k| capitalize(&k.as_str().replacen("Type", "", 1)))
            .collect();
        assert_eq!(names, vec!["Query", "Mutation", "Subscription"]);
        assert_eq!(and_list(&names, true, None), "Query, Mutation, and Subscription");
    }
}
