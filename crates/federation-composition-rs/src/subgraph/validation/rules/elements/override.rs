//! Port of `src/subgraph/validation/rules/elements/override.ts`.

use graphql_rs::{Directive, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use crate::graphql::printer::print;
use crate::subgraph::helpers::validate_directive_against_original;
use crate::subgraph::validation::validation_context::{SubgraphRule, SubgraphValidationContext};
use crate::utils::js::{parse_float, JsResult};
use graphql_rs::Value;

use super::{finish, is_interface_kind, is_object_kind, node_name};

// Supergraph context:
// - save what field is being overridden and from which subgraph

/// `OverrideRules(context)`.
/// Visitor callbacks: `DirectiveDefinition` (`validateDirectiveAgainstOriginal`), `Directive`.
/// Fields: the locals of the TS factory.
pub struct OverrideRules {}

impl<'a> SubgraphRule<'a> for OverrideRules {
    const NAME: &'static str = "OverrideRules";

    fn new(_context: &mut SubgraphValidationContext<'a>) -> Self {
        OverrideRules {}
    }
}

impl<'a> Visitor<'a, SubgraphValidationContext<'a>> for OverrideRules {
    /// PERF: the kinds the callbacks below act on, so `visit_in_parallel` skips the others.
    fn kinds(&self) -> graphql_rs::VisitKinds {
        use graphql_rs::{Kind, VisitKinds};
        VisitKinds::enter(&[Kind::DirectiveDefinition, Kind::Directive])
    }

    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SubgraphValidationContext<'a>, _info: &VisitInfo<'a, '_>) -> VisitAction {
        let result = match node {
            NodeRef::DirectiveDefinition(node) => validate_directive_against_original(node, "override", cx),
            NodeRef::Directive(node) => directive(node, cx),
            _ => Ok(()),
        };
        finish(cx, result)
    }
}

/// `fieldDef.directives` of what `TypeNodeInfo.getFieldDef()` returns.
fn field_directives(field_def: NodeRef<'_>) -> &[Directive] {
    match field_def {
        NodeRef::FieldDefinition(field) => &field.directives,
        NodeRef::InputValueDefinition(input) => &input.directives,
        _ => &[],
    }
}

/// The `Directive(node)` callback.
fn directive<'a>(node: &'a Directive, context: &mut SubgraphValidationContext<'a>) -> JsResult<()> {
    if !context.is_available_federation_directive("override", &node.name.value) {
        return Ok(());
    }

    let field_def = context.type_node_info.get_field_def();
    let type_def = context.type_node_info.get_type_def();

    let (Some(field_def), Some(type_def)) = (field_def, type_def) else {
        return Ok(());
    };

    let type_name = node_name(type_def);
    let field_name = node_name(field_def);

    if is_interface_kind(type_def.kind()) && context.satisfies_version_range(">= v2.3") {
        context.report_error(GraphQLError::with_code(
            format!(
                "@override cannot be used on field \"{type_name}.{field_name}\" on subgraph \"{}\": @override is not supported on interface type fields.",
                context.get_subgraph_name()
            ),
            "OVERRIDE_ON_INTERFACE",
        ));
        // We stop here as we don't want to report other errors on this field (Apollo Composition does not)
        return Ok(());
    }

    let label_arg = node.arguments.iter().find(|arg| arg.name.value == "label");
    if let Some(label_arg) = label_arg {
        if context.satisfies_version_range(">= v2.7") && !is_valid_label(&label_arg.value) {
            context.report_error(GraphQLError::with_code(
                invalid_label_value_error(
                    &print(&label_arg.value),
                    &format!("{type_name}.{field_name}"),
                    context.get_subgraph_name(),
                ),
                "OVERRIDE_LABEL_INVALID",
            ));
            return Ok(());
        }
    }

    let label_value = label_arg.map(|arg| &arg.value);

    let label_value = match label_value {
        // other validation rule takes care of this case.
        // At this point, we can ignore this directive as the subgraph is invalid anyway.
        Some(value) if !matches!(value, Value::String(_)) => return Ok(()),
        Some(Value::String(value)) => Some(value.value.as_str()),
        _ => None,
    };

    let from_arg = node.arguments.iter().find(|arg| arg.name.value == "from");

    let Some(Value::String(from_value)) = from_arg.map(|arg| &arg.value) else {
        return Ok(());
    };

    // NOTE(port): the TS checks `!typeDef` again here and throws "Parent type not found but
    // `@override` directive is present on a field."; `typeDef` is known to be set.

    let conflicting_directives: Vec<&Directive> = field_directives(field_def)
        .iter()
        .filter(|directive| context.is_available_federation_directive("external", &directive.name.value))
        .collect();

    for directive in conflicting_directives {
        let subgraph_name = context.get_subgraph_name();
        context.report_error(GraphQLError::with_code(
            format!(
                "@override cannot be used on field \"{type_name}.{field_name}\" on subgraph \"{subgraph_name}\" since \"{type_name}.{field_name}\" on \"{subgraph_name}\" is marked with directive \"@{}\"",
                directive.name.value
            ),
            "OVERRIDE_COLLISION_WITH_ANOTHER_DIRECTIVE",
        ));
    }

    if from_value.value == context.get_subgraph_name() {
        context.report_error(GraphQLError::with_code(
            format!(
                "Source and destination subgraphs \"{}\" are the same for overridden field \"{type_name}.{field_name}\"",
                from_value.value
            ),
            "OVERRIDE_FROM_SELF_ERROR",
        ));
    }

    if is_object_kind(type_def.kind()) {
        context
            .state_builder
            .object_type()
            .field()
            .set_override(type_name, field_name, &from_value.value, label_value)?;
    } else {
        context
            .state_builder
            .interface_type()
            .field()
            .set_override(type_name, field_name, &from_value.value, label_value)?;
    }

    Ok(())
}

/// `labelNonPercentRegex.test(value)` for `/^[a-zA-Z][a-zA-Z0-9_\-:./]*$/`, hand-written (the
/// crate has no regex dependency).
fn matches_label_non_percent(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':' | '.' | '/'))
}

/// `labelPercentRegex.test(value)` for
/// `/^percent\(((\d{1,2}(\.\d{1,8})?)|100(\.[0]{1,8})?)\)$/`, hand-written. JS `\d` is
/// ASCII-only, and `$` without the `m` flag only matches at the end of the input.
fn matches_label_percent(value: &str) -> bool {
    let Some(inner) = value.strip_prefix("percent(").and_then(|rest| rest.strip_suffix(')')) else {
        return false;
    };

    /// `digits{min,max}` followed by an optional `\.` + `fraction{1,8}`.
    fn matches_number(inner: &str, int_part: impl Fn(&str) -> bool, fraction_digit: impl Fn(u8) -> bool) -> bool {
        let (int, fraction) = match inner.split_once('.') {
            Some((int, fraction)) => (int, Some(fraction)),
            None => (inner, None),
        };
        if !int_part(int) {
            return false;
        }
        match fraction {
            None => true,
            Some(fraction) => (1..=8).contains(&fraction.len()) && fraction.bytes().all(&fraction_digit),
        }
    }

    // `\d{1,2}(\.\d{1,8})?`
    let small = matches_number(
        inner,
        |int| (1..=2).contains(&int.len()) && int.bytes().all(|b| b.is_ascii_digit()),
        |b| b.is_ascii_digit(),
    );
    // `100(\.[0]{1,8})?`
    let hundred = matches_number(inner, |int| int == "100", |b| b == b'0');

    small || hundred
}

fn invalid_label_value_error(value: &str, coordinate: &str, subgraph_name: &str) -> String {
    format!(
        "Invalid @override label {value} on field \"{coordinate}\" on subgraph \"{subgraph_name}\": labels must start with a letter and after that may contain alphanumerics, underscores, minuses, colons, periods, or slashes. Alternatively, labels may be of the form \"percent(x)\" where x is a float between 0-100 inclusive."
    )
}

/// `isValidLabel(valueNode)`. NOTE(port): JS `\d` is ASCII-only, like `regex`'s `[0-9]`;
/// use `[0-9]` or `(?-u:\d)` when porting the patterns.
fn is_valid_label(value_node: &Value) -> bool {
    let Value::String(value_node) = value_node else {
        return false;
    };

    let value = value_node.value.as_str();

    if value.starts_with("percent(+") || value.starts_with("percent(-") {
        return false;
    }

    if value.starts_with("percent(") && value.ends_with(')') {
        if !matches_label_percent(value) {
            return false;
        }

        // `value.replace("percent(", "").replace(")", "")`: each replaces the first occurrence.
        let percentage = parse_float(&value.replacen("percent(", "", 1).replacen(')', "", 1));
        return (0.0..=100.0).contains(&percentage);
    }

    matches_label_non_percent(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(value: &str) -> bool {
        is_valid_label(&Value::string(value))
    }

    #[test]
    fn non_percent_labels() {
        assert!(label("a"));
        assert!(label("canary-1_x:y.z/w"));
        assert!(!label(""));
        assert!(!label("1abc"));
        assert!(!label("_abc"));
        assert!(!label("ab c"));
        assert!(!label("abc\n"));
        assert!(!label("ab$"));
        assert!(!label("é"));
    }

    #[test]
    fn percent_labels() {
        assert!(label("percent(0)"));
        assert!(label("percent(5)"));
        assert!(label("percent(50)"));
        assert!(label("percent(99.99999999)"));
        assert!(label("percent(100)"));
        assert!(label("percent(100.0)"));
        assert!(label("percent(100.00000000)"));
        assert!(!label("percent(100.000000000)"));
        assert!(!label("percent(100.1)"));
        assert!(!label("percent(101)"));
        assert!(!label("percent(123)"));
        assert!(!label("percent(1.)"));
        assert!(!label("percent(.5)"));
        assert!(!label("percent(1.123456789)"));
        assert!(!label("percent()"));
        assert!(!label("percent(+5)"));
        assert!(!label("percent(-5)"));
        assert!(!label("percent(5))"));
        assert!(!label("percent(1e1)"));
        // Not a percent form, and `(` is not allowed in a plain label.
        assert!(!label("percent(5"));
    }

    #[test]
    fn non_string_values_are_invalid() {
        assert!(!is_valid_label(&Value::Boolean(graphql_rs::BooleanValue { value: true })));
    }
}
