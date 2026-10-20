//! Port of `src/subgraph/helpers.ts`.

use std::rc::Rc;

use crate::utils::collections::IndexMap;
use graphql_rs::{
    Argument, Definition, Directive as DirectiveNode, DirectiveDefinition, FieldDefinition, GraphQLError,
    InputValueDefinition, Kind, Name, NamedType, Selection, SelectionSet, StringValue, Type, Value,
};

use crate::graphql::printer::print;
use crate::subgraph::validation::validation_context::{MarkAsUsedReason, SubgraphValidationContext};
use crate::utils::js::{JsError, JsResult};

/// `validateDirectiveAgainstOriginal(providedDirectiveNode, directiveName, context)`: checks a
/// subgraph's own definition of a federation directive against the spec; reports errors or
/// marks it as a replacement.
pub fn validate_directive_against_original(
    provided_directive_node: &DirectiveDefinition,
    directive_name: &str,
    context: &mut SubgraphValidationContext<'_>,
) -> JsResult<()> {
    if !context.is_available_federation_directive(directive_name, &provided_directive_node.name.value) {
        return Ok(());
    }

    let is_federation_v2 = context.satisfies_version_range(">= v2.0");

    let federation_directive = context
        .get_known_federation_directives()
        .iter()
        .find(|d| context.is_available_federation_directive(directive_name, &d.name.value))
        .cloned();

    let Some(federation_directive) = federation_directive else {
        return Err(JsError::error(format!(
            "Federation directive @{directive_name} not found"
        )));
    };

    let mut errors: Vec<GraphQLError> = Vec::new();

    // `original` / `provided`: `{ args: Map<name, arg>, locations }`.
    let original_args: IndexMap<&str, &InputValueDefinition> = federation_directive
        .arguments
        .iter()
        .map(|arg| (arg.name.value.as_str(), arg))
        .collect();
    let original_locations: Vec<&str> = federation_directive
        .locations
        .iter()
        .map(|loc| loc.value.as_str())
        .collect();
    let provided_args: IndexMap<&str, &InputValueDefinition> = provided_directive_node
        .arguments
        .iter()
        .map(|arg| (arg.name.value.as_str(), arg))
        .collect();
    let provided_locations: Vec<&str> = provided_directive_node
        .locations
        .iter()
        .map(|loc| loc.value.as_str())
        .collect();

    for (arg_name, arg_def) in original_args.iter() {
        let provided_arg_node = provided_args.get(arg_name).copied();

        // Compares required arguments
        if is_non_null_type_node(&arg_def.ty) && provided_arg_node.is_none() {
            errors.push(GraphQLError::with_code(
                format!(
                    "Invalid definition for directive \"@{directive_name}\": missing required argument \"{arg_name}\""
                ),
                "DIRECTIVE_DEFINITION_INVALID",
            ));
        }

        if let Some(provided_arg_node) = provided_arg_node {
            let expected_type = print(&arg_def.ty);
            let provided_type = print(&provided_arg_node.ty);

            // TODO: let's clean it up, it's such a mess
            if expected_type != provided_type {
                let is_non_nullable_string = provided_type == "String!";
                let allowed_field_set_types: &[&str] = if is_federation_v2 {
                    &["FieldSet!", "federation__FieldSet!", "_FieldSet!", "[String!]!"]
                } else {
                    &["_FieldSet!", "String", "String!"]
                };
                let field_set_types_in_spec: &[&str] = if is_federation_v2 {
                    &["FieldSet!", "federation__FieldSet!", "_FieldSet!", "[String!]!"]
                } else {
                    &["_FieldSet!", "FieldSet!", "String"]
                };
                let expects_field_set = field_set_types_in_spec.contains(&expected_type.as_str());

                if !is_non_nullable_string && expects_field_set {
                    // [T!]! should be accepted as well
                    let is_one_of_expected = allowed_field_set_types
                        .iter()
                        .map(|f| f.to_string())
                        .chain(allowed_field_set_types.iter().map(|f| format!("[{f}]!")))
                        .any(|f| f == provided_type);

                    if !is_one_of_expected {
                        errors.push(GraphQLError::with_code(
                            // TODO: it should be `federation__FieldSet!` or `FieldSet!`, but it depends on the import
                            format!(
                                "Invalid definition for directive \"@{directive_name}\": argument \"{arg_name}\" should have type \"{expected_type}\" but found type \"{provided_type}\""
                            ),
                            "DIRECTIVE_DEFINITION_INVALID",
                        ));
                    }
                }
            }

            // Compares default values
            // We care only about default values for booleans (only `@key` directive uses a default value anyway)
            if let (true, Some(Value::Boolean(arg_default))) = (expected_type == "Boolean", &arg_def.default_value) {
                let mut provided_value: Option<bool> = None;

                if let Some(provided_default) = &provided_arg_node.default_value {
                    let Value::Boolean(b) = provided_default else {
                        return Err(JsError::error("Expected a Boolean"));
                    };

                    provided_value = Some(b.value);
                }

                if Some(arg_default.value) != provided_value {
                    // NOTE(port): `argDef.defaultValue ? "true" : "false"` tests the node, which is
                    // always present here, so it always prints `true`.
                    let provided_printed = match provided_value {
                        Some(true) => "true",
                        Some(false) => "false",
                        None => "null",
                    };
                    errors.push(GraphQLError::with_code(
                        format!(
                            "Invalid definition for directive \"@{directive_name}\": argument \"{arg_name}\" should have default value true but found default value {provided_printed}"
                        ),
                        "DIRECTIVE_DEFINITION_INVALID",
                    ));
                }
            }
        }
    }

    // Compares locations
    let location_intersection: Vec<&str> = provided_locations
        .iter()
        .copied()
        .filter(|loc| original_locations.contains(loc))
        .collect();

    if location_intersection.is_empty() {
        errors.push(GraphQLError::with_code(
            format!(
                "Invalid definition for directive \"@{directive_name}\": \"@{directive_name}\" should have locations {}, but found (non-subset) {}",
                original_locations.join(", "),
                provided_locations.join(", ")
            ),
            "DIRECTIVE_DEFINITION_INVALID",
        ));
    }

    if !errors.is_empty() {
        for error in errors {
            context.report_error(error);
        }
    } else {
        // It has no errors, we can mark it as a replacement
        context.mark_as_federation_definition_replacement(&provided_directive_node.name.value);
    }

    Ok(())
}

/// `ObjectOrInterface`: an object or interface type definition or extension. The context's
/// `getSubgraphObjectOrInterfaceTypes()` map holds merged copies (`{ ...def }` with the fields,
/// interfaces and directives of all definitions and extensions concatenated, and `kind`
/// switched to the definition kind when one exists), so this is an owned struct rather than an
/// AST node.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectOrInterface {
    /// One of `ObjectTypeDefinition`, `ObjectTypeExtension`, `InterfaceTypeDefinition`,
    /// `InterfaceTypeExtension`.
    pub kind: Kind,
    pub description: Option<StringValue>,
    pub name: Name,
    pub interfaces: Vec<NamedType>,
    pub directives: Vec<DirectiveNode>,
    pub fields: Vec<FieldDefinition>,
}

/// The `info` of most `visitFields` interceptors: `{ typeDefinition, fieldName }`.
#[derive(Clone, Debug)]
pub struct InterceptFieldInfo {
    pub type_definition: Rc<ObjectOrInterface>,
    pub field_name: String,
}

/// The `info` of `interceptFieldWithMissingSelectionSet`.
#[derive(Clone, Debug)]
pub struct InterceptMissingSelectionSetInfo {
    pub type_definition: Rc<ObjectOrInterface>,
    pub field_name: String,
    /// `print(selectionFieldDef.type)` (library printer).
    pub output_type: String,
}

/// The `info` of `interceptDirective`.
#[derive(Clone, Debug)]
pub struct InterceptDirectiveInfo {
    pub directive_name: String,
    pub is_known: bool,
}

/// Which interceptors a `visitFields` call passes. The TS tests their presence
/// (`if (interceptUnknownField)`), and the recursive calls forward only some of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Intercepts {
    pub field: bool,
    pub field_with_missing_selection_set: bool,
    pub arguments: bool,
    pub unknown_field: bool,
    pub directive: bool,
    pub interface_type: bool,
    pub external_field: bool,
    pub non_external_field: bool,
}

/// The interceptor callbacks of `visitFields`. The TS callbacks are closures sharing the
/// rule's locals (`isValid`, `fieldsUsedInKey`, ...) and the context; a rule ports them as a
/// local struct implementing this trait, and gets the context as a parameter.
/// Only the methods named in the `Intercepts` passed to `visit_fields` are called.
pub trait VisitFieldsInterceptor<'a> {
    fn intercept_field(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_field_with_missing_selection_set(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptMissingSelectionSetInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_arguments(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_unknown_field(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_directive(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptDirectiveInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_interface_type(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_external_field(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_non_external_field(
        &mut self,
        _context: &mut SubgraphValidationContext<'a>,
        _info: &InterceptFieldInfo,
    ) -> JsResult<()> {
        Ok(())
    }
}

/// `visitFields({ context, selectionSet, typeDefinition, ...interceptors })`.
/// `type_definition` is an `Rc` taken from `context.get_subgraph_object_or_interface_types()`
/// so the context can be borrowed mutably by the interceptors. The nested
/// `handleInnerSelection` closure becomes a local helper inside the body.
pub fn visit_fields<'a>(
    context: &mut SubgraphValidationContext<'a>,
    selection_set: &SelectionSet,
    type_definition: &Rc<ObjectOrInterface>,
    interceptor: &mut dyn VisitFieldsInterceptor<'a>,
    intercepts: Intercepts,
) -> JsResult<()> {
    for selection in &selection_set.selections {
        let selection = match selection {
            Selection::FragmentSpread(_) => {
                continue; // TODO: we should error here I believe, but we need to check what Apollo Composition yields
            }
            Selection::InlineFragment(fragment) => {
                let Some(type_condition) = &fragment.type_condition else {
                    continue; // TODO: we should error here I believe, but we need to check what Apollo Composition yields
                };

                let interface_name = &type_condition.name.value;
                let Some(interface_definition) = context
                    .get_subgraph_object_or_interface_types()
                    .get(interface_name)
                    .cloned()
                else {
                    continue;
                };

                visit_fields(
                    context,
                    &fragment.selection_set,
                    &interface_definition,
                    interceptor,
                    Intercepts {
                        arguments: intercepts.arguments,
                        unknown_field: intercepts.unknown_field,
                        interface_type: intercepts.interface_type,
                        ..Intercepts::default()
                    },
                )?;
                continue;
            }
            Selection::Field(field) => field,
        };

        let typename_field_def;
        let selection_field_def: &FieldDefinition = if selection.name.value == "__typename" {
            typename_field_def = FieldDefinition {
                name: Name::new("__typename"),
                ty: Type::named("String"),
                ..Default::default()
            };
            &typename_field_def
        } else {
            match type_definition
                .fields
                .iter()
                .find(|field| field.name.value == selection.name.value)
            {
                Some(field) => field,
                None => {
                    if intercepts.unknown_field {
                        interceptor.intercept_unknown_field(
                            context,
                            &InterceptFieldInfo {
                                type_definition: type_definition.clone(),
                                field_name: selection.name.value.clone(),
                            },
                        )?;
                    }
                    break;
                }
            }
        };

        let is_typename = selection.name.value == "__typename";

        if !is_typename && intercepts.directive && !selection.directives.is_empty() {
            for directive in &selection.directives {
                let is_known = context
                    .get_subgraph_directive_definitions()
                    .contains_key(&directive.name.value);
                interceptor.intercept_directive(
                    context,
                    &InterceptDirectiveInfo {
                        directive_name: directive.name.value.clone(),
                        is_known,
                    },
                )?;
            }
        }

        if !is_typename {
            context.mark_as_used(
                MarkAsUsedReason::Fields,
                type_definition.kind,
                &type_definition.name.value,
                &selection_field_def.name.value,
            )?;
        }

        if !is_typename && intercepts.field {
            interceptor.intercept_field(
                context,
                &InterceptFieldInfo {
                    type_definition: type_definition.clone(),
                    field_name: selection.name.value.clone(),
                },
            )?;
        }

        if !is_typename && !selection_field_def.arguments.is_empty() && intercepts.arguments {
            interceptor.intercept_arguments(
                context,
                &InterceptFieldInfo {
                    type_definition: type_definition.clone(),
                    field_name: selection.name.value.clone(),
                },
            )?;
            continue;
        }

        if !is_typename && (intercepts.non_external_field || intercepts.external_field) {
            let is_external = selection_field_def
                .directives
                .iter()
                .any(|d| context.is_available_federation_directive("external", &d.name.value))
                || type_definition
                    .directives
                    .iter()
                    .any(|d| context.is_available_federation_directive("external", &d.name.value));
            let field_name = &selection.name.value;

            // ignore if it's not a leaf
            let Some(field_def) = type_definition
                .fields
                .iter()
                .find(|field| &field.name.value == field_name)
            else {
                continue;
            };

            let named_type = named_type_from_type_node(&field_def.ty);
            let is_leaf = context.is_leaf_type(&named_type.name.value);

            if is_leaf {
                if is_external && intercepts.external_field {
                    interceptor.intercept_external_field(
                        context,
                        &InterceptFieldInfo {
                            type_definition: type_definition.clone(),
                            field_name: field_name.clone(),
                        },
                    )?;
                } else if !is_external && intercepts.non_external_field {
                    interceptor.intercept_non_external_field(
                        context,
                        &InterceptFieldInfo {
                            type_definition: type_definition.clone(),
                            field_name: field_name.clone(),
                        },
                    )?;
                }
            }
        }

        // `handleInnerSelection(innerTypeDef)`.
        let handle_inner_selection = |context: &mut SubgraphValidationContext<'a>,
                                      interceptor: &mut dyn VisitFieldsInterceptor<'a>,
                                      inner_type_def: &Rc<ObjectOrInterface>|
         -> JsResult<()> {
            if !is_typename
                && intercepts.interface_type
                && (inner_type_def.kind == Kind::InterfaceTypeDefinition
                    || inner_type_def.kind == Kind::InterfaceTypeExtension)
            {
                interceptor.intercept_interface_type(
                    context,
                    &InterceptFieldInfo {
                        type_definition: type_definition.clone(),
                        field_name: selection.name.value.clone(),
                    },
                )?;
            }

            let Some(inner_selection) = &selection.selection_set else {
                if intercepts.field_with_missing_selection_set {
                    interceptor.intercept_field_with_missing_selection_set(
                        context,
                        &InterceptMissingSelectionSetInfo {
                            type_definition: type_definition.clone(),
                            field_name: selection.name.value.clone(),
                            output_type: print(&selection_field_def.ty),
                        },
                    )?;
                }
                return Ok(());
            };

            visit_fields(
                context,
                inner_selection,
                inner_type_def,
                interceptor,
                Intercepts {
                    field: intercepts.field,
                    arguments: intercepts.arguments,
                    unknown_field: intercepts.unknown_field,
                    interface_type: intercepts.interface_type,
                    directive: intercepts.directive,
                    external_field: intercepts.external_field,
                    field_with_missing_selection_set: intercepts.field_with_missing_selection_set,
                    // NOTE(port): the TS does not forward `interceptNonExternalField` here.
                    non_external_field: false,
                },
            )
        };

        let output_type = &named_type_from_type_node(&selection_field_def.ty).name.value;
        // This does not return a type definition for union fields.
        let inner_type_def = context
            .get_subgraph_object_or_interface_types()
            .get(output_type)
            .cloned();

        let Some(inner_type_def) = inner_type_def else {
            // In case it is a union type we need to process the selection for each union member
            let Some(union_member_type_names) = context.get_subgraph_union_types().get(output_type).cloned() else {
                continue;
            };

            for type_name in &union_member_type_names {
                let Some(inner_type_def) = context.get_subgraph_object_or_interface_types().get(type_name).cloned()
                else {
                    continue;
                };

                handle_inner_selection(context, interceptor, &inner_type_def)?;
            }

            continue;
        };

        handle_inner_selection(context, interceptor, &inner_type_def)?;
    }

    Ok(())
}

/// `getFieldsArgument(directiveNode)`: the `fields` argument.
pub fn get_fields_argument(directive_node: &DirectiveNode) -> Option<&Argument> {
    directive_node.arguments.iter().find(|arg| arg.name.value == "fields")
}

/// `parseFields(fields)`: parses `{ fields }` (or `fields` wrapped in braces) as a query and
/// returns its selection set. Throws the parser's `GraphQLError`.
pub fn parse_fields(fields: &str) -> JsResult<Option<SelectionSet>> {
    let source = if js_trim(fields).starts_with('{') {
        format!("query {fields}")
    } else {
        format!("query {{ {fields} }}")
    };
    let parsed = graphql_rs::parse(&source)?;

    Ok(parsed.definitions.into_iter().find_map(|d| match d {
        Definition::OperationDefinition(op) => Some(op.selection_set),
        _ => None,
    }))
}

/// JS `String.prototype.trim`: strips JS WhiteSpace and LineTerminator code points.
/// NOTE(port): differs from Rust's `str::trim` for U+0085 (not JS whitespace) and U+FEFF (JS
/// whitespace).
fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{FEFF}'
            | '\u{000A}'
            | '\u{000D}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

/// `namedTypeFromTypeNode(type)`.
pub fn named_type_from_type_node(ty: &Type) -> &NamedType {
    match ty {
        Type::Named(n) => n,
        Type::List(l) => named_type_from_type_node(&l.ty),
        Type::NonNull(n) => named_type_from_type_node(&n.ty),
    }
}

/// `isDirectiveDefinitionNode(node)`.
// TS: used by `compose-directive.ts`; the ported callers do not need it.
#[allow(dead_code)]
pub fn is_directive_definition_node(node: graphql_rs::NodeRef<'_>) -> bool {
    node.kind() == Kind::DirectiveDefinition
}

/// `printOutputType(type)`: `[A!]!` style type string.
/// PERF: writes into one string instead of formatting a new one per wrapper type.
pub fn print_output_type(ty: &Type) -> String {
    fn write(ty: &Type, out: &mut String) {
        match ty {
            Type::Named(n) => out.push_str(&n.name.value),
            Type::List(l) => {
                out.push('[');
                write(&l.ty, out);
                out.push(']');
            }
            Type::NonNull(n) => {
                write(&n.ty, out);
                out.push('!');
            }
        }
    }
    let mut out = String::new();
    write(ty, &mut out);
    out
}

fn is_non_null_type_node(node: &Type) -> bool {
    matches!(node, Type::NonNull(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_trim_matches_js_semantics() {
        assert_eq!(js_trim("  { a }\n"), "{ a }");
        // U+FEFF is JS whitespace, U+0085 is not.
        assert_eq!(js_trim("\u{FEFF}{ a }"), "{ a }");
        assert_eq!(js_trim("\u{0085}{ a }"), "\u{0085}{ a }");
        assert_eq!(js_trim("\u{2028}\u{3000}x\u{00A0}"), "x");
    }

    fn selection_names(ss: &SelectionSet) -> Vec<String> {
        ss.selections
            .iter()
            .map(|s| match s {
                Selection::Field(f) => f.name.value.clone(),
                _ => String::new(),
            })
            .collect()
    }

    #[test]
    fn parse_fields_wraps_or_not() {
        let ss = parse_fields("id name").unwrap().unwrap();
        assert_eq!(selection_names(&ss), vec!["id", "name"]);
        let ss = parse_fields("  { id }").unwrap().unwrap();
        assert_eq!(selection_names(&ss), vec!["id"]);
        // `{ a } b` starts with a brace, so it is parsed as `query { a } b`, which fails.
        assert!(parse_fields("{ a } b").is_err());
        assert!(matches!(parse_fields("a {").unwrap_err(), JsError::GraphQL(_)));
    }

    #[test]
    fn print_output_type_prints_modifiers() {
        let ty = graphql_rs::parse_type("[A!]!").unwrap();
        assert_eq!(print_output_type(&ty), "[A!]!");
        assert_eq!(named_type_from_type_node(&ty).name.value, "A");
    }
}
