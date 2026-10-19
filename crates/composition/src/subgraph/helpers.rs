//! Port of `src/subgraph/helpers.ts`.

use std::rc::Rc;

use graphql_js::{
    Argument, Directive as DirectiveNode, DirectiveDefinition, FieldDefinition, Kind, Name, NamedType, SelectionSet,
    StringValue, Type,
};

use crate::subgraph::validation::validation_context::SubgraphValidationContext;
use crate::utils::js::JsResult;

/// `validateDirectiveAgainstOriginal(providedDirectiveNode, directiveName, context)`: checks a
/// subgraph's own definition of a federation directive against the spec; reports errors or
/// marks it as a replacement.
pub fn validate_directive_against_original(
    provided_directive_node: &DirectiveDefinition,
    directive_name: &str,
    context: &mut SubgraphValidationContext<'_>,
) -> JsResult<()> {
    todo!()
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
    fn intercept_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        Ok(())
    }
    fn intercept_field_with_missing_selection_set(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptMissingSelectionSetInfo,
    ) -> JsResult<()> {
        Ok(())
    }
    fn intercept_arguments(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        Ok(())
    }
    fn intercept_unknown_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        Ok(())
    }
    fn intercept_directive(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptDirectiveInfo) -> JsResult<()> {
        Ok(())
    }
    fn intercept_interface_type(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        Ok(())
    }
    fn intercept_external_field(&mut self, context: &mut SubgraphValidationContext<'a>, info: &InterceptFieldInfo) -> JsResult<()> {
        Ok(())
    }
    fn intercept_non_external_field(
        &mut self,
        context: &mut SubgraphValidationContext<'a>,
        info: &InterceptFieldInfo,
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
    todo!()
}

/// `getFieldsArgument(directiveNode)`: the `fields` argument.
pub fn get_fields_argument(directive_node: &DirectiveNode) -> Option<&Argument> {
    directive_node.arguments.iter().find(|arg| arg.name.value == "fields")
}

/// `parseFields(fields)`: parses `{ fields }` (or `fields` wrapped in braces) as a query and
/// returns its selection set. Throws the parser's `GraphQLError`.
pub fn parse_fields(fields: &str) -> JsResult<Option<SelectionSet>> {
    todo!()
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
pub fn is_directive_definition_node(node: graphql_js::NodeRef<'_>) -> bool {
    node.kind() == Kind::DirectiveDefinition
}

/// `printOutputType(type)`: `[A!]!` style type string.
pub fn print_output_type(ty: &Type) -> String {
    match ty {
        Type::Named(n) => n.name.value.clone(),
        Type::List(l) => format!("[{}]", print_output_type(&l.ty)),
        Type::NonNull(n) => format!("{}!", print_output_type(&n.ty)),
    }
}

fn is_non_null_type_node(node: &Type) -> bool {
    matches!(node, Type::NonNull(_))
}
