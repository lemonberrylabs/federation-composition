//! Port of `src/subgraph/validation/rules/elements/requires.ts`.

use graphql_js::{Directive, GraphQLError, NodeRef, Value, VisitAction, VisitInfo, Visitor};
use crate::graphql::printer::print;
use crate::subgraph::helpers::{
    get_fields_argument, parse_fields, validate_directive_against_original, visit_fields, InterceptDirectiveInfo,
    InterceptFieldInfo, Intercepts, VisitFieldsInterceptor,
};
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};

use super::{finish, is_interface_kind, is_object_kind, node_name};

/// `RequiresRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive(directiveNode)`. The `visitFields` interceptors become a local struct implementing `VisitFieldsInterceptor`.
/// Fields: the locals of the TS factory.
pub struct RequiresRules {}

impl<'a> SubgraphRule<'a> for RequiresRules {
    const NAME: &'static str = "RequiresRules";

    fn new(context: &mut SubgraphValidationContext<'a>) -> Self {
        RequiresRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for RequiresRules {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "requires", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// The `visitFields` interceptors of the `Directive` callback, with the locals they share.
struct RequiresFieldsInterceptor<'r> {
    field_coordinate: &'r str,
    printed_fields_value: &'r str,
    is_valid: bool,
}

impl<'a> VisitFieldsInterceptor<'a> for RequiresFieldsInterceptor<'_> {
    fn intercept_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        if is_object_kind(info.type_definition.kind) && info.field_name != "__typename" {
            context
                .state_builder
                .object_type()
                .field()
                .marked_as_required(&info.type_definition.name.value, &info.field_name)?;
        }
        Ok(())
    }

    fn intercept_unknown_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        self.is_valid = false;
        context.report_error(GraphQLError::with_code(
            format!(
                "On field \"{}\", for @requires(fields: {}): Cannot query field \"{}\" on type \"{}\" (if the field is defined in another subgraph, you need to add it to this subgraph with @external).",
                self.field_coordinate, self.printed_fields_value, info.field_name, info.type_definition.name.value
            ),
            "REQUIRES_INVALID_FIELDS",
        ));
        Ok(())
    }

    fn intercept_directive(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptDirectiveInfo) -> JsResult<()> {
        self.is_valid = false;
        if info.is_known {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{}\", for @requires(fields: {}): cannot have directive applications in the @requires(fields:) argument but found @{}.",
                    self.field_coordinate, self.printed_fields_value, info.directive_name
                ),
                "REQUIRES_DIRECTIVE_IN_FIELDS_ARG",
            ));
        } else {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{}\", for @requires(fields: {}): Unknown directive \"@{}\" in selection",
                    self.field_coordinate, self.printed_fields_value, info.directive_name
                ),
                "REQUIRES_INVALID_FIELDS",
            ));
        }
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
                    "On field \"{}\", for @requires(fields: {}): field \"{}.{}\" should not be part of a @requires since it is already provided by this subgraph (it is not marked @external)",
                    self.field_coordinate, self.printed_fields_value, info.type_definition.name.value, info.field_name
                ),
                "REQUIRES_FIELDS_MISSING_EXTERNAL",
            ));
        }
        Ok(())
    }
}

/// The `Directive(directiveNode)` callback.
fn directive<'a>(directive_node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("requires", &directive_node.name.value) {
        return Ok(());
    }

    let annotated_type = context.type_node_info.get_type_def();
    let annotated_field = context.type_node_info.get_field_def();

    // `@requires` needs to be used on a field of an object type or interface type
    let (Some(annotated_type), Some(annotated_field)) = (annotated_type, annotated_field) else {
        // Let regular validation handle this
        return Ok(());
    };

    let annotated_type_name = node_name(annotated_type);
    let annotated_field_name = node_name(annotated_field);
    let field_coordinate = format!("{annotated_type_name}.{annotated_field_name}");

    let used_on_interface = is_interface_kind(annotated_type.kind());

    if used_on_interface {
        context.report_error(GraphQLError::with_code(
            format!(
                "Cannot use @requires on field \"{field_coordinate}\" of parent type \"{annotated_type_name}\": @requires is not yet supported within interfaces"
            ),
            "REQUIRES_UNSUPPORTED_ON_INTERFACE",
        ));
        return Ok(());
    }

    let Some(fields_arg) = get_fields_argument(directive_node) else {
        return Ok(());
    };

    let printed_fields_value = print(&fields_arg.value);

    let fields_value = match &fields_arg.value {
        Value::String(value) => &value.value,
        Value::Enum(value) => &value.value,
        _ => {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{field_coordinate}\", for @requires(fields: {printed_fields_value}): Invalid value for argument \"fields\": must be a string."
                ),
                "REQUIRES_INVALID_FIELDS_TYPE",
            ));
            return Ok(());
        }
    };

    let selection_set = match parse_fields(fields_value) {
        Ok(selection_set) => selection_set,
        Err(JsError::GraphQL(error)) => {
            context.report_error(GraphQLError::with_code(
                format!(
                    "On field \"{field_coordinate}\", for @requires(fields: {printed_fields_value}): {}",
                    error.message
                ),
                "REQUIRES_INVALID_FIELDS",
            ));
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    let Some(selection_set) = selection_set else {
        // error already reported
        return Ok(());
    };

    if !is_interface_kind(annotated_type.kind()) && !is_object_kind(annotated_type.kind()) {
        return Ok(());
    }

    let Some(merged_type_def) = context
        .get_subgraph_object_or_interface_types()
        .get(annotated_type_name)
        .cloned()
    else {
        return Err(JsError::error(format!("Could not find type \"{annotated_type_name}\"")));
    };

    let mut interceptor = RequiresFieldsInterceptor {
        field_coordinate: &field_coordinate,
        printed_fields_value: &printed_fields_value,
        is_valid: true,
    };

    visit_fields(
        context,
        &selection_set,
        &merged_type_def,
        &mut interceptor,
        Intercepts {
            field: true,
            unknown_field: true,
            directive: true,
            non_external_field: true,
            ..Intercepts::default()
        },
    )?;

    if interceptor.is_valid {
        if used_on_interface {
            context
                .state_builder
                .interface_type()
                .field()
                .set_requires(annotated_type_name, annotated_field_name, fields_value)?;
            return Ok(());
        }

        context
            .state_builder
            .object_type()
            .field()
            .set_requires(annotated_type_name, annotated_field_name, fields_value)?;
    }

    Ok(())
}
