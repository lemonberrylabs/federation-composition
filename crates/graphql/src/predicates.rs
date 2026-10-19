//! Port of graphql-js `language/predicates`.

use crate::ast::{Kind, NodeRef, OperationDefinition, OperationType};

/// graphql-js `isDefinitionNode`.
pub fn is_definition_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    let node = node.into();
    is_executable_definition_node(node)
        || is_type_system_definition_node(node)
        || is_type_system_extension_node(node)
}

/// graphql-js `isExecutableDefinitionNode`.
pub fn is_executable_definition_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    matches!(
        node.into().kind(),
        Kind::OperationDefinition | Kind::FragmentDefinition
    )
}

/// graphql-js `isSubscriptionOperationDefinitionNode`.
pub fn is_subscription_operation_definition_node(node: &OperationDefinition) -> bool {
    node.operation == OperationType::Subscription
}

/// graphql-js `isSelectionNode`.
pub fn is_selection_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    matches!(
        node.into().kind(),
        Kind::Field | Kind::FragmentSpread | Kind::InlineFragment
    )
}

/// graphql-js `isValueNode`.
pub fn is_value_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    matches!(
        node.into().kind(),
        Kind::Variable
            | Kind::IntValue
            | Kind::FloatValue
            | Kind::StringValue
            | Kind::BooleanValue
            | Kind::NullValue
            | Kind::EnumValue
            | Kind::ListValue
            | Kind::ObjectValue
    )
}

/// graphql-js `isConstValueNode`. Like graphql-js, a list or object is
/// const when *some* item is const (so an empty list is not).
pub fn is_const_value_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    let node = node.into();
    is_value_node(node)
        && match node {
            NodeRef::ListValue(l) => l.values.iter().any(is_const_value_node),
            NodeRef::ObjectValue(o) => o.fields.iter().any(|f| is_const_value_node(&f.value)),
            _ => node.kind() != Kind::Variable,
        }
}

/// graphql-js `isTypeNode`.
pub fn is_type_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    matches!(
        node.into().kind(),
        Kind::NamedType | Kind::ListType | Kind::NonNullType
    )
}

/// graphql-js `isTypeSystemDefinitionNode`.
pub fn is_type_system_definition_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    let node = node.into();
    node.kind() == Kind::SchemaDefinition
        || is_type_definition_node(node)
        || node.kind() == Kind::DirectiveDefinition
}

/// graphql-js `isTypeDefinitionNode`.
pub fn is_type_definition_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    matches!(
        node.into().kind(),
        Kind::ScalarTypeDefinition
            | Kind::ObjectTypeDefinition
            | Kind::InterfaceTypeDefinition
            | Kind::UnionTypeDefinition
            | Kind::EnumTypeDefinition
            | Kind::InputObjectTypeDefinition
    )
}

/// graphql-js `isTypeSystemExtensionNode`.
pub fn is_type_system_extension_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    let node = node.into();
    node.kind() == Kind::SchemaExtension
        || node.kind() == Kind::DirectiveExtension
        || is_type_extension_node(node)
}

/// graphql-js `isTypeExtensionNode`.
pub fn is_type_extension_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    matches!(
        node.into().kind(),
        Kind::ScalarTypeExtension
            | Kind::ObjectTypeExtension
            | Kind::InterfaceTypeExtension
            | Kind::UnionTypeExtension
            | Kind::EnumTypeExtension
            | Kind::InputObjectTypeExtension
    )
}

/// graphql-js `isSchemaCoordinateNode`.
pub fn is_schema_coordinate_node<'a>(node: impl Into<NodeRef<'a>>) -> bool {
    matches!(
        node.into().kind(),
        Kind::TypeCoordinate
            | Kind::MemberCoordinate
            | Kind::ArgumentCoordinate
            | Kind::DirectiveCoordinate
            | Kind::DirectiveArgumentCoordinate
    )
}
