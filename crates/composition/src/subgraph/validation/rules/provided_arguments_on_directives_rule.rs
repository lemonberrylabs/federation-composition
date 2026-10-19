//! Port of `src/subgraph/validation/rules/provided-arguments-on-directives-rule.ts`.

use graphql_js::{GraphQLError, Kind, NodeRef, VisitAction, VisitInfo, Visitor};

use crate::graphql::printer::print;
use crate::subgraph::helpers::named_type_from_type_node;
use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::{Type, Value};

/// `ProvidedArgumentsOnDirectivesRule(context)`.
/// Visitor callbacks: `Directive: { leave(directiveNode, _k, _p, _pp, ancestors) }`.
/// Fields: the locals of the TS factory.
pub struct ProvidedArgumentsOnDirectivesRule {}

impl<'a> SimpleRule<'a> for ProvidedArgumentsOnDirectivesRule {
    const NAME: &'static str = "ProvidedArgumentsOnDirectivesRule";
    type Visitor = Self;

    fn new(_context: &mut SimpleValidationContext<'a>) -> Self {
        ProvidedArgumentsOnDirectivesRule {}
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for ProvidedArgumentsOnDirectivesRule {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_js::VisitKinds {
        use graphql_js::{Kind, VisitKinds};
        VisitKinds::enter_leave(&[], &[Kind::Directive])
    }

    // Validate on leave to allow for deeper errors to appear first.
    fn leave(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        let NodeRef::Directive(directive_node) = node else {
            return VisitAction::Continue;
        };

        let directive_name = &directive_node.name.value;
        let Some(directive_definition) = cx.get_known_directive_definition(directive_name) else {
            return VisitAction::Continue; // Let KnownDirectivesRule handle this
        };

        // `if (args && directiveDefinition.arguments)`: empty lists iterate nothing.
        let args = &directive_node.arguments;
        if !args.is_empty() && !directive_definition.arguments.is_empty() {
            let coordinate = cx.get_schema_coordinate(info.ancestors);

            for arg in args {
                let Some(arg_definition) = directive_definition
                    .arguments
                    .iter()
                    .find(|a| a.name.value == arg.name.value)
                else {
                    continue;
                };

                if matches!(arg_definition.ty, Type::NonNull(_)) && matches!(arg.value, Value::Null(_)) {
                    // Let RequiredArgumentsOnDirectivesRule handle this
                    continue;
                }

                let printed_type = print_type_node(&arg_definition.ty, cx);
                let printed_value = print_value_node(&arg.value);

                let Some(printed_type) = printed_type else {
                    continue; // Let KnownTypesRule handle this
                };

                if printed_type != "Any" && printed_type != printed_value {
                    // received empty list
                    if printed_value == "[]" {
                        // An empty list can be a valid value for a list type
                        // or a non-null list type
                        let is_list_type = match &arg_definition.ty {
                            Type::List(_) => true,
                            Type::NonNull(n) => matches!(*n.ty, Type::List(_)),
                            Type::Named(_) => false,
                        };
                        if is_list_type {
                            continue;
                        }
                    }

                    let named_type = named_type_from_type_node(&arg_definition.ty);
                    let type_name = &named_type.name.value;

                    // Float accepts Int, but not the other way around
                    if printed_value == "Int" && type_name == "Float" {
                        continue;
                    }

                    cx.report_error(GraphQLError::with_code(
                        format!(
                            "Invalid value for \"@{directive_name}({}:)\" of type \"{}\" in application of \"@{directive_name}\" to \"{coordinate}\".",
                            arg.name.value,
                            print(&arg_definition.ty)
                        ),
                        "INVALID_GRAPHQL",
                    ));
                }
            }
        }

        VisitAction::Continue
    }
}

/// `printValueNode(valueNode)`. The TS throws for a variable, which const SDL values never are.
fn print_value_node(value_node: &Value) -> String {
    match value_node {
        Value::List(list) => match list.values.first() {
            None => "[]".to_string(),
            Some(first) => print_value_node(first),
        },
        Value::Enum(_) => "Enum".to_string(),
        Value::Null(_) => "Null".to_string(),
        Value::Int(_) => "Int".to_string(),
        Value::Float(_) => "Float".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Boolean(_) => "Boolean".to_string(),
        Value::Object(_) => "Object".to_string(),
        // NOTE(port): `throw new Error("Unknown value node kind: [object Object]")`; the SDL
        // parser rejects variables in const values, so this cannot be reached.
        Value::Variable(_) => unreachable!("Unknown value node kind: [object Object]"),
    }
}

/// `printTypeNode(typeNode, context)`.
fn print_type_node(type_node: &Type, context: &SimpleValidationContext<'_>) -> Option<String> {
    let inner = match type_node {
        Type::Named(named) => {
            let Some(def) = context.get_known_type_definition(&named.name.value) else {
                return graphql_js::specified_scalar_types()
                    .iter()
                    .find(|s| s.name == named.name.value)
                    .map(|s| s.name.to_string());
            };

            return match def.kind {
                // Accept any value for scalars, we don't know what they are
                Kind::ScalarTypeDefinition | Kind::ScalarTypeExtension => Some("Any".to_string()),
                Kind::EnumTypeDefinition | Kind::EnumTypeExtension => Some("Enum".to_string()),
                Kind::InputObjectTypeDefinition
                | Kind::InputObjectTypeExtension
                | Kind::ObjectTypeDefinition
                | Kind::ObjectTypeExtension
                | Kind::InterfaceTypeDefinition
                | Kind::InterfaceTypeExtension
                | Kind::UnionTypeDefinition
                | Kind::UnionTypeExtension => Some("Object".to_string()),
                // NOTE(port): another named kind (an operation or fragment) falls out of the TS
                // `switch` into `printTypeNode(typeNode.type)`, which reads `undefined.kind`
                // and throws a `TypeError`. Subgraph SDL has no such definitions.
                _ => unreachable!("Cannot read properties of undefined (reading 'kind')"),
            };
        }
        Type::List(list) => &list.ty,
        Type::NonNull(non_null) => &non_null.ty,
    };

    print_type_node(inner, context)
}
