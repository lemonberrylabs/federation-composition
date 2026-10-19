//! Port of `src/subgraph/validation/rules/elements/provides.ts`.

use std::rc::Rc;

use graphql_js::{Directive, GraphQLError, Kind, NodeRef, Value, VisitAction, VisitInfo, Visitor};
use crate::graphql::printer::print;
use crate::subgraph::helpers::{
    get_fields_argument, named_type_from_type_node, parse_fields, validate_directive_against_original, visit_fields,
    InterceptDirectiveInfo, InterceptFieldInfo, InterceptMissingSelectionSetInfo, Intercepts, ObjectOrInterface,
    VisitFieldsInterceptor,
};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};

use super::{field_type, finish, is_interface_kind, is_object_kind, node_name};

/// `ProvidesRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(directiveNode)`. The `visitFields` interceptors become a local struct implementing `VisitFieldsInterceptor`.
/// Fields: the locals of the TS factory.
pub struct ProvidesRules {}

impl<'a> SubgraphRule<'a> for ProvidesRules {
    const NAME: &'static str = "ProvidesRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        ProvidesRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for ProvidesRules {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_js::VisitKinds {
        use graphql_js::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "provides", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The locals of the `Directive` callback that `processTargetType` closes over.
struct ProvidesDirective<'r, 'a> {
    directive_node: &'a Directive,
    annotated_type_name: &'r str,
    annotated_field_name: &'r str,
    /// `print(annotatedField.type)`.
    printed_annotated_field_type: &'r str,
    field_coordinate: &'r str,
    used_on_interface: bool,
}

/// The `Directive(directiveNode)` callback.
fn directive<'a>(directive_node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("provides", &directive_node.name.value) {
        return Ok(());
    }

    let annotated_type = context.type_node_info.get_type_def();
    let annotated_field = context.type_node_info.get_field_def();

    // `@provides` needs to be used on a field of an object type or interface type
    let (Some(annotated_type), Some(annotated_field)) = (annotated_type, annotated_field) else {
        // Let regular validation handle this
        return Ok(());
    };

    let annotated_type_name = node_name(annotated_type);
    let annotated_field_name = node_name(annotated_field);
    let field_coordinate = format!("{annotated_type_name}.{annotated_field_name}");

    let used_on_interface = is_interface_kind(annotated_type.kind());

    // NOTE(port): `getFieldDef()` is a `FieldDefinition` or an `InputValueDefinition`; both have
    // a `type`. Anything else would make the TS throw a TypeError on `annotatedField.type`.
    let Some(annotated_field_type) = field_type(annotated_field) else {
        return Err(JsError::type_error("Cannot read properties of undefined (reading 'kind')"));
    };

    let output_type = named_type_from_type_node(annotated_field_type);
    let target_type = context
        .get_subgraph_object_or_interface_types()
        .get(&output_type.name.value)
        .cloned();

    let printed_annotated_field_type = print(annotated_field_type);
    let input = ProvidesDirective {
        directive_node,
        annotated_type_name,
        annotated_field_name,
        printed_annotated_field_type: &printed_annotated_field_type,
        field_coordinate: &field_coordinate,
        used_on_interface,
    };

    if target_type.is_none() {
        let named_types = context.get_subgraph_union_types().get(&output_type.name.value).cloned();

        if let Some(named_types) = named_types {
            for named_type in &named_types {
                let target_type = context.get_subgraph_object_or_interface_types().get(named_type).cloned();
                process_target_type(context, &input, target_type)?;
            }
            return Ok(());
        }
    }
    process_target_type(context, &input, target_type)
}

/// `processTargetType(targetType)`.
fn process_target_type<'a>(
    context: &mut SubgraphValidationContext<'a>,
    input: &ProvidesDirective<'_, 'a>,
    target_type: Option<Rc<ObjectOrInterface>>,
) -> JsResult<()> {
    let field_coordinate = input.field_coordinate;

    let Some(target_type) = target_type else {
        context.report_error(GraphQLError::with_code(
            format!(
                "Invalid @provides directive on field \"{field_coordinate}\": field has type \"{}\" which is not a Composite Type",
                input.printed_annotated_field_type
            ),
            "PROVIDES_ON_NON_OBJECT_FIELD",
        ));
        return Ok(());
    };

    if input.used_on_interface {
        context.report_error(GraphQLError::with_code(
            format!(
                "Cannot use @provides on field \"{field_coordinate}\" of parent type \"{}\": @provides is not yet supported within interfaces",
                input.annotated_type_name
            ),
            "PROVIDES_UNSUPPORTED_ON_INTERFACE",
        ));
        return Ok(());
    }

    let Some(fields_arg) = get_fields_argument(input.directive_node) else {
        return Ok(());
    };

    let printed_fields_value = print(&fields_arg.value);

    let Value::String(fields_value) = &fields_arg.value else {
        context.report_error(GraphQLError::with_code(
            format!(
                "On field \"{field_coordinate}\", for @provides(fields: {printed_fields_value}): Invalid value for argument \"fields\": must be a string."
            ),
            "PROVIDES_INVALID_FIELDS_TYPE",
        ));
        return Ok(());
    };

    let selection_set = match parse_fields(&fields_value.value) {
        Ok(selection_set) => selection_set,
        Err(JsError::GraphQL(error)) => {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{field_coordinate}\", for @provides(fields: {printed_fields_value}): {}",
                    error.message
                ),
                "PROVIDES_INVALID_FIELDS",
            ));
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    let Some(selection_set) = selection_set else {
        // error already reported
        return Ok(());
    };

    let mut interceptor = ProvidesFieldsInterceptor {
        field_coordinate,
        printed_fields_value: &printed_fields_value,
        is_valid: true,
    };

    visit_fields(
        context,
        &selection_set,
        &target_type,
        &mut interceptor,
        Intercepts {
            field: true,
            field_with_missing_selection_set: true,
            unknown_field: true,
            directive: true,
            arguments: true,
            non_external_field: true,
            external_field: true,
            ..Intercepts::default()
        },
    )?;

    if interceptor.is_valid {
        context.state_builder.object_type().field().set_provides(
            input.annotated_type_name,
            input.annotated_field_name,
            &fields_value.value,
        )?;
    }

    Ok(())
}

/// The `visitFields` interceptors of `processTargetType`, with the locals they share.
struct ProvidesFieldsInterceptor<'r> {
    field_coordinate: &'r str,
    printed_fields_value: &'r str,
    is_valid: bool,
}

impl<'a> VisitFieldsInterceptor<'a> for ProvidesFieldsInterceptor<'_> {
    fn intercept_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        if is_object_kind(info.type_definition.kind) && info.field_name != "__typename" {
            context
                .state_builder
                .object_type()
                .field()
                .mark_as_provided(&info.type_definition.name.value, &info.field_name)?;
        }
        Ok(())
    }

    fn intercept_field_with_missing_selection_set(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptMissingSelectionSetInfo,
    ) -> JsResult<()> {
        self.is_valid = false;
        context.report_error(GraphQLError::with_code(
            format!(
                "On field \"{}\", for @provides(fields: {}): Invalid empty selection set for field \"{}.{}\" of non-leaf type {}",
                self.field_coordinate,
                self.printed_fields_value,
                info.type_definition.name.value,
                info.field_name,
                info.output_type
            ),
            "PROVIDES_INVALID_FIELDS",
        ));
        Ok(())
    }

    fn intercept_unknown_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        self.is_valid = false;
        context.report_error(GraphQLError::with_code(
            format!(
                "On field \"{}\", for @provides(fields: {}): Cannot query field \"{}\" on type \"{}\" (if the field is defined in another subgraph, you need to add it to this subgraph with @external).",
                self.field_coordinate, self.printed_fields_value, info.field_name, info.type_definition.name.value
            ),
            "PROVIDES_INVALID_FIELDS",
        ));
        Ok(())
    }

    fn intercept_directive(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptDirectiveInfo) -> JsResult<()> {
        self.is_valid = false;
        if info.is_known {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{}\", for @provides(fields: {}): cannot have directive applications in the @provides(fields:) argument but found @{}.",
                    self.field_coordinate, self.printed_fields_value, info.directive_name
                ),
                "PROVIDES_DIRECTIVE_IN_FIELDS_ARG",
            ));
        } else {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{}\", for @provides(fields: {}): Unknown directive \"@{}\" in selection",
                    self.field_coordinate, self.printed_fields_value, info.directive_name
                ),
                "PROVIDES_INVALID_FIELDS",
            ));
        }
        Ok(())
    }

    fn intercept_arguments(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        self.is_valid = false;
        context.report_error(GraphQLError::with_code(
            format!(
                "On field \"{}\", for @provides(fields: {}): field {}.{} cannot be included because it has arguments (fields with argument are not allowed in @provides)",
                self.field_coordinate, self.printed_fields_value, info.type_definition.name.value, info.field_name
            ),
            "PROVIDES_FIELDS_HAS_ARGS",
        ));
        Ok(())
    }

    fn intercept_non_external_field(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        if context.satisfies_version_range("> v1.0") {
            self.is_valid = false;
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{}\", for @provides(fields: {}): field \"{}.{}\" should not be part of a @provides since it is already provided by this subgraph (it is not marked @external)",
                    self.field_coordinate, self.printed_fields_value, info.type_definition.name.value, info.field_name
                ),
                "PROVIDES_FIELDS_MISSING_EXTERNAL",
            ));
        }
        Ok(())
    }

    fn intercept_external_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        // Oh, it hurts performance. We will fix it later. First, we need to make sure it works.
        let key_directives: Vec<&Directive> = info
            .type_definition
            .directives
            .iter()
            .filter(|directive| context.is_available_federation_directive("key", &directive.name.value))
            .collect();

        // If a field is part of the @key but also part of the @provides
        // then it should not be marked @external (because it is already provided by this subgraph - historical/backward compatibility reasons...)

        if key_directives.is_empty() {
            return Ok(());
        }

        let mut key_field_interceptor = KeyFieldOfExtensionInterceptor {
            type_name: &info.type_definition.name.value,
            field_name: &info.field_name,
            intercepted_field_is_primary_key_from_extension: false,
        };

        for key_directive in key_directives {
            if key_field_interceptor.intercepted_field_is_primary_key_from_extension {
                break;
            }

            // `arguments.find((arg) => arg.name.value === "fields" && arg.value.kind === Kind.STRING)`,
            // keeping the string value.
            let fields_value = key_directive.arguments.iter().find_map(|arg| match &arg.value {
                Value::String(value) if arg.name.value == "fields" => Some(value),
                _ => None,
            });

            if let Some(fields_value) = fields_value {
                let key_fields = parse_fields(&fields_value.value)?;
                let Some(merged_type_def) = context
                    .get_subgraph_object_or_interface_types()
                    .get(&info.type_definition.name.value)
                    .cloned()
                else {
                    return Err(JsError::error(format!(
                        "Could not find type \"{}\"",
                        info.type_definition.name.value
                    )));
                };

                if let Some(key_fields) = key_fields {
                    visit_fields(
                        context,
                        &key_fields,
                        &merged_type_def,
                        &mut key_field_interceptor,
                        Intercepts {
                            field: true,
                            ..Intercepts::default()
                        },
                    )?;
                }
            }
        }

        if context.satisfies_version_range(">= v2.0")
            && key_field_interceptor.intercepted_field_is_primary_key_from_extension
        {
            self.is_valid = false;
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{}\", for @provides(fields: {}): field \"{}.{}\" should not be part of a @provides since it is already \"effectively\" provided by this subgraph (while it is marked @external, it is a @key field of an extension type, which are not internally considered external for historical/backward compatibility reasons)",
                    self.field_coordinate, self.printed_fields_value, info.type_definition.name.value, info.field_name
                ),
                "PROVIDES_FIELDS_MISSING_EXTERNAL",
            ));
        }

        Ok(())
    }
}

/// The `interceptField` of the nested `visitFields` in `interceptExternalField`, with the
/// outer `info` and `interceptedFieldIsPrimaryKeyFromExtension`.
struct KeyFieldOfExtensionInterceptor<'r> {
    type_name: &'r str,
    field_name: &'r str,
    intercepted_field_is_primary_key_from_extension: bool,
}

impl<'a> VisitFieldsInterceptor<'a> for KeyFieldOfExtensionInterceptor<'_> {
    fn intercept_field(&mut self, context: &mut SubgraphValidationContext<'a>, key_field_info: &InterceptFieldInfo) -> JsResult<()> {
        if key_field_info.type_definition.name.value == self.type_name && key_field_info.field_name == self.field_name {
            let is_interface_type = is_interface_kind(key_field_info.type_definition.kind);

            if is_interface_type {
                // If it's an interface, we shouldn't report it as an error
                return Ok(());
            }

            let is_extension =
                // Check if it's type extension
                matches!(
                    key_field_info.type_definition.kind,
                    Kind::ObjectTypeExtension | Kind::InterfaceTypeExtension
                ) ||
                // Check if type is marked with @extends
                key_field_info
                    .type_definition
                    .directives
                    .iter()
                    .any(|directive| context.is_available_federation_directive("extends", &directive.name.value));

            // if it is an extension, we should report it as an error (historical thingy)
            if is_extension {
                self.intercepted_field_is_primary_key_from_extension = true;
            }
        }
        Ok(())
    }
}
